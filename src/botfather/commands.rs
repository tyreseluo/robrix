use crate::shared::popup_list::PopupKind;

use super::{
    bind_room_to_bot, bots_overview, create_bot, run_bot_healthcheck, run_room_healthcheck,
    runtime_summary, set_bot_runtime_profile, status_overview, unbind_room, workspace_overview,
};
use robrix_botfather::RuntimeKind;

#[derive(Clone, Debug)]
pub struct CommandFeedback {
    pub message: String,
    pub kind: PopupKind,
    pub auto_dismissal_duration: Option<f64>,
    pub clear_input: bool,
}

#[derive(Clone, Debug)]
pub enum CommandHandling {
    NotACommand,
    Consumed(CommandFeedback),
}

pub fn help_text() -> &'static str {
    "/bot help\n\
/bot status\n\
/bot bots\n\
/bot runtimes\n\
/bot bind <bot>\n\
/bot use <bot>\n\
/bot unbind\n\
/bot create <bot> [profile]\n\
/bot set-profile <bot> <profile>\n\
/bot health [bot]"
}

pub fn handle_room_command(room_id: &str, input: &str) -> CommandHandling {
    let trimmed = input.trim();
    if !trimmed.starts_with("/bot") {
        return CommandHandling::NotACommand;
    }

    let mut parts = trimmed.split_whitespace();
    let _root = parts.next();
    let subcommand = parts
        .next()
        .map(|command| command.trim_start_matches('/').to_ascii_lowercase());

    let outcome = match subcommand.as_deref() {
        None | Some("") | Some("help") => ok_feedback(help_text(), PopupKind::Info, Some(10.0)),
        Some("status") => ok_feedback(status_overview(Some(room_id)), PopupKind::Info, Some(8.0)),
        Some("bots") => ok_feedback(bots_overview(), PopupKind::Info, Some(10.0)),
        Some("runtimes") => ok_feedback(
            format!(
                "crew\n{}\n\nopenclaw\n{}\n\nworkspace\n{}",
                runtime_summary(RuntimeKind::Crew),
                runtime_summary(RuntimeKind::OpenClaw),
                workspace_overview(),
            ),
            PopupKind::Info,
            Some(12.0),
        ),
        Some("bind") | Some("use") => match parts.next() {
            Some(bot_selector) => result_feedback(
                bind_room_to_bot(room_id, bot_selector),
                PopupKind::Success,
                Some(5.0),
                true,
            ),
            None => err_feedback("Usage: /bot bind <bot-id>"),
        },
        Some("unbind") => result_feedback(
            unbind_room(room_id).map(|()| "Removed the room-level bot override.".to_string()),
            PopupKind::Success,
            Some(5.0),
            true,
        ),
        Some("create") => match parts.next() {
            Some(bot_id) => result_feedback(
                create_bot(bot_id, parts.next(), Some(room_id)),
                PopupKind::Success,
                Some(6.0),
                true,
            ),
            None => err_feedback("Usage: /bot create <bot-id> [profile-id]"),
        },
        Some("set-profile") => match (parts.next(), parts.next()) {
            (Some(bot_id), Some(profile_id)) => result_feedback(
                set_bot_runtime_profile(bot_id, profile_id),
                PopupKind::Success,
                Some(6.0),
                true,
            ),
            _ => err_feedback("Usage: /bot set-profile <bot-id> <profile-id>"),
        },
        Some("health") => {
            let result = match parts.next() {
                Some(bot_selector) => run_bot_healthcheck(bot_selector.to_string())
                    .map(|()| format!("Running healthcheck for `{bot_selector}`...")),
                None => run_room_healthcheck(room_id.to_string())
                    .map(|()| "Running healthcheck for this room's main bot...".to_string()),
            };
            result_feedback(result, PopupKind::Info, Some(5.0), true)
        }
        Some("set-model") => err_feedback(
            "Bot-level model override is not available yet. Use `/bot set-profile` for now.",
        ),
        Some(other) => err_feedback(format!("Unknown bot command `{other}`.\nTry `/bot help`.")),
    };

    CommandHandling::Consumed(outcome)
}

fn ok_feedback(
    message: impl Into<String>,
    kind: PopupKind,
    auto_dismissal_duration: Option<f64>,
) -> CommandFeedback {
    CommandFeedback {
        message: message.into(),
        kind,
        auto_dismissal_duration,
        clear_input: true,
    }
}

fn err_feedback(message: impl Into<String>) -> CommandFeedback {
    CommandFeedback {
        message: message.into(),
        kind: PopupKind::Error,
        auto_dismissal_duration: Some(6.0),
        clear_input: false,
    }
}

fn result_feedback(
    result: Result<String, String>,
    success_kind: PopupKind,
    auto_dismissal_duration: Option<f64>,
    clear_input: bool,
) -> CommandFeedback {
    match result {
        Ok(message) => CommandFeedback {
            message,
            kind: success_kind,
            auto_dismissal_duration,
            clear_input,
        },
        Err(error) => CommandFeedback {
            message: error,
            kind: PopupKind::Error,
            auto_dismissal_duration: Some(6.0),
            clear_input: false,
        },
    }
}
