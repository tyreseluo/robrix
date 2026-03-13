use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use futures_util::StreamExt;
use makepad_widgets::Cx;
use robrix_botfather::{
    BindingSource, BotBinding, BotDefinition, BotEvent, BotRuntime, BotfatherDefaults,
    BotfatherManager, BotfatherState, DeliveryTarget, InventorySnapshot, OpenClawRuntimeConfig,
    PermissionPolicy, ResolveError, RoomInventory, RuntimeConfig, RuntimeKind, RuntimeProfile,
    SpaceInventory, StateStore, TriggerMode, TriggerPolicy, UserSnapshot, Workspace,
    resolve_room_bot, resolve_room_bots, runtime_feature_enabled,
};

use crate::{
    home::rooms_list::RoomsListRef,
    persistence::matrix_state::persistent_state_dir,
    shared::popup_list::PopupKind,
    sliding_sync::{current_user_id, get_client, spawn_on_tokio, spawn_on_tokio_with_handle},
};
use tokio::task::JoinHandle;

pub mod commands;

const DEFAULT_WORKSPACE_ID: &str = "default-botfather-workspace";
const DEFAULT_CREW_RUNTIME_ID: &str = "default-crew-runtime";
const DEFAULT_OPENCLAW_RUNTIME_ID: &str = "default-openclaw-runtime";
const DEFAULT_CREW_BOT_ID: &str = "default-crew-bot";
const DEFAULT_OPENCLAW_BOT_ID: &str = "default-openclaw-bot";

static BRIDGE_CONTEXT: Mutex<Option<BridgeContext>> = Mutex::new(None);
static ACTIVE_STREAMS: Mutex<Vec<ActiveBotStream>> = Mutex::new(Vec::new());

struct BridgeContext {
    user_id: String,
    store: StateStore,
    state: BotfatherState,
}

struct ActiveBotStream {
    room_id: String,
    local_created_at: u64,
    task: JoinHandle<()>,
}

#[derive(Clone, Debug, Default)]
pub struct DefaultConfigForm {
    pub crew_endpoint: String,
    pub crew_auth_token_env: String,
    pub openclaw_gateway_url: String,
    pub openclaw_auth_token_env: String,
    pub workspace_root: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoomBotOption {
    pub bot_id: String,
    pub label: String,
    pub runtime_kind: RuntimeKind,
}

#[derive(Clone, Debug)]
pub enum BotfatherAction {
    StateChanged,
    Status(String),
    CommandFeedback {
        message: String,
        kind: PopupKind,
        auto_dismissal_duration: Option<f64>,
    },
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
    StreamCancelled {
        room_id: String,
    },
}

pub fn clear_loaded_state() {
    abort_all_active_streams();
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

pub fn default_config_form() -> DefaultConfigForm {
    snapshot().map_or_else(DefaultConfigForm::default, |state| {
        let crew_runtime = state.runtime_profiles.get(DEFAULT_CREW_RUNTIME_ID);
        let openclaw_runtime = state.runtime_profiles.get(DEFAULT_OPENCLAW_RUNTIME_ID);
        let workspace = state.workspaces.get(DEFAULT_WORKSPACE_ID);

        let (crew_endpoint, crew_auth_token_env) = match crew_runtime.map(|runtime| &runtime.config)
        {
            Some(RuntimeConfig::Crew {
                base_url,
                api_key_env,
            }) => (base_url.clone(), api_key_env.clone().unwrap_or_default()),
            _ => (String::new(), String::new()),
        };

        let (openclaw_gateway_url, openclaw_auth_token_env) =
            match openclaw_runtime.map(|runtime| &runtime.config) {
                Some(RuntimeConfig::OpenClaw(config)) => (
                    config.gateway_url.clone(),
                    config.auth_token_env.clone().unwrap_or_default(),
                ),
                _ => (String::new(), String::new()),
            };

        DefaultConfigForm {
            crew_endpoint,
            crew_auth_token_env,
            openclaw_gateway_url,
            openclaw_auth_token_env,
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
    let room_snapshots = rooms_list.bot_room_snapshots();
    let space_snapshots = rooms_list.bot_space_snapshots();
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

pub fn save_default_profiles(
    crew_endpoint: &str,
    crew_auth_token_env: &str,
    openclaw_gateway_url: &str,
    openclaw_auth_token_env: &str,
    workspace_root: &str,
) -> Result<(), String> {
    ensure_loaded_for_current_user()?;

    let crew_endpoint = crew_endpoint.trim();
    let openclaw_gateway_url = openclaw_gateway_url.trim();
    if crew_endpoint.is_empty() && openclaw_gateway_url.is_empty() {
        return Err("Configure at least one runtime: Crew or OpenClaw.".into());
    }
    if !crew_endpoint.is_empty() && !runtime_feature_enabled(RuntimeKind::Crew) {
        return Err("This build of Robrix BotFather does not include the Crew runtime.".into());
    }
    if !openclaw_gateway_url.is_empty() && !runtime_feature_enabled(RuntimeKind::OpenClaw) {
        return Err("This build of Robrix BotFather does not include the OpenClaw runtime.".into());
    }

    with_context_mut(|ctx| {
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
                            "Default workspace configured from Robrix BotFather settings.".into(),
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

        if crew_endpoint.is_empty() {
            remove_bot_runtime(ctx, DEFAULT_CREW_RUNTIME_ID, DEFAULT_CREW_BOT_ID);
        } else {
            ctx.state.runtime_profiles.insert(
                DEFAULT_CREW_RUNTIME_ID.to_string(),
                RuntimeProfile {
                    id: DEFAULT_CREW_RUNTIME_ID.to_string(),
                    name: "Crew Runtime".into(),
                    workspace_id: workspace_id.clone(),
                    description: Some("Default Crew SSE runtime used by Robrix.".into()),
                    config: RuntimeConfig::Crew {
                        base_url: crew_endpoint.to_string(),
                        api_key_env: non_empty(crew_auth_token_env),
                    },
                },
            );
            ctx.state.bots.insert(
                DEFAULT_CREW_BOT_ID.to_string(),
                BotDefinition {
                    id: DEFAULT_CREW_BOT_ID.to_string(),
                    name: "Crew".into(),
                    runtime_profile_id: DEFAULT_CREW_RUNTIME_ID.to_string(),
                    priority: 10,
                    enabled: true,
                    trigger: TriggerPolicy {
                        mode: TriggerMode::Manual,
                        command_prefix: None,
                        mention_name: Some("crew".into()),
                        reply_only: false,
                        thread_only: false,
                    },
                    default_delivery: DeliveryTarget::CurrentRoom,
                    permissions: PermissionPolicy::default(),
                    description: Some("Default Crew bot.".into()),
                },
            );
        }

        if openclaw_gateway_url.is_empty() {
            remove_bot_runtime(ctx, DEFAULT_OPENCLAW_RUNTIME_ID, DEFAULT_OPENCLAW_BOT_ID);
        } else {
            ctx.state.runtime_profiles.insert(
                DEFAULT_OPENCLAW_RUNTIME_ID.to_string(),
                RuntimeProfile {
                    id: DEFAULT_OPENCLAW_RUNTIME_ID.to_string(),
                    name: "OpenClaw Runtime".into(),
                    workspace_id: workspace_id.clone(),
                    description: Some("Default OpenClaw gateway runtime used by Robrix.".into()),
                    config: RuntimeConfig::OpenClaw(OpenClawRuntimeConfig {
                        gateway_url: openclaw_gateway_url.to_string(),
                        auth_token_env: non_empty(openclaw_auth_token_env),
                        agent_id: "main".into(),
                    }),
                },
            );
            ctx.state.bots.insert(
                DEFAULT_OPENCLAW_BOT_ID.to_string(),
                BotDefinition {
                    id: DEFAULT_OPENCLAW_BOT_ID.to_string(),
                    name: "OpenClaw".into(),
                    runtime_profile_id: DEFAULT_OPENCLAW_RUNTIME_ID.to_string(),
                    priority: 0,
                    enabled: true,
                    trigger: TriggerPolicy {
                        mode: TriggerMode::Manual,
                        command_prefix: None,
                        mention_name: Some("openclaw".into()),
                        reply_only: false,
                        thread_only: false,
                    },
                    default_delivery: DeliveryTarget::CurrentRoom,
                    permissions: PermissionPolicy::default(),
                    description: Some("Default OpenClaw bot.".into()),
                },
            );
        }

        ctx.state.defaults = BotfatherDefaults {
            bot_ids: default_bot_ids(&ctx.state),
        };
        cleanup_orphan_bindings(&mut ctx.state);
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())
    })?;

    Cx::post_action(BotfatherAction::StateChanged);
    Ok(())
}

pub fn runtime_summary(runtime_kind: RuntimeKind) -> String {
    let Some(state) = snapshot() else {
        return "BotFather state is not loaded.".into();
    };

    let Some(profile) = runtime_profile_for_kind(&state, runtime_kind) else {
        return match runtime_kind {
            RuntimeKind::Crew => "Crew runtime is not configured yet.".into(),
            RuntimeKind::OpenClaw => "OpenClaw runtime is not configured yet.".into(),
        };
    };

    let (endpoint, auth_env) = match &profile.config {
        RuntimeConfig::Crew {
            base_url,
            api_key_env,
        } => (
            base_url.as_str(),
            api_key_env.as_deref().unwrap_or("(none)"),
        ),
        RuntimeConfig::OpenClaw(config) => (
            config.gateway_url.as_str(),
            config.auth_token_env.as_deref().unwrap_or("(none)"),
        ),
    };
    let workspace = profile
        .workspace_id
        .as_ref()
        .and_then(|workspace_id| state.workspaces.get(workspace_id))
        .map(|workspace| workspace.root_dir.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(none)".into());
    let bot_count = state
        .bots
        .values()
        .filter(|bot| bot.runtime_profile_id == profile.id && bot.enabled)
        .count();

    format!(
        "profile: {}\nendpoint: {}\nauth env: {}\nworkspace: {}\nactive bots: {}",
        profile.name, endpoint, auth_env, workspace, bot_count,
    )
}

pub fn bots_overview() -> String {
    let Some(state) = snapshot() else {
        return "BotFather state is not loaded.".into();
    };

    if state.bots.is_empty() {
        return "No bots configured yet.\nTry `/bot create assistant` in a room.".into();
    }

    let mut lines = Vec::new();
    for bot in state.bots.values() {
        let runtime_label = state
            .runtime_profiles
            .get(&bot.runtime_profile_id)
            .map(|profile| runtime_kind_label(profile.kind()))
            .unwrap_or("missing-runtime");
        let room_bindings = state
            .room_bindings
            .values()
            .filter(|bindings| bindings.iter().any(|binding| binding.bot_id == bot.id))
            .count();
        lines.push(format!(
            "- {} [{}] -> {} | rooms: {}",
            bot.id, runtime_label, bot.runtime_profile_id, room_bindings,
        ));
    }
    lines.join("\n")
}

pub fn workspace_overview() -> String {
    let Some(state) = snapshot() else {
        return "BotFather state is not loaded.".into();
    };

    let workspace = state.workspaces.get(DEFAULT_WORKSPACE_ID);
    let workspace_root = workspace
        .map(|workspace| workspace.root_dir.to_string_lossy().into_owned())
        .unwrap_or_else(|| "(none)".into());

    format!(
        "workspace: {}\nrooms in inventory: {}\nspaces in inventory: {}\nroom overrides: {}",
        workspace_root,
        state.inventory.rooms.len(),
        state.inventory.spaces.len(),
        state.room_bindings.len(),
    )
}

pub fn status_overview(room_id: Option<&str>) -> String {
    let Some(state) = snapshot() else {
        return "BotFather state is not loaded.".into();
    };

    let mut lines = vec![
        format!("bots: {}", state.bots.len()),
        format!("runtime profiles: {}", state.runtime_profiles.len()),
        format!("room overrides: {}", state.room_bindings.len()),
    ];

    if let Some(room_id) = room_id {
        match resolve_room_bot(&state, room_id, None) {
            Ok(resolved) => lines.push(format!(
                "current room -> {} ({})",
                resolved.bot.id,
                runtime_kind_label(resolved.runtime_kind()),
            )),
            Err(error) => lines.push(describe_resolve_error(error)),
        }
    }

    lines.join("\n")
}

pub fn bind_room_to_bot(room_id: &str, bot_selector: &str) -> Result<String, String> {
    ensure_loaded_for_current_user()?;
    let room_id = room_id.trim();
    let selector = bot_selector.trim();
    if selector.is_empty() {
        return Err("Usage: /bot bind <bot-id>".into());
    }

    let bot_name = with_context_mut(|ctx| {
        if !ctx.state.inventory.rooms.contains_key(room_id) {
            return Err(format!(
                "Room {room_id} is not in the current inventory snapshot."
            ));
        }
        let bot_id = resolve_bot_id_selector(&ctx.state, selector)
            .ok_or_else(|| format!("Bot selector `{selector}` did not match any bot."))?;
        let bot = ctx
            .state
            .bots
            .get(&bot_id)
            .ok_or_else(|| format!("Bot {bot_id} is missing from the state file."))?;
        let profile = ctx
            .state
            .runtime_profiles
            .get(&bot.runtime_profile_id)
            .ok_or_else(|| format!("Runtime profile {} is missing.", bot.runtime_profile_id))?;
        if !runtime_feature_enabled(profile.kind()) {
            return Err(format!(
                "{} is not enabled in this build.",
                runtime_kind_label(profile.kind()),
            ));
        }

        ctx.state.room_bindings.insert(
            room_id.to_string(),
            vec![BotBinding {
                bot_id,
                enabled: true,
                priority: 0,
                trigger: None,
                delivery: None,
                permissions: None,
            }],
        );
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok(bot.name.clone())
    })?;

    Cx::post_action(BotfatherAction::StateChanged);
    Ok(format!("Bound this room to bot \"{bot_name}\"."))
}

pub fn create_bot(
    bot_id: &str,
    runtime_profile_selector: Option<&str>,
    room_hint: Option<&str>,
) -> Result<String, String> {
    ensure_loaded_for_current_user()?;
    let bot_id = normalize_identifier(bot_id)?;

    let message = with_context_mut(|ctx| {
        if ctx.state.bots.contains_key(&bot_id) {
            return Err(format!("Bot `{bot_id}` already exists."));
        }

        let runtime_profile_id = match runtime_profile_selector.and_then(non_empty) {
            Some(selector) => resolve_runtime_profile_id_selector(&ctx.state, &selector)
                .ok_or_else(|| format!("Runtime profile selector `{selector}` did not match."))?,
            None => default_runtime_profile_id(&ctx.state, room_hint)
                .ok_or_else(|| "No runtime profile is configured yet.".to_string())?,
        };

        let profile = ctx
            .state
            .runtime_profiles
            .get(&runtime_profile_id)
            .ok_or_else(|| format!("Runtime profile {runtime_profile_id} is missing."))?;
        if !runtime_feature_enabled(profile.kind()) {
            return Err(format!(
                "{} is not enabled in this build.",
                runtime_kind_label(profile.kind()),
            ));
        }

        let bot_name = display_name_from_identifier(&bot_id);
        ctx.state.bots.insert(
            bot_id.clone(),
            BotDefinition {
                id: bot_id.clone(),
                name: bot_name.clone(),
                runtime_profile_id: runtime_profile_id.clone(),
                priority: 0,
                enabled: true,
                trigger: TriggerPolicy {
                    mode: TriggerMode::Manual,
                    command_prefix: Some(format!("/{}", bot_id)),
                    mention_name: Some(bot_id.clone()),
                    reply_only: false,
                    thread_only: false,
                },
                default_delivery: DeliveryTarget::CurrentRoom,
                permissions: PermissionPolicy::default(),
                description: Some("User-defined bot created from Robrix.".into()),
            },
        );
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok(format!(
            "Created bot \"{}\" on profile \"{}\".",
            bot_name, runtime_profile_id
        ))
    })?;

    Cx::post_action(BotfatherAction::StateChanged);
    Ok(message)
}

pub fn set_bot_runtime_profile(
    bot_selector: &str,
    profile_selector: &str,
) -> Result<String, String> {
    ensure_loaded_for_current_user()?;
    let bot_selector = bot_selector.trim();
    let profile_selector = profile_selector.trim();
    if bot_selector.is_empty() || profile_selector.is_empty() {
        return Err("Usage: /bot set-profile <bot-id> <profile-id>".into());
    }

    let message = with_context_mut(|ctx| {
        let bot_id = resolve_bot_id_selector(&ctx.state, bot_selector)
            .ok_or_else(|| format!("Bot selector `{bot_selector}` did not match any bot."))?;
        if is_system_bot_id(&bot_id) {
            return Err(
                "Built-in bots keep their default runtime profiles. Create a custom bot instead."
                    .into(),
            );
        }

        let profile_id = resolve_runtime_profile_id_selector(&ctx.state, profile_selector)
            .ok_or_else(|| {
                format!("Runtime profile selector `{profile_selector}` did not match.")
            })?;
        let profile = ctx
            .state
            .runtime_profiles
            .get(&profile_id)
            .ok_or_else(|| format!("Runtime profile {profile_id} is missing."))?;
        if !runtime_feature_enabled(profile.kind()) {
            return Err(format!(
                "{} is not enabled in this build.",
                runtime_kind_label(profile.kind()),
            ));
        }

        let bot = ctx
            .state
            .bots
            .get_mut(&bot_id)
            .ok_or_else(|| format!("Bot {bot_id} is missing from the state file."))?;
        bot.runtime_profile_id = profile_id.clone();
        let bot_name = bot.name.clone();
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok(format!(
            "Updated bot \"{}\" to profile \"{}\".",
            bot_name, profile_id
        ))
    })?;

    Cx::post_action(BotfatherAction::StateChanged);
    Ok(message)
}

pub fn bind_room_to_default(room_id: &str) -> Result<(), String> {
    ensure_loaded_for_current_user()?;
    let default_kind = with_context_mut(|ctx| {
        resolve_room_bot(&ctx.state, room_id, None)
            .map(|resolved| resolved.runtime_kind())
            .map_err(|error| describe_resolve_error(error))
    })?;
    bind_room_to_runtime(room_id, default_kind)
}

pub fn bind_room_to_runtime(room_id: &str, runtime_kind: RuntimeKind) -> Result<(), String> {
    ensure_loaded_for_current_user()?;
    if !runtime_feature_enabled(runtime_kind) {
        return Err(match runtime_kind {
            RuntimeKind::Crew => {
                "This build of Robrix BotFather does not include the Crew runtime.".into()
            }
            RuntimeKind::OpenClaw => {
                "This build of Robrix BotFather does not include the OpenClaw runtime.".into()
            }
        });
    }
    with_context_mut(|ctx| {
        if !ctx.state.inventory.rooms.contains_key(room_id) {
            return Err(format!(
                "Room {room_id} is not in the current inventory snapshot."
            ));
        }

        let Some(bot_id) = default_bot_id_for_runtime(&ctx.state, runtime_kind) else {
            return Err(match runtime_kind {
                RuntimeKind::Crew => {
                    "Crew is not configured yet. Save a Crew runtime first.".into()
                }
                RuntimeKind::OpenClaw => {
                    "OpenClaw is not configured yet. Save an OpenClaw runtime first.".into()
                }
            });
        };

        ctx.state.room_bindings.insert(
            room_id.to_string(),
            vec![BotBinding {
                bot_id,
                enabled: true,
                priority: 0,
                trigger: None,
                delivery: None,
                permissions: None,
            }],
        );
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())
    })?;

    Cx::post_action(BotfatherAction::StateChanged);
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

    Cx::post_action(BotfatherAction::StateChanged);
    Ok(())
}

pub fn room_primary_runtime_kind(room_id: &str) -> Option<RuntimeKind> {
    let state = snapshot()?;
    resolve_room_bot(&state, room_id, None)
        .ok()
        .map(|resolved| resolved.runtime_kind())
}

pub fn room_primary_bot_id(room_id: &str) -> Option<String> {
    let state = snapshot()?;
    resolve_room_bot(&state, room_id, None)
        .ok()
        .map(|resolved| resolved.bot.id)
}

pub fn room_bot_options(_room_id: Option<&str>) -> Vec<RoomBotOption> {
    let Some(state) = snapshot() else {
        return Vec::new();
    };

    let mut options = state
        .bots
        .values()
        .filter_map(|bot| {
            if !bot.enabled {
                return None;
            }
            let profile = state.runtime_profiles.get(&bot.runtime_profile_id)?;
            if !runtime_feature_enabled(profile.kind()) {
                return None;
            }

            Some(RoomBotOption {
                bot_id: bot.id.clone(),
                label: format!("{} ({})", bot.name, runtime_kind_label(profile.kind())),
                runtime_kind: profile.kind(),
            })
        })
        .collect::<Vec<_>>();

    options.sort_by(|lhs, rhs| {
        room_bot_runtime_priority(rhs.runtime_kind)
            .cmp(&room_bot_runtime_priority(lhs.runtime_kind))
            .then_with(|| lhs.label.cmp(&rhs.label))
            .then_with(|| lhs.bot_id.cmp(&rhs.bot_id))
    });

    options
}

pub fn describe_room_binding(room_id: &str) -> String {
    let Some(state) = snapshot() else {
        return "BotFather state is not loaded.".into();
    };

    match resolve_room_bots(&state, room_id) {
        Ok(resolved_bots) => {
            let Some(primary) = resolved_bots.first() else {
                return "No bot is currently available for this room.".into();
            };

            let source = match &primary.source {
                BindingSource::Room { .. } => "room binding",
                BindingSource::Space { .. } => "space binding",
                BindingSource::Default => "global default",
            };
            let workspace = primary
                .workspace
                .as_ref()
                .map(|workspace| workspace.root_dir.to_string_lossy().into_owned())
                .unwrap_or_else(|| "(none)".into());
            let runtime_endpoint = match &primary.runtime_profile.config {
                RuntimeConfig::Crew { base_url, .. } => base_url.clone(),
                RuntimeConfig::OpenClaw(config) => config.gateway_url.clone(),
            };
            let available = resolved_bots
                .iter()
                .map(|resolved| format!("{} ({:?})", resolved.bot.name, resolved.runtime_kind()))
                .collect::<Vec<_>>()
                .join(", ");

            format!(
                "main bot: {} ({:?})\nsource: {source}\nruntime: {}\nworkspace: {}\navailable: {}",
                primary.bot.name,
                primary.runtime_kind(),
                runtime_endpoint,
                workspace,
                available,
            )
        }
        Err(error) => describe_resolve_error(error),
    }
}

pub fn run_room_healthcheck(room_id: String) -> Result<(), String> {
    let (bot_name, runtime) = with_context_mut(|ctx| {
        let manager = BotfatherManager::from_parts(ctx.store.clone(), ctx.state.clone());
        let resolved = manager
            .resolve_room_bot(&room_id, None)
            .map_err(describe_resolve_error)?;
        let runtime = manager
            .runtime_for_resolved(&resolved)
            .map_err(|error| error.to_string())?;
        Ok((resolved.bot.name, runtime))
    })?;

    spawn_on_tokio(async move {
        let result = runtime
            .healthcheck()
            .await
            .map(|_| format!("Healthcheck succeeded for \"{bot_name}\"."))
            .map_err(|error| error.to_string());
        let (message, kind) = match &result {
            Ok(message) => (message.clone(), PopupKind::Success),
            Err(error) => (error.clone(), PopupKind::Error),
        };
        Cx::post_action(BotfatherAction::HealthcheckFinished { room_id, result });
        Cx::post_action(BotfatherAction::CommandFeedback {
            message,
            kind,
            auto_dismissal_duration: Some(6.0),
        });
    });
    Ok(())
}

pub fn run_bot_healthcheck(bot_selector: String) -> Result<(), String> {
    let selector = bot_selector.trim().to_string();
    if selector.is_empty() {
        return Err("Usage: /bot health <bot-id>".into());
    }

    let (bot_name, runtime) = with_context_mut(|ctx| {
        let bot_id = resolve_bot_id_selector(&ctx.state, &selector)
            .ok_or_else(|| format!("Bot selector `{selector}` did not match any bot."))?;
        let bot = ctx
            .state
            .bots
            .get(&bot_id)
            .ok_or_else(|| format!("Bot {bot_id} is missing from the state file."))?;
        let profile = ctx
            .state
            .runtime_profiles
            .get(&bot.runtime_profile_id)
            .ok_or_else(|| format!("Runtime profile {} is missing.", bot.runtime_profile_id))?;
        let runtime = robrix_botfather::RuntimeAdapter::from_profile(profile)
            .map_err(|error| error.to_string())?;
        Ok((bot.name.clone(), runtime))
    })?;

    spawn_on_tokio(async move {
        let (message, kind) = match runtime.healthcheck().await {
            Ok(()) => (
                format!("Healthcheck succeeded for \"{bot_name}\"."),
                PopupKind::Success,
            ),
            Err(error) => (
                format!("Healthcheck failed for \"{bot_name}\": {error}"),
                PopupKind::Error,
            ),
        };
        Cx::post_action(BotfatherAction::CommandFeedback {
            message,
            kind,
            auto_dismissal_duration: Some(6.0),
        });
    });
    Ok(())
}

pub fn stream_room_prompt(room_id: String, prompt: String) -> Result<(), String> {
    stream_room_prompt_for_local_echo(
        room_id,
        prompt,
        matrix_sdk::ruma::MilliSecondsSinceUnixEpoch::now()
            .get()
            .into(),
    )
}

pub fn stream_room_prompt_for_local_echo(
    room_id: String,
    prompt: String,
    local_created_at: u64,
) -> Result<(), String> {
    let (bot_name, runtime, request) = with_context_mut(|ctx| {
        let mut manager = BotfatherManager::from_parts(ctx.store.clone(), ctx.state.clone());
        let (resolved, runtime, request) = manager
            .prepare_dispatch(&room_id, None, prompt, None)
            .map_err(|error| error.to_string())?;
        ctx.state = manager.state().clone();
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok((resolved.bot.name, runtime, request))
    })?;

    let tracked_room_id = room_id.clone();
    let task = spawn_on_tokio_with_handle(async move {
        Cx::post_action(BotfatherAction::StreamStarted {
            room_id: room_id.clone(),
            profile_name: bot_name,
        });

        let mut accumulated = String::new();
        let stream_result = runtime.dispatch_stream(request).await;
        let mut stream = match stream_result {
            Ok(stream) => stream,
            Err(error) => {
                clear_active_stream(&room_id, local_created_at);
                Cx::post_action(BotfatherAction::StreamFailed {
                    room_id: room_id.clone(),
                    error: error.to_string(),
                });
                return;
            }
        };

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(BotEvent::TextDelta { text }) => {
                    accumulated.push_str(&text);
                    Cx::post_action(BotfatherAction::StreamDelta {
                        room_id: room_id.clone(),
                        text,
                    });
                }
                Ok(BotEvent::Done { content }) => {
                    if accumulated.is_empty() {
                        accumulated = content;
                    }
                    clear_active_stream(&room_id, local_created_at);
                    Cx::post_action(BotfatherAction::StreamFinished {
                        room_id: room_id.clone(),
                        text: accumulated,
                    });
                    return;
                }
                Ok(BotEvent::Error { message }) => {
                    clear_active_stream(&room_id, local_created_at);
                    Cx::post_action(BotfatherAction::StreamFailed {
                        room_id: room_id.clone(),
                        error: message,
                    });
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    clear_active_stream(&room_id, local_created_at);
                    Cx::post_action(BotfatherAction::StreamFailed {
                        room_id: room_id.clone(),
                        error: error.to_string(),
                    });
                    return;
                }
            }
        }

        clear_active_stream(&room_id, local_created_at);
        Cx::post_action(BotfatherAction::StreamFinished {
            room_id,
            text: accumulated,
        });
    });
    register_active_stream(tracked_room_id, local_created_at, task);
    Ok(())
}

pub fn cancel_stream_for_local_echo(room_id: &str, local_created_at: u64) -> bool {
    let Some(task) = take_active_stream(room_id, local_created_at) else {
        return false;
    };

    task.abort();
    Cx::post_action(BotfatherAction::StreamCancelled {
        room_id: room_id.to_string(),
    });
    true
}

fn snapshot() -> Option<BotfatherState> {
    ensure_loaded_for_current_user().ok()?;
    BRIDGE_CONTEXT
        .lock()
        .unwrap()
        .as_ref()
        .map(|ctx| ctx.state.clone())
}

fn bridge_state_dir(user_id: &matrix_sdk::ruma::OwnedUserId) -> PathBuf {
    persistent_state_dir(user_id).join("botfather")
}

fn current_user_snapshot(user_id: &matrix_sdk::ruma::OwnedUserId) -> UserSnapshot {
    UserSnapshot {
        matrix_user_id: Some(user_id.to_string()),
        homeserver_url: get_client().map(|client| client.homeserver().to_string()),
    }
}

fn register_active_stream(room_id: String, local_created_at: u64, task: JoinHandle<()>) {
    if task.is_finished() {
        return;
    }
    let mut active_streams = ACTIVE_STREAMS.lock().unwrap();
    if let Some(index) = active_streams
        .iter()
        .position(|stream| stream.room_id == room_id && stream.local_created_at == local_created_at)
    {
        active_streams.swap_remove(index).task.abort();
    }
    active_streams.push(ActiveBotStream {
        room_id,
        local_created_at,
        task,
    });
}

fn take_active_stream(room_id: &str, local_created_at: u64) -> Option<JoinHandle<()>> {
    let mut active_streams = ACTIVE_STREAMS.lock().unwrap();
    active_streams
        .iter()
        .position(|stream| stream.room_id == room_id && stream.local_created_at == local_created_at)
        .map(|index| active_streams.swap_remove(index).task)
}

fn clear_active_stream(room_id: &str, local_created_at: u64) {
    let _ = take_active_stream(room_id, local_created_at);
}

fn abort_all_active_streams() {
    let mut active_streams = ACTIVE_STREAMS.lock().unwrap();
    for active_stream in active_streams.drain(..) {
        active_stream.task.abort();
    }
}

fn default_bot_ids(state: &BotfatherState) -> Vec<String> {
    [DEFAULT_CREW_BOT_ID, DEFAULT_OPENCLAW_BOT_ID]
        .into_iter()
        .filter(|bot_id| state.bots.get(*bot_id).is_some_and(|bot| bot.enabled))
        .map(ToOwned::to_owned)
        .collect()
}

fn default_bot_id_for_runtime(state: &BotfatherState, runtime_kind: RuntimeKind) -> Option<String> {
    let target_runtime_id = match runtime_kind {
        RuntimeKind::Crew => DEFAULT_CREW_RUNTIME_ID,
        RuntimeKind::OpenClaw => DEFAULT_OPENCLAW_RUNTIME_ID,
    };

    state
        .bots
        .values()
        .find(|bot| bot.enabled && bot.runtime_profile_id == target_runtime_id)
        .map(|bot| bot.id.clone())
}

fn runtime_profile_for_kind(
    state: &BotfatherState,
    runtime_kind: RuntimeKind,
) -> Option<&RuntimeProfile> {
    let runtime_id = match runtime_kind {
        RuntimeKind::Crew => DEFAULT_CREW_RUNTIME_ID,
        RuntimeKind::OpenClaw => DEFAULT_OPENCLAW_RUNTIME_ID,
    };

    state.runtime_profiles.get(runtime_id).or_else(|| {
        state.runtime_profiles.values().find(|profile| {
            profile.kind() == runtime_kind && runtime_feature_enabled(profile.kind())
        })
    })
}

fn default_runtime_profile_id(state: &BotfatherState, room_hint: Option<&str>) -> Option<String> {
    if let Some(room_id) = room_hint {
        if let Ok(resolved) = resolve_room_bot(state, room_id, None) {
            return Some(resolved.runtime_profile.id);
        }
    }

    [RuntimeKind::Crew, RuntimeKind::OpenClaw]
        .into_iter()
        .find_map(|kind| runtime_profile_for_kind(state, kind).map(|profile| profile.id.clone()))
}

fn resolve_bot_id_selector(state: &BotfatherState, selector: &str) -> Option<String> {
    let selector = selector.trim();
    if selector.is_empty() {
        return None;
    }
    let normalized = selector.trim_start_matches('@');

    state
        .bots
        .get(selector)
        .map(|bot| bot.id.clone())
        .or_else(|| state.bots.get(normalized).map(|bot| bot.id.clone()))
        .or_else(|| {
            state
                .bots
                .values()
                .find(|bot| bot.id.eq_ignore_ascii_case(normalized))
                .map(|bot| bot.id.clone())
        })
        .or_else(|| {
            state
                .bots
                .values()
                .find(|bot| bot.name.eq_ignore_ascii_case(selector))
                .map(|bot| bot.id.clone())
        })
        .or_else(|| {
            state
                .bots
                .values()
                .find(|bot| {
                    bot.trigger
                        .mention_name
                        .as_deref()
                        .is_some_and(|mention| mention.eq_ignore_ascii_case(normalized))
                })
                .map(|bot| bot.id.clone())
        })
}

fn resolve_runtime_profile_id_selector(state: &BotfatherState, selector: &str) -> Option<String> {
    let selector = selector.trim();
    if selector.is_empty() {
        return None;
    }

    state
        .runtime_profiles
        .get(selector)
        .map(|profile| profile.id.clone())
        .or_else(|| {
            state
                .runtime_profiles
                .values()
                .find(|profile| profile.id.eq_ignore_ascii_case(selector))
                .map(|profile| profile.id.clone())
        })
        .or_else(|| {
            state
                .runtime_profiles
                .values()
                .find(|profile| profile.name.eq_ignore_ascii_case(selector))
                .map(|profile| profile.id.clone())
        })
        .or_else(|| match selector.to_ascii_lowercase().as_str() {
            "crew" => {
                runtime_profile_for_kind(state, RuntimeKind::Crew).map(|profile| profile.id.clone())
            }
            "openclaw" => runtime_profile_for_kind(state, RuntimeKind::OpenClaw)
                .map(|profile| profile.id.clone()),
            _ => None,
        })
}

fn runtime_kind_label(kind: RuntimeKind) -> &'static str {
    match kind {
        RuntimeKind::Crew => "crew",
        RuntimeKind::OpenClaw => "openclaw",
    }
}

fn room_bot_runtime_priority(kind: RuntimeKind) -> i32 {
    match kind {
        RuntimeKind::Crew => 2,
        RuntimeKind::OpenClaw => 1,
    }
}

fn display_name_from_identifier(identifier: &str) -> String {
    identifier
        .split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut word = first.to_ascii_uppercase().to_string();
                    word.push_str(chars.as_str());
                    word
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_identifier(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Identifier cannot be empty.".into());
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return Err("Identifier may only contain ASCII letters, digits, '-' and '_'.".into());
    }
    Ok(trimmed.to_ascii_lowercase())
}

fn is_system_bot_id(bot_id: &str) -> bool {
    matches!(bot_id, DEFAULT_CREW_BOT_ID | DEFAULT_OPENCLAW_BOT_ID)
}

fn remove_bot_runtime(ctx: &mut BridgeContext, runtime_id: &str, bot_id: &str) {
    ctx.state.runtime_profiles.remove(runtime_id);
    ctx.state.bots.remove(bot_id);
}

fn cleanup_orphan_bindings(state: &mut BotfatherState) {
    state.room_bindings.retain(|_, bindings| {
        bindings.retain(|binding| state.bots.contains_key(&binding.bot_id));
        !bindings.is_empty()
    });
    state.space_bindings.retain(|_, bindings| {
        bindings.retain(|binding| state.bots.contains_key(&binding.bot_id));
        !bindings.is_empty()
    });
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
        return Err("BotFather state is not loaded.".into());
    };
    f(ctx)
}

fn describe_resolve_error(error: ResolveError) -> String {
    match error {
        ResolveError::UnknownRoom(room_id) => {
            format!("Room {room_id} is not present in the bot inventory yet.")
        }
        ResolveError::UnknownBot(bot_id) => format!("Bot {bot_id} is missing from the state file."),
        ResolveError::UnknownRuntimeProfile(profile_id) => {
            format!("Runtime profile {profile_id} is missing from the state file.")
        }
        ResolveError::UnknownWorkspace(workspace_id) => {
            format!("Workspace {workspace_id} is missing from the state file.")
        }
        ResolveError::NoBotsConfigured(room_id) => {
            format!("Room {room_id} has no bot configured. Save runtime profiles or bind the room.")
        }
        ResolveError::PreferredBotNotAvailable { room_id, bot_id } => {
            format!("Bot {bot_id} is not currently available for room {room_id}.")
        }
    }
}
