use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::Mutex,
};

use futures_util::StreamExt;
use makepad_widgets::{Cx, error};
use matrix_sdk::send_queue::SendHandle;
use robrix_botfather::{
    BindingSource, BotBinding, BotDefinition, BotEvent, BotRuntime, BotRuntimeOverride,
    BotfatherDefaults, BotfatherManager, BotfatherState, DeliveryTarget, DispatchPolicy,
    InventorySnapshot, OpenClawRuntimeConfig, PermissionPolicy, ResolveError, RoomInventory,
    RuntimeConfig, RuntimeKind, RuntimeProfile, SpaceInventory, StateStore, TriggerMode,
    TriggerPolicy, UserSnapshot, Workspace,
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
static QUEUED_STREAMS: Mutex<VecDeque<QueuedBotStream>> = Mutex::new(VecDeque::new());
static STREAM_PREVIEWS: Mutex<Vec<StreamPreviewState>> = Mutex::new(Vec::new());
static DIRECT_STREAM_MESSAGES: Mutex<Vec<DirectStreamMessageState>> = Mutex::new(Vec::new());

struct BridgeContext {
    user_id: String,
    store: StateStore,
    state: BotfatherState,
}

struct ActiveBotStream {
    room_id: String,
    thread_root_event_id: Option<String>,
    runtime_profile_id: String,
    local_created_at: u64,
    task: JoinHandle<()>,
}

struct QueuedBotStream {
    room_id: String,
    thread_root_event_id: Option<String>,
    reply_root_event_id: Option<String>,
    runtime_profile_id: String,
    runtime_kind: RuntimeKind,
    profile_name: String,
    dispatch_policy: DispatchPolicy,
    local_created_at: u64,
    runtime: robrix_botfather::RuntimeAdapter,
    request: robrix_botfather::BotRequest,
}

struct PreparedBotStream {
    room_id: String,
    thread_root_event_id: Option<String>,
    reply_root_event_id: Option<String>,
    runtime_profile_id: String,
    runtime_kind: RuntimeKind,
    profile_name: String,
    dispatch_policy: DispatchPolicy,
    runtime: robrix_botfather::RuntimeAdapter,
    request: robrix_botfather::BotRequest,
}

struct StreamPreviewState {
    room_id: String,
    thread_root_event_id: Option<String>,
    runtime_kind: RuntimeKind,
    bot_name: String,
    status: BotStreamPreviewStatus,
    text: String,
    detail: String,
    can_post: bool,
}

struct DirectStreamMessageState {
    room_id: String,
    thread_root_event_id: Option<String>,
    send_handle: Option<SendHandle>,
    pending_action: Option<DirectStreamPendingAction>,
}

#[derive(Clone)]
enum DirectStreamPendingAction {
    Finalize(String),
    Cancel(Option<String>),
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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BotDispatchContext {
    pub thread_root_event_id: Option<String>,
    pub reply_root_event_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BotStreamPreviewStatus {
    #[default]
    Idle,
    Queued,
    Streaming,
    Finished,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BotStreamPreviewSnapshot {
    pub room_id: String,
    pub thread_root_event_id: Option<String>,
    pub runtime_kind: Option<RuntimeKind>,
    pub bot_name: String,
    pub status: BotStreamPreviewStatus,
    pub text: String,
    pub detail: String,
    pub can_post: bool,
}

pub enum DirectStreamHandleState {
    Ready(SendHandle),
    Pending,
    Missing,
}

pub struct InterruptScopeResult {
    pub interrupted: bool,
    pub queued_remaining_in_scope: bool,
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
    StreamQueued {
        room_id: String,
        thread_root_event_id: Option<String>,
        profile_name: String,
        position: usize,
    },
    StreamStarted {
        room_id: String,
        thread_root_event_id: Option<String>,
        profile_name: String,
    },
    StreamDelta {
        room_id: String,
        thread_root_event_id: Option<String>,
        text: String,
    },
    StreamFinished {
        room_id: String,
        thread_root_event_id: Option<String>,
        text: String,
    },
    StreamFailed {
        room_id: String,
        thread_root_event_id: Option<String>,
        error: String,
    },
    StreamCancelled {
        room_id: String,
        thread_root_event_id: Option<String>,
    },
}

pub fn clear_loaded_state() {
    abort_all_active_streams();
    clear_all_stream_queues();
    clear_all_stream_previews();
    clear_all_direct_stream_messages();
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
                ..
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
                    dispatch_policy: DispatchPolicy::default(),
                    config: RuntimeConfig::Crew {
                        base_url: crew_endpoint.to_string(),
                        api_key_env: non_empty(crew_auth_token_env),
                        model: None,
                        system_prompt: None,
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
                    runtime_override: BotRuntimeOverride::default(),
                    dispatch_policy_override: None,
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
                    dispatch_policy: DispatchPolicy::default(),
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
                    runtime_override: BotRuntimeOverride::default(),
                    dispatch_policy_override: None,
                    description: Some("Default OpenClaw bot.".into()),
                },
            );
        }

        ctx.state.defaults = BotfatherDefaults {
            bot_ids: default_bot_ids(&ctx.state),
            room_stream_preview_enabled: ctx.state.defaults.room_stream_preview_enabled,
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
            model,
            system_prompt,
        } => (
            format!(
                "{}\nmodel: {}\nsystem prompt: {}",
                base_url,
                model.as_deref().unwrap_or("(default)"),
                system_prompt
                    .as_ref()
                    .map(|prompt| if prompt.is_empty() { "(cleared)" } else { "(custom)" })
                    .unwrap_or("(default)")
            ),
            api_key_env.as_deref().unwrap_or("(none)").to_string(),
        ),
        RuntimeConfig::OpenClaw(config) => (
            format!("{}\nagent: {}", config.gateway_url, config.agent_id),
            config.auth_token_env.as_deref().unwrap_or("(none)").to_string(),
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
        "profile: {}\nendpoint: {}\nauth env: {}\nworkspace: {}\npolicy: room {}, runtime {}, queue {}\nactive bots: {}",
        profile.name,
        endpoint,
        auth_env,
        workspace,
        profile.dispatch_policy.max_parallel_per_room,
        profile.dispatch_policy.max_parallel_per_runtime,
        profile.dispatch_policy.queue_limit,
        bot_count,
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
            "- {} [{}] -> {} | rooms: {} | override: {}",
            bot.id,
            runtime_label,
            bot.runtime_profile_id,
            room_bindings,
            bot_override_summary(&bot.runtime_override),
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
        format!("active bot streams: {}", ACTIVE_STREAMS.lock().unwrap().len()),
        format!("queued bot streams: {}", QUEUED_STREAMS.lock().unwrap().len()),
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

pub fn diagnostics_overview() -> String {
    let Some(state) = snapshot() else {
        return "BotFather state is not loaded.".into();
    };

    let state_path = BRIDGE_CONTEXT
        .lock()
        .unwrap()
        .as_ref()
        .map(|ctx| ctx.store.path().display().to_string())
        .unwrap_or_else(|| "(unavailable)".into());
    let preview_count = STREAM_PREVIEWS.lock().unwrap().len();
    let queue_len = QUEUED_STREAMS.lock().unwrap().len();
    let active_len = ACTIVE_STREAMS.lock().unwrap().len();

    format!(
        "state file: {}\nstate version: {}\nrooms: {}\nspaces: {}\nactive sessions: {}\nactive streams: {}\nqueued streams: {}\npreview buffers: {}\npreview mode: {}\ndefault bots: {}",
        state_path,
        state.version,
        state.inventory.rooms.len(),
        state.inventory.spaces.len(),
        state.runtime.active_sessions.len(),
        active_len,
        queue_len,
        preview_count,
        if state.defaults.room_stream_preview_enabled {
            "manual-post"
        } else {
            "auto-send"
        },
        state.defaults.bot_ids.join(", "),
    )
}

pub fn room_stream_preview_enabled() -> bool {
    snapshot()
        .map(|state| state.defaults.room_stream_preview_enabled)
        .unwrap_or(false)
}

pub fn direct_stream_message_body(thread_root_event_id: Option<&str>) -> String {
    format!(
        "!BOT_STREAM|{}|",
        thread_root_event_id.unwrap_or("main"),
    )
}

pub fn request_direct_stream_message(room_id: &str, thread_root_event_id: Option<&str>) -> bool {
    let mut messages = DIRECT_STREAM_MESSAGES.lock().unwrap();
    if messages
        .iter()
        .any(|message| direct_stream_scope_matches(message, room_id, thread_root_event_id))
    {
        return false;
    }

    messages.push(DirectStreamMessageState {
        room_id: room_id.to_string(),
        thread_root_event_id: thread_root_event_id.map(ToOwned::to_owned),
        send_handle: None,
        pending_action: None,
    });
    true
}

pub fn attach_direct_stream_message_handle(
    room_id: &str,
    thread_root_event_id: Option<&str>,
    send_handle: SendHandle,
) {
    let mut messages = DIRECT_STREAM_MESSAGES.lock().unwrap();
    let Some(index) = messages
        .iter()
        .position(|message| direct_stream_scope_matches(message, room_id, thread_root_event_id))
    else {
        return;
    };

    let pending_action = messages[index].pending_action.clone();
    messages[index].send_handle = Some(send_handle.clone());
    if let Some(pending_action) = pending_action {
        resolve_direct_stream_pending_action(send_handle, pending_action);
    }
}

pub fn has_live_direct_stream_message(
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> bool {
    DIRECT_STREAM_MESSAGES.lock().unwrap().iter().any(|message| {
        direct_stream_scope_matches(message, room_id, thread_root_event_id)
    })
}

pub fn finalize_direct_stream_message(
    room_id: &str,
    thread_root_event_id: Option<&str>,
    text: String,
) -> DirectStreamHandleState {
    update_direct_stream_message_state(
        room_id,
        thread_root_event_id,
        DirectStreamPendingAction::Finalize(text),
    )
}

pub fn cancel_direct_stream_message(
    room_id: &str,
    thread_root_event_id: Option<&str>,
    fallback_text: Option<String>,
) -> DirectStreamHandleState {
    update_direct_stream_message_state(
        room_id,
        thread_root_event_id,
        DirectStreamPendingAction::Cancel(fallback_text),
    )
}

pub fn clear_direct_stream_message(
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> bool {
    let mut messages = DIRECT_STREAM_MESSAGES.lock().unwrap();
    let Some(index) = messages
        .iter()
        .position(|message| direct_stream_scope_matches(message, room_id, thread_root_event_id))
    else {
        return false;
    };
    messages.swap_remove(index);
    true
}

pub fn prune_terminal_direct_stream_messages(
    room_id: &str,
    present_thread_root_event_ids: &[Option<String>],
) -> usize {
    let mut messages = DIRECT_STREAM_MESSAGES.lock().unwrap();
    let original_len = messages.len();
    messages.retain(|message| {
        if message.room_id != room_id {
            return true;
        }

        if message.send_handle.is_none() || message.pending_action.is_none() {
            return true;
        }

        present_thread_root_event_ids.iter().any(|thread_root_event_id| {
            thread_root_event_id.as_deref() == message.thread_root_event_id.as_deref()
        })
    });
    original_len.saturating_sub(messages.len())
}

fn update_direct_stream_message_state(
    room_id: &str,
    thread_root_event_id: Option<&str>,
    pending_action: DirectStreamPendingAction,
) -> DirectStreamHandleState {
    let mut messages = DIRECT_STREAM_MESSAGES.lock().unwrap();
    let Some(index) = messages
        .iter()
        .position(|message| direct_stream_scope_matches(message, room_id, thread_root_event_id))
    else {
        return DirectStreamHandleState::Missing;
    };

    messages[index].pending_action = Some(pending_action);
    messages[index]
        .send_handle
        .clone()
        .map_or(DirectStreamHandleState::Pending, DirectStreamHandleState::Ready)
}

fn resolve_direct_stream_pending_action(
    send_handle: SendHandle,
    pending_action: DirectStreamPendingAction,
) {
    spawn_on_tokio(async move {
        match pending_action {
            DirectStreamPendingAction::Finalize(text) => {
                if let Err(error) = send_handle
                    .edit(matrix_sdk::ruma::events::room::message::RoomMessageEventContent::text_markdown(text).into())
                    .await
                {
                    error!("Failed to finalize delayed direct bot stream message: {error}");
                }
            }
            DirectStreamPendingAction::Cancel(fallback_text) => match send_handle.abort().await {
                Ok(true) => {}
                Ok(false) => {
                    if let Some(text) = fallback_text {
                        if let Err(error) = send_handle
                            .edit(matrix_sdk::ruma::events::room::message::RoomMessageEventContent::text_markdown(text).into())
                            .await
                        {
                            error!("Failed to update delayed cancelled bot stream message: {error}");
                        }
                    }
                }
                Err(error) => {
                    error!("Failed to stop delayed direct bot stream placeholder: {error}");
                }
            },
        }
    });
}

fn direct_stream_scope_matches(
    message: &DirectStreamMessageState,
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> bool {
    message.room_id == room_id
        && message.thread_root_event_id.as_deref() == thread_root_event_id
}

pub fn set_room_stream_preview_enabled(enabled: bool) -> Result<String, String> {
    ensure_loaded_for_current_user()?;
    with_context_mut(|ctx| {
        ctx.state.defaults.room_stream_preview_enabled = enabled;
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())
    })?;
    Cx::post_action(BotfatherAction::StateChanged);
    Ok(if enabled {
        "Bot stream preview is enabled. Room panel will keep the streamed output until you post it."
            .into()
    } else {
        "Bot stream preview is disabled. Finished bot output will be sent back to Matrix automatically."
            .into()
    })
}

pub fn room_stream_preview(
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> Option<BotStreamPreviewSnapshot> {
    STREAM_PREVIEWS
        .lock()
        .unwrap()
        .iter()
        .find(|preview| {
            preview.room_id == room_id
                && preview.thread_root_event_id.as_deref() == thread_root_event_id
        })
        .map(|preview| BotStreamPreviewSnapshot {
            room_id: preview.room_id.clone(),
            thread_root_event_id: preview.thread_root_event_id.clone(),
            runtime_kind: Some(preview.runtime_kind),
            bot_name: preview.bot_name.clone(),
            status: preview.status,
            text: preview.text.clone(),
            detail: preview.detail.clone(),
            can_post: preview.can_post,
        })
}

pub fn clear_room_stream_preview(room_id: &str, thread_root_event_id: Option<&str>) {
    let _ = remove_stream_preview(room_id, thread_root_event_id);
}

pub fn take_room_stream_preview_text(
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> Option<String> {
    let mut previews = STREAM_PREVIEWS.lock().unwrap();
    let preview = previews.iter_mut().find(|preview| {
        preview.room_id == room_id && preview.thread_root_event_id.as_deref() == thread_root_event_id
    })?;
    if !preview.can_post || preview.text.trim().is_empty() {
        return None;
    }

    preview.can_post = false;
    preview.status = BotStreamPreviewStatus::Idle;
    let text = std::mem::take(&mut preview.text);
    preview.detail = "Preview posted to Matrix.".into();
    Some(text)
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
                runtime_override: BotRuntimeOverride::default(),
                dispatch_policy_override: None,
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

pub fn set_bot_model(bot_selector: &str, model: &str) -> Result<String, String> {
    set_bot_override(bot_selector, "model", model, |bot, profile, value| {
        if profile.kind() != RuntimeKind::Crew {
            return Err("`/bot set-model` is currently only available for Crew-backed bots.".into());
        }
        bot.runtime_override.model = value;
        Ok(format!(
            "Bot \"{}\" will use Crew model `{}` on the next run.",
            bot.name,
            bot.runtime_override
                .model
                .as_deref()
                .unwrap_or("(runtime default)")
        ))
    })
}

pub fn set_bot_system_prompt(bot_selector: &str, prompt: &str) -> Result<String, String> {
    set_bot_override(bot_selector, "prompt", prompt, |bot, profile, value| {
        if profile.kind() != RuntimeKind::Crew {
            return Err(
                "`/bot set-prompt` is currently only available for Crew-backed bots.".into(),
            );
        }
        bot.runtime_override.system_prompt = value;
        Ok(format!(
            "Bot \"{}\" updated its Crew system prompt override.",
            bot.name
        ))
    })
}

pub fn set_bot_agent(bot_selector: &str, agent: &str) -> Result<String, String> {
    set_bot_override(bot_selector, "agent", agent, |bot, profile, value| {
        if profile.kind() != RuntimeKind::OpenClaw {
            return Err(
                "`/bot set-agent` is currently only available for OpenClaw-backed bots.".into(),
            );
        }
        bot.runtime_override.agent_id = value;
        Ok(format!(
            "Bot \"{}\" will use OpenClaw agent `{}` on the next run.",
            bot.name,
            bot.runtime_override
                .agent_id
                .as_deref()
                .unwrap_or("(runtime default)")
        ))
    })
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
                "main bot: {} ({:?})\nsource: {source}\nruntime: {}\nworkspace: {}\noverride: {}\navailable: {}",
                primary.bot.name,
                primary.runtime_kind(),
                runtime_endpoint,
                workspace,
                bot_override_summary(&primary.runtime_override),
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
        BotDispatchContext::default(),
        matrix_sdk::ruma::MilliSecondsSinceUnixEpoch::now()
            .get()
            .into(),
    )
}

pub fn stream_room_prompt_for_local_echo(
    room_id: String,
    prompt: String,
    dispatch_context: BotDispatchContext,
    local_created_at: u64,
) -> Result<(), String> {
    let prepared = with_context_mut(|ctx| {
        let mut manager = BotfatherManager::from_parts(ctx.store.clone(), ctx.state.clone());
        let (resolved, runtime, request) = manager
            .prepare_dispatch(
                &room_id,
                dispatch_context.thread_root_event_id.as_deref(),
                dispatch_context.reply_root_event_id.as_deref(),
                prompt,
                None,
            )
            .map_err(|error| error.to_string())?;
        ctx.state = manager.state().clone();
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok(PreparedBotStream {
            room_id: room_id.clone(),
            thread_root_event_id: request.thread_root_event_id.clone(),
            reply_root_event_id: request.reply_root_event_id.clone(),
            runtime_profile_id: resolved.runtime_profile.id.clone(),
            runtime_kind: resolved.runtime_kind(),
            profile_name: resolved.bot.name.clone(),
            dispatch_policy: resolved.dispatch_policy.clone(),
            runtime,
            request,
        })
    })?;

    update_stream_preview(
        &prepared.room_id,
        prepared.thread_root_event_id.as_deref(),
        prepared.runtime_kind,
        &prepared.profile_name,
        BotStreamPreviewStatus::Idle,
        "Ready to dispatch.".into(),
        String::new(),
        false,
    );

    if can_start_stream(&prepared.room_id, &prepared.runtime_profile_id, &prepared.dispatch_policy) {
        start_prepared_stream(prepared, local_created_at);
        return Ok(());
    }

    enqueue_stream(prepared, local_created_at)?;
    Ok(())
}

pub fn cancel_stream_for_local_echo(room_id: &str, local_created_at: u64) -> bool {
    if let Some(active_stream) = take_active_stream(room_id, local_created_at) {
        active_stream.task.abort();
        mark_stream_cancelled(room_id, active_stream.thread_root_event_id.as_deref());
        Cx::post_action(BotfatherAction::StreamCancelled {
            room_id: room_id.to_string(),
            thread_root_event_id: active_stream.thread_root_event_id,
        });
        drain_stream_queue();
        return true;
    }

    if let Some(queued_stream) = take_queued_stream(room_id, local_created_at) {
        mark_stream_cancelled(room_id, queued_stream.thread_root_event_id.as_deref());
        Cx::post_action(BotfatherAction::StreamCancelled {
            room_id: room_id.to_string(),
            thread_root_event_id: queued_stream.thread_root_event_id,
        });
        return true;
    }

    false
}

pub fn interrupt_active_scope(
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> InterruptScopeResult {
    let Some(active_stream) = take_active_stream_for_scope(room_id, thread_root_event_id) else {
        return InterruptScopeResult {
            interrupted: false,
            queued_remaining_in_scope: has_queued_stream_for_scope(room_id, thread_root_event_id),
        };
    };

    let queued_remaining_in_scope = has_queued_stream_for_scope(room_id, thread_root_event_id);
    active_stream.task.abort();
    mark_stream_cancelled(room_id, active_stream.thread_root_event_id.as_deref());
    Cx::post_action(BotfatherAction::StreamCancelled {
        room_id: room_id.to_string(),
        thread_root_event_id: active_stream.thread_root_event_id,
    });
    drain_stream_queue();
    InterruptScopeResult {
        interrupted: true,
        queued_remaining_in_scope,
    }
}

fn start_prepared_stream(prepared: PreparedBotStream, local_created_at: u64) {
    let room_id = prepared.room_id.clone();
    let thread_root_event_id = prepared.thread_root_event_id.clone();
    let runtime_profile_id = prepared.runtime_profile_id.clone();
    let profile_name = prepared.profile_name.clone();
    let runtime_kind = prepared.runtime_kind;
    let task_room_id = room_id.clone();
    let task_thread_root_event_id = thread_root_event_id.clone();

    update_stream_preview(
        &room_id,
        thread_root_event_id.as_deref(),
        runtime_kind,
        &profile_name,
        BotStreamPreviewStatus::Streaming,
        "Streaming bot response...".into(),
        String::new(),
        false,
    );
    Cx::post_action(BotfatherAction::StreamStarted {
        room_id: room_id.clone(),
        thread_root_event_id: thread_root_event_id.clone(),
        profile_name,
    });

    let task = spawn_on_tokio_with_handle(async move {
        let mut accumulated = String::new();
        let stream_result = prepared.runtime.dispatch_stream(prepared.request).await;
        let mut stream = match stream_result {
            Ok(stream) => stream,
            Err(error) => {
                release_stream_slot(&task_room_id, local_created_at);
                mark_stream_failed(
                    &task_room_id,
                    task_thread_root_event_id.as_deref(),
                    &error.to_string(),
                );
                Cx::post_action(BotfatherAction::StreamFailed {
                    room_id: task_room_id.clone(),
                    thread_root_event_id: task_thread_root_event_id.clone(),
                    error: error.to_string(),
                });
                return;
            }
        };

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(BotEvent::TextDelta { text }) => {
                    accumulated.push_str(&text);
                    append_stream_preview(
                        &task_room_id,
                        task_thread_root_event_id.as_deref(),
                        &text,
                    );
                    Cx::post_action(BotfatherAction::StreamDelta {
                        room_id: task_room_id.clone(),
                        thread_root_event_id: task_thread_root_event_id.clone(),
                        text,
                    });
                }
                Ok(BotEvent::Done { content }) => {
                    if accumulated.is_empty() {
                        accumulated = content;
                    }
                    release_stream_slot(&task_room_id, local_created_at);
                    mark_stream_finished(
                        &task_room_id,
                        task_thread_root_event_id.as_deref(),
                        &accumulated,
                    );
                    Cx::post_action(BotfatherAction::StreamFinished {
                        room_id: task_room_id.clone(),
                        thread_root_event_id: task_thread_root_event_id.clone(),
                        text: accumulated,
                    });
                    return;
                }
                Ok(BotEvent::Error { message }) => {
                    release_stream_slot(&task_room_id, local_created_at);
                    mark_stream_failed(&task_room_id, task_thread_root_event_id.as_deref(), &message);
                    Cx::post_action(BotfatherAction::StreamFailed {
                        room_id: task_room_id.clone(),
                        thread_root_event_id: task_thread_root_event_id.clone(),
                        error: message,
                    });
                    return;
                }
                Ok(_) => {}
                Err(error) => {
                    release_stream_slot(&task_room_id, local_created_at);
                    mark_stream_failed(
                        &task_room_id,
                        task_thread_root_event_id.as_deref(),
                        &error.to_string(),
                    );
                    Cx::post_action(BotfatherAction::StreamFailed {
                        room_id: task_room_id.clone(),
                        thread_root_event_id: task_thread_root_event_id.clone(),
                        error: error.to_string(),
                    });
                    return;
                }
            }
        }

        release_stream_slot(&task_room_id, local_created_at);
        mark_stream_finished(
            &task_room_id,
            task_thread_root_event_id.as_deref(),
            &accumulated,
        );
        Cx::post_action(BotfatherAction::StreamFinished {
            room_id: task_room_id,
            thread_root_event_id: task_thread_root_event_id,
            text: accumulated,
        });
    });

    register_active_stream(
        room_id,
        thread_root_event_id,
        runtime_profile_id,
        local_created_at,
        task,
    );
}

fn enqueue_stream(prepared: PreparedBotStream, local_created_at: u64) -> Result<usize, String> {
    let mut queued_streams = QUEUED_STREAMS.lock().unwrap();
    let queued_for_runtime = queued_streams
        .iter()
        .filter(|item| item.runtime_profile_id == prepared.runtime_profile_id)
        .count();
    if queued_for_runtime >= prepared.dispatch_policy.queue_limit {
        return Err(format!(
            "Bot queue is full for runtime `{}` (limit {}).",
            prepared.profile_name, prepared.dispatch_policy.queue_limit
        ));
    }

    queued_streams.push_back(QueuedBotStream {
        room_id: prepared.room_id.clone(),
        thread_root_event_id: prepared.thread_root_event_id.clone(),
        reply_root_event_id: prepared.reply_root_event_id.clone(),
        runtime_profile_id: prepared.runtime_profile_id.clone(),
        runtime_kind: prepared.runtime_kind,
        profile_name: prepared.profile_name.clone(),
        dispatch_policy: prepared.dispatch_policy.clone(),
        local_created_at,
        runtime: prepared.runtime,
        request: prepared.request,
    });
    let position = queued_streams.len();
    drop(queued_streams);

    update_stream_preview(
        &prepared.room_id,
        prepared.thread_root_event_id.as_deref(),
        prepared.runtime_kind,
        &prepared.profile_name,
        BotStreamPreviewStatus::Queued,
        format!("Queued at position {position}."),
        String::new(),
        false,
    );
    Cx::post_action(BotfatherAction::StreamQueued {
        room_id: prepared.room_id,
        thread_root_event_id: prepared.thread_root_event_id,
        profile_name: prepared.profile_name,
        position,
    });
    Ok(position)
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

fn register_active_stream(
    room_id: String,
    thread_root_event_id: Option<String>,
    runtime_profile_id: String,
    local_created_at: u64,
    task: JoinHandle<()>,
) {
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
        thread_root_event_id,
        runtime_profile_id,
        local_created_at,
        task,
    });
}

fn take_active_stream(room_id: &str, local_created_at: u64) -> Option<ActiveBotStream> {
    let mut active_streams = ACTIVE_STREAMS.lock().unwrap();
    active_streams
        .iter()
        .position(|stream| stream.room_id == room_id && stream.local_created_at == local_created_at)
        .map(|index| active_streams.swap_remove(index))
}

fn take_active_stream_for_scope(
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> Option<ActiveBotStream> {
    let mut active_streams = ACTIVE_STREAMS.lock().unwrap();
    active_streams
        .iter()
        .position(|stream| {
            stream.room_id == room_id
                && stream.thread_root_event_id.as_deref() == thread_root_event_id
        })
        .map(|index| active_streams.swap_remove(index))
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

fn can_start_stream(room_id: &str, runtime_profile_id: &str, dispatch_policy: &DispatchPolicy) -> bool {
    let active_streams = ACTIVE_STREAMS.lock().unwrap();
    let room_count = active_streams
        .iter()
        .filter(|stream| stream.room_id == room_id)
        .count();
    let runtime_count = active_streams
        .iter()
        .filter(|stream| stream.runtime_profile_id == runtime_profile_id)
        .count();
    room_count < dispatch_policy.max_parallel_per_room
        && runtime_count < dispatch_policy.max_parallel_per_runtime
}

fn release_stream_slot(room_id: &str, local_created_at: u64) {
    clear_active_stream(room_id, local_created_at);
    drain_stream_queue();
}

fn drain_stream_queue() {
    loop {
        let maybe_prepared = {
            let active_streams = ACTIVE_STREAMS.lock().unwrap();
            let mut queued_streams = QUEUED_STREAMS.lock().unwrap();
            let next_index = queued_streams.iter().position(|queued| {
                let room_count = active_streams
                    .iter()
                    .filter(|stream| stream.room_id == queued.room_id)
                    .count();
                let runtime_count = active_streams
                    .iter()
                    .filter(|stream| stream.runtime_profile_id == queued.runtime_profile_id)
                    .count();
                room_count < queued.dispatch_policy.max_parallel_per_room
                    && runtime_count < queued.dispatch_policy.max_parallel_per_runtime
            });
            next_index.and_then(|index| queued_streams.remove(index))
        };

        let Some(queued) = maybe_prepared else {
            break;
        };

        start_prepared_stream(
            PreparedBotStream {
                room_id: queued.room_id,
                thread_root_event_id: queued.thread_root_event_id,
                reply_root_event_id: queued.reply_root_event_id,
                runtime_profile_id: queued.runtime_profile_id,
                runtime_kind: queued.runtime_kind,
                profile_name: queued.profile_name,
                dispatch_policy: queued.dispatch_policy,
                runtime: queued.runtime,
                request: queued.request,
            },
            queued.local_created_at,
        );
    }
}

fn take_queued_stream(room_id: &str, local_created_at: u64) -> Option<QueuedBotStream> {
    let mut queued_streams = QUEUED_STREAMS.lock().unwrap();
    queued_streams
        .iter()
        .position(|stream| stream.room_id == room_id && stream.local_created_at == local_created_at)
        .and_then(|index| queued_streams.remove(index))
}

fn has_queued_stream_for_scope(room_id: &str, thread_root_event_id: Option<&str>) -> bool {
    QUEUED_STREAMS.lock().unwrap().iter().any(|stream| {
        stream.room_id == room_id
            && stream.thread_root_event_id.as_deref() == thread_root_event_id
    })
}

fn clear_all_stream_queues() {
    QUEUED_STREAMS.lock().unwrap().clear();
}

fn clear_all_stream_previews() {
    STREAM_PREVIEWS.lock().unwrap().clear();
}

fn clear_all_direct_stream_messages() {
    DIRECT_STREAM_MESSAGES.lock().unwrap().clear();
}

fn update_stream_preview(
    room_id: &str,
    thread_root_event_id: Option<&str>,
    runtime_kind: RuntimeKind,
    bot_name: &str,
    status: BotStreamPreviewStatus,
    detail: String,
    text: String,
    can_post: bool,
) {
    let mut previews = STREAM_PREVIEWS.lock().unwrap();
    if let Some(preview) = previews.iter_mut().find(|preview| {
        preview.room_id == room_id && preview.thread_root_event_id.as_deref() == thread_root_event_id
    }) {
        preview.runtime_kind = runtime_kind;
        preview.bot_name = bot_name.to_string();
        preview.status = status;
        preview.detail = detail;
        preview.text = text;
        preview.can_post = can_post;
        return;
    }

    previews.push(StreamPreviewState {
        room_id: room_id.to_string(),
        thread_root_event_id: thread_root_event_id.map(ToOwned::to_owned),
        runtime_kind,
        bot_name: bot_name.to_string(),
        status,
        text,
        detail,
        can_post,
    });
}

fn append_stream_preview(room_id: &str, thread_root_event_id: Option<&str>, delta: &str) {
    let mut previews = STREAM_PREVIEWS.lock().unwrap();
    let Some(preview) = previews.iter_mut().find(|preview| {
        preview.room_id == room_id && preview.thread_root_event_id.as_deref() == thread_root_event_id
    }) else {
        return;
    };
    preview.status = BotStreamPreviewStatus::Streaming;
    preview.detail = "Streaming bot response...".into();
    preview.text.push_str(delta);
}

fn mark_stream_finished(room_id: &str, thread_root_event_id: Option<&str>, text: &str) {
    let mut previews = STREAM_PREVIEWS.lock().unwrap();
    let Some(preview) = previews.iter_mut().find(|preview| {
        preview.room_id == room_id && preview.thread_root_event_id.as_deref() == thread_root_event_id
    }) else {
        return;
    };
    preview.status = BotStreamPreviewStatus::Finished;
    preview.text = text.to_string();
    preview.detail = "Bot stream finished. Review and post when ready.".into();
    preview.can_post = !text.trim().is_empty();
}

fn mark_stream_failed(room_id: &str, thread_root_event_id: Option<&str>, error: &str) {
    let mut previews = STREAM_PREVIEWS.lock().unwrap();
    let Some(preview) = previews.iter_mut().find(|preview| {
        preview.room_id == room_id && preview.thread_root_event_id.as_deref() == thread_root_event_id
    }) else {
        return;
    };
    preview.status = BotStreamPreviewStatus::Failed;
    preview.detail = error.to_string();
    preview.can_post = false;
}

fn mark_stream_cancelled(room_id: &str, thread_root_event_id: Option<&str>) {
    let mut previews = STREAM_PREVIEWS.lock().unwrap();
    let Some(preview) = previews.iter_mut().find(|preview| {
        preview.room_id == room_id && preview.thread_root_event_id.as_deref() == thread_root_event_id
    }) else {
        return;
    };
    preview.status = BotStreamPreviewStatus::Cancelled;
    preview.detail = "Bot stream cancelled.".into();
    preview.can_post = false;
}

fn remove_stream_preview(
    room_id: &str,
    thread_root_event_id: Option<&str>,
) -> Option<BotStreamPreviewSnapshot> {
    let mut previews = STREAM_PREVIEWS.lock().unwrap();
    let index = previews.iter().position(|preview| {
        preview.room_id == room_id && preview.thread_root_event_id.as_deref() == thread_root_event_id
    })?;
    let preview = previews.swap_remove(index);
    Some(BotStreamPreviewSnapshot {
        room_id: preview.room_id,
        thread_root_event_id: preview.thread_root_event_id,
        runtime_kind: Some(preview.runtime_kind),
        bot_name: preview.bot_name,
        status: preview.status,
        text: preview.text,
        detail: preview.detail,
        can_post: preview.can_post,
    })
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

fn bot_override_summary(runtime_override: &BotRuntimeOverride) -> String {
    let mut parts = Vec::new();
    if let Some(model) = runtime_override.model.as_deref() {
        parts.push(format!("model={model}"));
    }
    if runtime_override.system_prompt.is_some() {
        parts.push("prompt=custom".into());
    }
    if let Some(agent_id) = runtime_override.agent_id.as_deref() {
        parts.push(format!("agent={agent_id}"));
    }
    if parts.is_empty() {
        "(none)".into()
    } else {
        parts.join(", ")
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

fn set_bot_override(
    bot_selector: &str,
    field_name: &str,
    raw_value: &str,
    mut apply: impl FnMut(&mut BotDefinition, &RuntimeProfile, Option<String>) -> Result<String, String>,
) -> Result<String, String> {
    ensure_loaded_for_current_user()?;
    let selector = bot_selector.trim();
    if selector.is_empty() {
        return Err(format!("Usage: /bot set-{field_name} <bot-id> <value>"));
    }

    let value = normalize_override_value(raw_value);
    let message = with_context_mut(|ctx| {
        let bot_id = resolve_bot_id_selector(&ctx.state, selector)
            .ok_or_else(|| format!("Bot selector `{selector}` did not match any bot."))?;
        let runtime_profile_id = ctx
            .state
            .bots
            .get(&bot_id)
            .ok_or_else(|| format!("Bot {bot_id} is missing from the state file."))?
            .runtime_profile_id
            .clone();
        let profile = ctx
            .state
            .runtime_profiles
            .get(&runtime_profile_id)
            .cloned()
            .ok_or_else(|| format!("Runtime profile {runtime_profile_id} is missing."))?;
        let bot = ctx
            .state
            .bots
            .get_mut(&bot_id)
            .ok_or_else(|| format!("Bot {bot_id} is missing from the state file."))?;
        let message = apply(bot, &profile, value)?;
        ctx.store
            .save(&ctx.state)
            .map_err(|error| error.to_string())?;
        Ok(message)
    })?;

    Cx::post_action(BotfatherAction::StateChanged);
    Ok(message)
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

fn normalize_override_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "-" | "default" | "inherit" | "none" => None,
        _ => Some(trimmed.to_string()),
    }
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
