use ruma::OwnedRoomId;

#[derive(Clone, Debug)]
pub struct RobitScope;

pub fn init_from_config() {}

pub fn init_from_env() {}

pub fn init(_scope: RobitScope) {}

pub fn shutdown() {}

pub fn is_robit_message(_text: &str) -> bool {
    false
}

pub fn mark_room_ready(_room_id: &OwnedRoomId) {}

pub fn room_ready(_room_id: &OwnedRoomId) -> bool {
    false
}

pub fn context_window_size() -> usize {
    0
}

pub fn context_loaded(_room_id: &OwnedRoomId) -> bool {
    false
}

pub fn mark_context_loaded(_room_id: &OwnedRoomId) {}

pub fn submit_message(
    _room_id: &OwnedRoomId,
    _message_id: &str,
    _sender_id: &str,
    _text: &str,
) {
}

pub fn submit_context_message(
    _room_id: &OwnedRoomId,
    _message_id: &str,
    _sender_id: &str,
    _text: &str,
    _role: &str,
) {
}

pub fn strip_robit_prefix(text: &str) -> &str {
    text
}
