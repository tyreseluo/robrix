use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use futures_util::StreamExt;
use makepad_widgets::Cx;
use robrix_crew_bridge::{
    BridgeEvent, BridgeManager, CrewGlueState, CrewTransport, ExecutionProfile, InventorySnapshot,
    ProviderProfile, ResolveError, RoomBinding, RoomInventory, SpaceInventory, StateStore,
    UserSnapshot, Workspace, resolve_room_binding,
};

use crate::{
    home::rooms_list::RoomsListRef,
    persistence::matrix_state::persistent_state_dir,
    sliding_sync::{current_user_id, get_client, spawn_on_tokio},
};

const DEFAULT_PROVIDER_ID: &str = "default-crew-provider";
const DEFAULT_WORKSPACE_ID: &str = "default-crew-workspace";
const DEFAULT_PROFILE_ID: &str = "default-crew-profile";

static BRIDGE_CONTEXT: Mutex<Option<BridgeContext>> = Mutex::new(None);

struct BridgeContext {
    user_id: String,
    store: StateStore,
    state: CrewGlueState,
}

#[derive(Clone, Debug, Default)]
pub struct DefaultConfigForm {
    pub endpoint: String,
    pub auth_token_env: String,
    pub workspace_root: String,
}

#[derive(Clone, Debug)]
pub enum CrewSettingsAction {
    StateChanged,
    Status(String),
    HealthcheckFinished {
        room_id: String,
        result: Result<String, String>,
    },
    StreamStarted {
        room_id: String,
        profile_name: String,
    },
    StreamDelta {
        room_id: String,
        text: String,
    },
    StreamFinished {
        room_id: String,
        text: String,
    },
    StreamFailed {
        room_id: String,
        error: String,
    },
}

pub fn clear_loaded_state() {
    *BRIDGE_CONTEXT.lock().unwrap() = None;
}

pub fn ensure_loaded_for_current_user() -> Result<(), String> {
    let Some(user_id) = current_user_id() else {
        clear_loaded_state();
        return Err("No Matrix user is currently logged in.".into());
    };

    let user_id_str = user_id.to_string();
    let mut guard = BRIDGE_CONTEXT.lock().unwrap();
    let needs_reload = guard.as_ref().is_none_or(|ctx| ctx.user_id != user_id_str);
    if !needs_reload {
        return Ok(());
    }

    let store = StateStore::in_dir(bridge_state_dir(&user_id));
    let mut state = store.load_or_default().map_err(|error| error.to_string())?;
    state.user = current_user_snapshot(&user_id);
    *guard = Some(BridgeContext {
        user_id: user_id_str,
        store,
        state,
    });
    Ok(())
}

pub fn snapshot() -> Option<CrewGlueState> {
    ensure_loaded_for_current_user().ok()?;
    BRIDGE_CONTEXT
        .lock()
        .unwrap()
        .as_ref()
        .map(|ctx| ctx.state.clone())
}

pub fn default_config_form() -> DefaultConfigForm {
    snapshot().map_or_else(DefaultConfigForm::default, |state| {
        let provider = state.providers.get(DEFAULT_PROVIDER_ID);
        let workspace = state.workspaces.get(DEFAULT_WORKSPACE_ID);
        DefaultConfigForm {
            endpoint: provider
                .and_then(|provider| provider.base_url.clone())
                .unwrap_or_default(),
            auth_token_env: provider
                .and_then(|provider| provider.api_key_env.clone())
                .unwrap_or_default(),
            workspace_root: workspace
                .map(|workspace| workspace.root_dir.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    })
}

pub fn refresh_inventory_from_rooms_list(cx: &mut Cx) -> Result<bool, String> {
    ensure_loaded_for_current_user()?;
    if !cx.has_global::<RoomsListRef>() {
        return Err("Rooms list is not available yet.".into());
    }

    let rooms_list = cx.get_global::<RoomsListRef>();
    let room_snapshots = rooms_list.crew_room_snapshots();
    let space_snapshots = rooms_list.crew_space_snapshots();
    let user = current_user_snapshot(
        &current_user_id().ok_or_else(|| "No Matrix user is currently logged in.".to_string())?,
    );
    let inventory = InventorySnapshot {
        rooms: room_snapshots
            .into_iter()
            .map(|room| {
                let room_id = room.room_id.to_string();
                let inventory = RoomInventory {
                    room_id: room_id.clone(),
                    display_name: (!room.display_name.is_empty()).then_some(room.display_name),
                    canonical_alias: room.canonical_alias.map(|alias| alias.to_string()),
                    space_ids: room
                        .space_ids
                        .into_iter()
                        .map(|space_id| space_id.to_string())
                        .collect(),
                    is_direct: room.is_direct,
                    stale: false,
                };
                (room_id, inventory)
            })
            .collect(),
        spaces: space_snapshots
            .into_iter()
            .map(|space| {
                let space_id = space.space_id.to_string();
                let inventory = SpaceInventory {
                    space_id: space_id.clone(),
                    display_name: None,
                    canonical_alias: None,
                    child_room_ids: space
                        .child_room_ids
                        .into_iter()
                        .map(|room_id| room_id.to_string())
                        .collect(),
                };
                (space_id, inventory)
            })
            .collect(),
    };

    with_context_mut(|ctx| {
        if ctx.state.user == user && ctx.state.inventory == inventory {
            return Ok(false);
        }
        ctx.state.refresh_inventory(user, inventory);
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok(true)
    })
}

pub fn save_default_profile(
    endpoint: &str,
    auth_token_env: &str,
    workspace_root: &str,
) -> Result<(), String> {
    ensure_loaded_for_current_user()?;

    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return Err("Crew endpoint is required.".into());
    }

    with_context_mut(|ctx| {
        ctx.state.providers.insert(
            DEFAULT_PROVIDER_ID.to_string(),
            ProviderProfile {
                id: DEFAULT_PROVIDER_ID.to_string(),
                provider: "crew-sse".into(),
                model: None,
                base_url: Some(endpoint.to_string()),
                api_type: None,
                api_key_env: non_empty(auth_token_env),
                description: Some("Default Crew SSE endpoint used by Robrix.".into()),
            },
        );

        let workspace_id = match non_empty(workspace_root) {
            Some(workspace_root) => {
                let root_dir = PathBuf::from(&workspace_root);
                let workspace_name = Path::new(&workspace_root)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.is_empty())
                    .unwrap_or("Workspace")
                    .to_string();
                ctx.state.workspaces.insert(
                    DEFAULT_WORKSPACE_ID.to_string(),
                    Workspace {
                        id: DEFAULT_WORKSPACE_ID.to_string(),
                        name: workspace_name,
                        root_dir,
                        data_dir: None,
                        skills_dirs: Vec::new(),
                        description: Some(
                            "Default workspace configured from Robrix settings.".into(),
                        ),
                    },
                );
                Some(DEFAULT_WORKSPACE_ID.to_string())
            }
            None => {
                ctx.state.workspaces.remove(DEFAULT_WORKSPACE_ID);
                None
            }
        };

        ctx.state.execution_profiles.insert(
            DEFAULT_PROFILE_ID.to_string(),
            ExecutionProfile {
                id: DEFAULT_PROFILE_ID.to_string(),
                name: "Default Crew Profile".into(),
                provider_id: DEFAULT_PROVIDER_ID.to_string(),
                workspace_id,
                system_prompt: None,
                description: Some("Default Robrix -> Crew execution profile.".into()),
            },
        );
        ctx.state.defaults.execution_profile_id = Some(DEFAULT_PROFILE_ID.to_string());
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())
    })?;

    Cx::post_action(CrewSettingsAction::StateChanged);
    Ok(())
}

pub fn bind_room_to_default(room_id: &str) -> Result<(), String> {
    ensure_loaded_for_current_user()?;
    with_context_mut(|ctx| {
        if !ctx
            .state
            .execution_profiles
            .contains_key(DEFAULT_PROFILE_ID)
        {
            return Err("Save a default Crew profile before binding a room.".into());
        }
        if !ctx.state.inventory.rooms.contains_key(room_id) {
            return Err(format!(
                "Room {room_id} is not in the current inventory snapshot."
            ));
        }
        ctx.state.room_bindings.insert(
            room_id.to_string(),
            RoomBinding {
                room_id: room_id.to_string(),
                execution_profile_id: DEFAULT_PROFILE_ID.to_string(),
                enabled: true,
            },
        );
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())
    })?;

    Cx::post_action(CrewSettingsAction::StateChanged);
    Ok(())
}

pub fn unbind_room(room_id: &str) -> Result<(), String> {
    ensure_loaded_for_current_user()?;
    with_context_mut(|ctx| {
        ctx.state.room_bindings.remove(room_id);
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())
    })?;

    Cx::post_action(CrewSettingsAction::StateChanged);
    Ok(())
}

pub fn describe_room_binding(room_id: &str) -> String {
    let Some(state) = snapshot() else {
        return "Crew bridge state is not loaded.".into();
    };

    match resolve_room_binding(&state, room_id) {
        Ok(resolved) => {
            let source = match &resolved.source {
                robrix_crew_bridge::resolver::BindingSource::Room(_) => "room binding",
                robrix_crew_bridge::resolver::BindingSource::Space(_) => "space binding",
                robrix_crew_bridge::resolver::BindingSource::Default => "global default",
            };
            let workspace = resolved
                .workspace
                .map(|workspace| workspace.root_dir.to_string_lossy().into_owned())
                .unwrap_or_else(|| "(none)".into());
            let endpoint = resolved
                .provider
                .base_url
                .unwrap_or_else(|| "(missing endpoint)".into());
            format!(
                "{source}: {} -> {}\nworkspace: {}",
                resolved.execution_profile.name, endpoint, workspace,
            )
        }
        Err(error) => describe_resolve_error(error),
    }
}

pub fn run_room_healthcheck(room_id: String) -> Result<(), String> {
    let (profile_name, transport) = with_context_mut(|ctx| {
        let manager = BridgeManager::from_parts(ctx.store.clone(), ctx.state.clone());
        let (resolved, transport) = manager
            .transport_for_room(&room_id)
            .map_err(|error| error.to_string())?;
        Ok((resolved.execution_profile.name, transport))
    })?;

    spawn_on_tokio(async move {
        let result = transport
            .healthcheck()
            .await
            .map(|_| format!("Healthcheck succeeded for \"{profile_name}\"."))
            .map_err(|error| error.to_string());
        Cx::post_action(CrewSettingsAction::HealthcheckFinished { room_id, result });
    });
    Ok(())
}

pub fn stream_room_prompt(room_id: String, prompt: String) -> Result<(), String> {
    let (resolved, transport, request) = with_context_mut(|ctx| {
        let mut manager = BridgeManager::from_parts(ctx.store.clone(), ctx.state.clone());
        let prepared = manager
            .prepare_room_message(&room_id, prompt)
            .map_err(|error| error.to_string())?;
        ctx.state = manager.state().clone();
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok(prepared)
    })?;

    let profile_name = resolved.execution_profile.name.clone();
    spawn_on_tokio(async move {
        Cx::post_action(CrewSettingsAction::StreamStarted {
            room_id: room_id.clone(),
            profile_name,
        });

        let mut accumulated = String::new();
        let stream_result = transport.submit_stream(request).await;
        let mut stream = match stream_result {
            Ok(stream) => stream,
            Err(error) => {
                Cx::post_action(CrewSettingsAction::StreamFailed {
                    room_id: room_id.clone(),
                    error: error.to_string(),
                });
                return;
            }
        };

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(BridgeEvent::TextDelta { text }) => {
                    accumulated.push_str(&text);
                    Cx::post_action(CrewSettingsAction::StreamDelta {
                        room_id: room_id.clone(),
                        text,
                    });
                }
                Ok(BridgeEvent::Done { content, .. }) => {
                    if accumulated.is_empty() {
                        accumulated = content;
                    }
                    Cx::post_action(CrewSettingsAction::StreamFinished {
                        room_id: room_id.clone(),
                        text: accumulated,
                    });
                    return;
                }
                Ok(BridgeEvent::Error { message }) => {
                    Cx::post_action(CrewSettingsAction::StreamFailed {
                        room_id: room_id.clone(),
                        error: message,
                    });
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    Cx::post_action(CrewSettingsAction::StreamFailed {
                        room_id: room_id.clone(),
                        error: error.to_string(),
                    });
                    return;
                }
            }
        }

        Cx::post_action(CrewSettingsAction::StreamFinished {
            room_id,
            text: accumulated,
        });
    });
    Ok(())
}

fn bridge_state_dir(user_id: &matrix_sdk::ruma::OwnedUserId) -> PathBuf {
    persistent_state_dir(user_id).join("crew_bridge")
}

fn current_user_snapshot(user_id: &matrix_sdk::ruma::OwnedUserId) -> UserSnapshot {
    UserSnapshot {
        matrix_user_id: Some(user_id.to_string()),
        homeserver_url: get_client().map(|client| client.homeserver().to_string()),
    }
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed.to_string())
}

fn with_context_mut<R>(
    f: impl FnOnce(&mut BridgeContext) -> Result<R, String>,
) -> Result<R, String> {
    ensure_loaded_for_current_user()?;
    let mut guard = BRIDGE_CONTEXT.lock().unwrap();
    let Some(ctx) = guard.as_mut() else {
        return Err("Crew bridge state is not loaded.".into());
    };
    f(ctx)
}

fn describe_resolve_error(error: ResolveError) -> String {
    match error {
        ResolveError::UnknownRoom(room_id) => {
            format!("Room {room_id} is not present in the Crew inventory yet.")
        }
        ResolveError::NoExecutionProfile(room_id) => format!(
            "Room {room_id} has no execution profile. Save a default profile or bind the room."
        ),
        ResolveError::UnknownExecutionProfile(profile_id) => {
            format!("Execution profile {profile_id} is missing from the state file.")
        }
        ResolveError::UnknownProvider(provider_id) => {
            format!("Provider {provider_id} is missing from the state file.")
        }
        ResolveError::UnknownWorkspace(workspace_id) => {
            format!("Workspace {workspace_id} is missing from the state file.")
        }
    }
}
