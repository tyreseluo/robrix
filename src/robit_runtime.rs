use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use crossbeam_channel::{unbounded, Sender};
use makepad_widgets::log;
use ruma::{OwnedEventId, OwnedRoomId};
use matrix_sdk::room::reply::{EnforceThread, Reply};
use serde_json::json;

use robit::default_registry;
use robit::{
    AiPlanner, AiClient, AiConfig, AiProvider, ConfigMode, Engine, MessagePayload,
    Policy, ProtocolBody, ProtocolEvent, RoomScopeItem, RoomScopePayload, RulePlanner,
    WorkspaceScope,
};
#[cfg(feature = "robit-omnix-mlx")]
use robit::{MlxQwenClient, MlxQwenConfig};

#[derive(Clone, Debug)]
pub struct RobitScope {
    pub workspace_id: String,
    pub rooms: HashSet<String>,
}

const ROBIT_WORKSPACE_ID: &str = "!jiykcEdlcruEoeQPcG:matrix.org";
const ROBIT_ROOM_IDS: &[&str] = &[
    "!KHZpGbrVPoZqDtAkAg:matrix.org",
];
const ROBIT_MESSAGE_PREFIX: &str = "[Robit] ";
const ROBIT_MESSAGE_PREFIX_LEGACY: &str = "[Robit-LEGACY] ";
const ROBIT_CONTEXT_WINDOW: usize = 50;
// const ROBIT_AI_BACKEND: &str = "http";
const ROBIT_AI_BACKEND: &str = "omnix-mlx";
const ROBIT_AI_PROVIDER: &str = "deepseek";
const ROBIT_AI_MODEL: &str = "deepseek-chat";
const ROBIT_AI_KEY: &str = "YOUR_KEY";
const ROBIT_AI_BASE_URL: Option<&str> = None;
const ROBIT_AI_TEMPERATURE: f64 = 0.2;
#[cfg(feature = "robit-omnix-mlx")]
const ROBIT_MLX_MODEL_DIR: &str = "/Users/tyreseluo/Projects/OminiX-MLX/models/Qwen3-4B";
#[cfg(feature = "robit-omnix-mlx")]
const ROBIT_MLX_TEMPERATURE: f32 = 0.2;
#[cfg(feature = "robit-omnix-mlx")]
const ROBIT_MLX_MAX_TOKENS: usize = 128;

pub fn is_robit_message(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with(ROBIT_MESSAGE_PREFIX) || trimmed.starts_with(ROBIT_MESSAGE_PREFIX_LEGACY)
}

impl RobitScope {
    pub fn from_config() -> Option<Self> {
        let rooms = ROBIT_ROOM_IDS
            .iter()
            .map(|room| room.trim().to_string())
            .filter(|room| !room.is_empty())
            .collect::<HashSet<_>>();
        if rooms.is_empty() {
            return None;
        }
        Some(Self {
            workspace_id: ROBIT_WORKSPACE_ID.to_string(),
            rooms,
        })
    }
}

fn build_ai_client() -> Option<AiClient> {
    let model = ROBIT_AI_MODEL.trim();
    if model.is_empty() {
        log!("Robit AI disabled: ROBIT_AI_MODEL is empty.");
        return None;
    }
    let key = ROBIT_AI_KEY.trim();
    if key.is_empty() {
        log!("Robit AI disabled: ROBIT_AI_KEY is empty.");
        return None;
    }
    let provider_value = ROBIT_AI_PROVIDER.trim().to_lowercase();
    let provider = match provider_value.as_str() {
        "openai" | "chatgpt" => AiProvider::OpenAI,
        "deepseek" => AiProvider::DeepSeek,
        other => {
            log!("Robit AI disabled: unknown provider '{other}'.");
            return None;
        }
    };
    let config = AiConfig {
        provider,
        api_key: key.to_string(),
        model: model.to_string(),
        base_url: ROBIT_AI_BASE_URL.map(|value| value.to_string()),
        temperature: Some(ROBIT_AI_TEMPERATURE),
    };
    match AiClient::new(config) {
        Ok(client) => {
            log!(
                "Robit AI enabled: provider={}, model={}",
                provider_value,
                model
            );
            Some(client)
        }
        Err(err) => {
            log!("Robit AI disabled: {err}");
            None
        }
    }
}

#[cfg(feature = "robit-omnix-mlx")]
fn build_mlx_backend() -> Option<(Arc<dyn AiPlanner>, String)> {
    let model_dir = ROBIT_MLX_MODEL_DIR.trim();
    if model_dir.is_empty() {
        log!("Robit MLX disabled: ROBIT_MLX_MODEL_DIR is empty.");
        return None;
    }
    let config = MlxQwenConfig {
        model_dir: model_dir.into(),
        temperature: ROBIT_MLX_TEMPERATURE,
        max_tokens: ROBIT_MLX_MAX_TOKENS,
    };
    match MlxQwenClient::new(config) {
        Ok(client) => {
            log!("Robit MLX enabled: model_dir={}", model_dir);
            Some((Arc::new(client), format!("omnix-mlx:qwen3@{model_dir}")))
        }
        Err(err) => {
            log!("Robit MLX disabled: {err:?}");
            None
        }
    }
}

#[cfg(not(feature = "robit-omnix-mlx"))]
fn build_mlx_backend() -> Option<(Arc<dyn AiPlanner>, String)> {
    None
}

fn configure_ai(engine: &mut Engine) {
    match ROBIT_AI_BACKEND.trim().to_lowercase().as_str() {
        "omnix-mlx" | "omnix" | "mlx" => {
            let backend = build_mlx_backend();
            if let Some((client, label)) = backend {
                engine.set_ai_backend_with_label(Some(client), Some(label));
            } else {
                log!("Robit MLX backend not available; staying disabled.");
                engine.set_ai_backend_with_label(None, Some("omnix-mlx:unavailable".to_string()));
            }
        }
        _ => {
            engine.set_ai_client(build_ai_client());
        }
    }
}

struct RobitRuntime {
    sender: Sender<ProtocolEvent>,
    scope: RobitScope,
    ready_rooms: Mutex<HashSet<String>>,
    context_loaded_rooms: Mutex<HashSet<String>>,
}

static ROBIT_RUNTIME: OnceLock<RobitRuntime> = OnceLock::new();

pub fn init_from_config() {
    let Some(scope) = RobitScope::from_config() else {
        log!("Robit runtime not initialized: configure ROBIT_ROOM_IDS in src/robit_runtime.rs.");
        return;
    };
    if ROBIT_RUNTIME.get().is_some() {
        return;
    }
    init(scope);
}

pub fn init(scope: RobitScope) {
    if ROBIT_RUNTIME.get().is_some() {
        return;
    }
    let (sender, receiver) = unbounded();
    let scope_for_thread = scope.clone();
    thread::spawn(move || {
        let registry = default_registry();
        let planner = RulePlanner::new();
        let policy = Policy::default_with_home();
        let mut engine = match Engine::new(registry, planner, policy) {
            Ok(engine) => engine,
            Err(err) => {
                log!("Failed to start Robit engine: {err}");
                return;
            }
        };
        configure_ai(&mut engine);

        log!(
            "Robit engine started. workspace_id={}, rooms={:?}",
            scope_for_thread.workspace_id,
            scope_for_thread.rooms
        );
        let scope_event = build_scope_event(&scope_for_thread);
        let _ = engine.handle_protocol_event(scope_event);

        while let Ok(event) = receiver.recv() {
            let responses = engine.handle_protocol_event(event);
            for response in responses {
                dispatch_response(response);
            }
        }
    });

    let runtime = RobitRuntime {
        sender,
        scope,
        ready_rooms: Mutex::new(HashSet::new()),
        context_loaded_rooms: Mutex::new(HashSet::new()),
    };
    let _ = ROBIT_RUNTIME.set(runtime);
}

pub fn shutdown() {
    // For now we rely on process exit; channel closure will stop the worker loop.
}

pub fn mark_room_ready(room_id: &OwnedRoomId) {
    let Some(runtime) = ROBIT_RUNTIME.get() else {
        return;
    };
    let mut ready = runtime.ready_rooms.lock().unwrap();
    if ready.insert(room_id.to_string()) {
        log!("Robit room ready: {}", room_id);
    }
}

pub fn room_ready(room_id: &OwnedRoomId) -> bool {
    let Some(runtime) = ROBIT_RUNTIME.get() else {
        return false;
    };
    let ready = runtime.ready_rooms.lock().unwrap();
    ready.contains(room_id.as_str())
}

pub fn context_window_size() -> usize {
    ROBIT_CONTEXT_WINDOW
}

pub fn context_loaded(room_id: &OwnedRoomId) -> bool {
    let Some(runtime) = ROBIT_RUNTIME.get() else {
        return false;
    };
    let loaded = runtime.context_loaded_rooms.lock().unwrap();
    loaded.contains(room_id.as_str())
}

pub fn mark_context_loaded(room_id: &OwnedRoomId) {
    let Some(runtime) = ROBIT_RUNTIME.get() else {
        return;
    };
    let mut loaded = runtime.context_loaded_rooms.lock().unwrap();
    loaded.insert(room_id.to_string());
}

pub fn submit_message(
    room_id: &OwnedRoomId,
    message_id: &str,
    sender_id: &str,
    text: &str,
) {
    let Some(runtime) = ROBIT_RUNTIME.get() else {
        log!("Robit runtime not ready; ignoring message {message_id}.");
        return;
    };
    let room_id_str = room_id.as_str();
    if !runtime.scope.rooms.contains(room_id_str) {
        log!(
            "Robit ignored message {message_id}: room not in scope ({room_id_str})."
        );
        return;
    }
    if is_robit_message(text) {
        log!("Robit ignored message {message_id}: already tagged.");
        return;
    }
    let payload = MessagePayload {
        message_id: message_id.to_string(),
        room_id: room_id_str.to_string(),
        workspace_id: runtime.scope.workspace_id.clone(),
        sender_id: sender_id.to_string(),
        text: text.to_string(),
        event_kind: Some("text".to_string()),
        metadata: json!({"source": "robrix"}),
    };
    let event = ProtocolEvent {
        schema_version: "robit.v1".to_string(),
        id: format!("msg-{message_id}"),
        timestamp: None,
        body: ProtocolBody::Message(payload),
    };
    log!(
        "Robit submit: room={}, sender={}, msg_id={}, text={:?}",
        room_id_str,
        sender_id,
        message_id,
        text
    );
    let _ = runtime.sender.send(event);
}

pub fn submit_context_message(
    room_id: &OwnedRoomId,
    message_id: &str,
    sender_id: &str,
    text: &str,
    role: &str,
) {
    let Some(runtime) = ROBIT_RUNTIME.get() else {
        return;
    };
    let room_id_str = room_id.as_str();
    if !runtime.scope.rooms.contains(room_id_str) {
        return;
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let payload = MessagePayload {
        message_id: format!("ctx-{message_id}"),
        room_id: room_id_str.to_string(),
        workspace_id: runtime.scope.workspace_id.clone(),
        sender_id: sender_id.to_string(),
        text: trimmed.to_string(),
        event_kind: Some("text".to_string()),
        metadata: json!({"source": "robrix", "context_only": true, "role": role}),
    };
    let event = ProtocolEvent {
        schema_version: "robit.v1".to_string(),
        id: format!("ctx-msg-{message_id}"),
        timestamp: None,
        body: ProtocolBody::Message(payload),
    };
    let _ = runtime.sender.send(event);
}

pub fn strip_robit_prefix(text: &str) -> &str {
    let trimmed = text.trim_start();
    let rest = if let Some(rest) = trimmed.strip_prefix(ROBIT_MESSAGE_PREFIX) {
        rest
    } else if let Some(rest) = trimmed.strip_prefix(ROBIT_MESSAGE_PREFIX_LEGACY) {
        rest
    } else {
        return trimmed;
    };
    let rest = rest.trim_start();
    if rest.starts_with('[') {
        if let Some(end) = rest.find(']') {
            return rest[end + 1..].trim_start();
        }
    }
    rest
}

fn build_scope_event(scope: &RobitScope) -> ProtocolEvent {
    let rooms = scope
        .rooms
        .iter()
        .map(|room_id| RoomScopeItem {
            room_id: room_id.clone(),
            name: None,
        })
        .collect();
    ProtocolEvent {
        schema_version: "robit.v1".to_string(),
        id: "scope-boot".to_string(),
        timestamp: None,
        body: ProtocolBody::RoomScope(RoomScopePayload {
            mode: Some(ConfigMode::Replace),
            workspaces: vec![WorkspaceScope {
                workspace_id: scope.workspace_id.clone(),
                name: None,
                rooms,
            }],
        }),
    }
}

fn dispatch_response(event: ProtocolEvent) {
    let ProtocolBody::Response(payload) = event.body else {
        return;
    };
    let Ok(room_id) = payload.room_id.parse::<OwnedRoomId>() else {
        log!("Robit response had invalid room_id: {}", payload.room_id);
        return;
    };
    let kind_tag = match payload.kind.as_str() {
        "approval_request" => "[approval] ",
        "action_result" => "[result] ",
        "error" => "[error] ",
        "need_input" => "[need] ",
        _ => "",
    };
    let base_text = if payload.text.starts_with(ROBIT_MESSAGE_PREFIX) {
        payload.text
    } else {
        format!("{ROBIT_MESSAGE_PREFIX}{kind_tag}{}", payload.text)
    };
    log!(
        "Robit response: room={}, kind={}, text={:?}",
        payload.room_id,
        payload.kind,
        base_text
    );
    let message = ruma::events::room::message::RoomMessageEventContent::text_plain(base_text);
    let replied_to = payload
        .in_reply_to
        .parse::<OwnedEventId>()
        .ok()
        .map(|event_id| Reply {
            event_id,
            enforce_thread: EnforceThread::MaybeThreaded,
        });
    crate::sliding_sync::submit_async_request(crate::sliding_sync::MatrixRequest::SendMessage {
        room_id,
        message,
        replied_to,
        #[cfg(feature = "tsp")]
        sign_with_tsp: false,
    });
}
