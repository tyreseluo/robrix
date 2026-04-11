//! The RoomInputBar widget contains all components related to sending messages/content to a room.
//!
//! The RoomInputBar is capped to a maximum height of 75% of the containing RoomScreen's height.
//!
//! The widgets included in the RoomInputBar are:
//! * a preview of the message the user is replying to.
//! * the location preview (which allows you to send your current location to the room),
//!   and a location card to show the location preview.
//! * If TSP is enabled, a checkbox to enable TSP signing for the outgoing message.
//! * A MentionableTextInput, which allows the user to type a message
//!   and mention other users via the `@` key.
//! * A button to send the message.
//! * The editing pane, which is shown when the user is editing a previous message.
//! * A tombstone footer, which is shown if the room has been tombstoned (replaced).
//! * A "cannot-send-message" notice, which is shown if the user cannot send messages to the room.
//!


use makepad_widgets::*;
use matrix_sdk::room::reply::{EnforceThread, Reply};
use ruma::events::room::message::AddMentions;
use matrix_sdk_ui::timeline::{EmbeddedEvent, EventTimelineItem, TimelineEventItemId};
use ruma::{events::room::message::{LocationMessageEventContent, MessageType, ReplyWithinThread, RoomMessageEventContent}, OwnedRoomId, OwnedUserId, UserId};
use crate::{app::AppState, home::{editing_pane::{EditingPaneState, EditingPaneWidgetExt, EditingPaneWidgetRefExt}, location_preview::{LocationPreviewWidgetExt, LocationPreviewWidgetRefExt}, room_screen::{MessageAction, RoomScreenProps, is_known_or_likely_bot, populate_preview_of_timeline_item}, tombstone_footer::{SuccessorRoomDetails, TombstoneFooterWidgetExt}, upload_progress::UploadProgressViewWidgetRefExt}, i18n::{AppLanguage, tr_fmt, tr_key}, location::init_location_subscriber, room::translation::{self, TRANSLATION_REQUEST_ID}, shared::{avatar::AvatarWidgetRefExt, file_upload_modal::{FileData, FileLoadedData, FilePreviewerAction}, html_or_plaintext::HtmlOrPlaintextWidgetRefExt, mentionable_text_input::MentionableTextInputWidgetExt, popup_list::{PopupKind, enqueue_popup_notification}, styles::*}, sliding_sync::{MatrixRequest, TimelineKind, UserPowerLevels, submit_async_request}, utils};
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use crate::shared::file_upload_modal::{FilePreviewerMetaData, ThumbnailData};

const ROOM_INFO_CARD_MOBILE_BREAKPOINT: f32 = 700.0;
#[cfg(test)]
const TRANSLATION_LANG_POPUP_WIDTH: f64 = 220.0;
#[cfg(test)]
const TRANSLATION_LANG_POPUP_SCROLL_HEIGHT: f64 = 288.0;
#[cfg(test)]
const TRANSLATION_LANG_POPUP_HEIGHT: f64 = TRANSLATION_LANG_POPUP_SCROLL_HEIGHT + 8.0;
#[cfg(test)]
const TRANSLATION_LANG_POPUP_GAP: f64 = 6.0;
#[cfg(test)]
const TRANSLATION_LANG_POPUP_MARGIN: f64 = 8.0;

#[cfg(test)]
fn compute_translation_popup_abs_pos(
    button_rect_local: Rect,
    container_rect_screen: Rect,
    pass_size: DVec2,
) -> DVec2 {
    let max_x = (container_rect_screen.size.x - TRANSLATION_LANG_POPUP_WIDTH - TRANSLATION_LANG_POPUP_MARGIN)
        .max(TRANSLATION_LANG_POPUP_MARGIN);
    let popup_x = (button_rect_local.pos.x + button_rect_local.size.x - TRANSLATION_LANG_POPUP_WIDTH)
        .max(TRANSLATION_LANG_POPUP_MARGIN)
        .min(max_x);

    let max_y = (pass_size.y - TRANSLATION_LANG_POPUP_HEIGHT - TRANSLATION_LANG_POPUP_MARGIN)
        .max(TRANSLATION_LANG_POPUP_MARGIN);
    let popup_y_above = button_rect_local.pos.y - TRANSLATION_LANG_POPUP_HEIGHT - TRANSLATION_LANG_POPUP_GAP;
    let button_screen_y = container_rect_screen.pos.y + button_rect_local.pos.y;
    let popup_y = if button_screen_y >= TRANSLATION_LANG_POPUP_HEIGHT + TRANSLATION_LANG_POPUP_GAP + TRANSLATION_LANG_POPUP_MARGIN {
        popup_y_above
    } else {
        let popup_y_screen = (button_screen_y + button_rect_local.size.y + TRANSLATION_LANG_POPUP_GAP)
            .max(TRANSLATION_LANG_POPUP_MARGIN)
            .min(max_y);
        popup_y_screen - container_rect_screen.pos.y
    };

    dvec2(popup_x, popup_y)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TranslationApplyOutcome {
    input_text: String,
    preserved_preview_text: String,
    next_last_source: String,
    keep_preview_visible: bool,
}

fn compute_translation_apply_outcome(translated_text: &str) -> TranslationApplyOutcome {
    let applied_text = translated_text.to_string();
    TranslationApplyOutcome {
        input_text: applied_text.clone(),
        preserved_preview_text: applied_text.clone(),
        next_last_source: applied_text,
        keep_preview_visible: true,
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum ExplicitOverride {
    #[default]
    None,
    Bot(OwnedUserId),
    Room,
}

impl ExplicitOverride {
    fn cleared(&self) -> Self {
        Self::None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ResolvedTarget {
    NoTarget,
    RoomDefault(OwnedUserId),
    ExplicitBot(OwnedUserId),
    ExplicitRoom,
    ReplyBot(OwnedUserId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TargetChipPresentation {
    visible: bool,
    label: String,
    subdued: bool,
    dismissible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TargetMenuSelection {
    Room,
    BoundBot,
}

fn known_bot_candidates<'a>(
    bound_bot_user_id: Option<&'a UserId>,
    resolved_parent_bot_user_id: Option<&'a UserId>,
    room_bot_user_ids: &'a [OwnedUserId],
    known_bot_user_ids: &'a [OwnedUserId],
) -> Vec<&'a UserId> {
    let mut candidates = Vec::with_capacity(room_bot_user_ids.len() + known_bot_user_ids.len() + 2);
    if let Some(bound_bot_user_id) = bound_bot_user_id {
        candidates.push(bound_bot_user_id);
    }
    if let Some(resolved_parent_bot_user_id) = resolved_parent_bot_user_id
        && candidates
            .iter()
            .all(|candidate| *candidate != resolved_parent_bot_user_id)
    {
        candidates.push(resolved_parent_bot_user_id);
    }
    for room_bot_user_id in room_bot_user_ids {
        let room_bot_user_id_ref: &UserId = room_bot_user_id.as_ref();
        if candidates
            .iter()
            .all(|candidate| *candidate != room_bot_user_id_ref)
        {
            candidates.push(room_bot_user_id_ref);
        }
    }
    for known_bot_user_id in known_bot_user_ids {
        let known_bot_user_id_ref: &UserId = known_bot_user_id.as_ref();
        if candidates
            .iter()
            .all(|candidate| *candidate != known_bot_user_id_ref)
        {
            candidates.push(known_bot_user_id_ref);
        }
    }
    candidates
}

fn is_matrix_localpart_mention_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '=' | '/' | '+')
}

fn contains_matrix_localpart_mention(text: &str, user_id: &UserId) -> bool {
    let mention = format!("@{}", user_id.localpart());
    for (idx, _) in text.match_indices(&mention) {
        let start_ok = text[..idx]
            .chars()
            .next_back()
            .is_none_or(|c| !is_matrix_localpart_mention_char(c));
        let end_idx = idx + mention.len();
        let end_ok = text[end_idx..]
            .chars()
            .next()
            .is_none_or(|c| !is_matrix_localpart_mention_char(c));
        if start_ok && end_ok {
            return true;
        }
    }
    false
}

fn text_mentions_known_bot(
    text: &str,
    bound_bot_user_id: Option<&UserId>,
    resolved_parent_bot_user_id: Option<&UserId>,
    room_bot_user_ids: &[OwnedUserId],
    known_bot_user_ids: &[OwnedUserId],
) -> bool {
    known_bot_candidates(
        bound_bot_user_id,
        resolved_parent_bot_user_id,
        room_bot_user_ids,
        known_bot_user_ids,
    )
    .into_iter()
    .any(|candidate| {
        text.contains(candidate.as_str()) || contains_matrix_localpart_mention(text, candidate)
    })
}

fn message_mentions_known_bot(
    message: &RoomMessageEventContent,
    bound_bot_user_id: Option<&UserId>,
    resolved_parent_bot_user_id: Option<&UserId>,
    room_bot_user_ids: &[OwnedUserId],
    known_bot_user_ids: &[OwnedUserId],
) -> bool {
    if message.mentions.as_ref().is_some_and(|mentions| {
        mentions.user_ids.iter().any(|user_id| {
            room_bot_user_ids
                .iter()
                .any(|room_bot_user_id| room_bot_user_id.as_str() == user_id.as_str())
                || is_known_or_likely_bot(
                    user_id.as_ref(),
                    resolved_parent_bot_user_id,
                    known_bot_user_ids,
                )
        })
    }) {
        return true;
    }

    if text_mentions_known_bot(
        message.body(),
        bound_bot_user_id,
        resolved_parent_bot_user_id,
        room_bot_user_ids,
        known_bot_user_ids,
    ) {
        return true;
    }

    match &message.msgtype {
        MessageType::Text(content) => content.formatted.as_ref().is_some_and(|formatted| {
            text_mentions_known_bot(
                &formatted.body,
                bound_bot_user_id,
                resolved_parent_bot_user_id,
                room_bot_user_ids,
                known_bot_user_ids,
            )
        }),
        MessageType::Notice(content) => content.formatted.as_ref().is_some_and(|formatted| {
            text_mentions_known_bot(
                &formatted.body,
                bound_bot_user_id,
                resolved_parent_bot_user_id,
                room_bot_user_ids,
                known_bot_user_ids,
            )
        }),
        MessageType::Emote(content) => content.formatted.as_ref().is_some_and(|formatted| {
            text_mentions_known_bot(
                &formatted.body,
                bound_bot_user_id,
                resolved_parent_bot_user_id,
                room_bot_user_ids,
                known_bot_user_ids,
            )
        }),
        _ => false,
    }
}

fn routing_directives_for_message(
    resolved_target: &ResolvedTarget,
    message_mentions_bot: bool,
) -> (Option<OwnedUserId>, bool) {
    let explicit_room = matches!(resolved_target, ResolvedTarget::ExplicitRoom);
    let target_user_id = if message_mentions_bot {
        None
    } else {
        resolved_target_user_id(resolved_target)
    };
    (target_user_id, explicit_room)
}

fn resolve_target(
    explicit_override: &ExplicitOverride,
    replying_to_sender: Option<&UserId>,
    bound_bot_user_id: Option<&UserId>,
    resolved_parent_bot_user_id: Option<&UserId>,
    known_bot_user_ids: &[OwnedUserId],
) -> ResolvedTarget {
    if let Some(replying_to_sender) = replying_to_sender
        && is_known_or_likely_bot(
            replying_to_sender,
            resolved_parent_bot_user_id,
            known_bot_user_ids,
        )
    {
        return ResolvedTarget::ReplyBot(replying_to_sender.to_owned());
    }

    match explicit_override {
        ExplicitOverride::Bot(bot_user_id) => ResolvedTarget::ExplicitBot(bot_user_id.clone()),
        ExplicitOverride::Room => ResolvedTarget::ExplicitRoom,
        ExplicitOverride::None => bound_bot_user_id
            .map(|bot_user_id| ResolvedTarget::RoomDefault(bot_user_id.to_owned()))
            .unwrap_or(ResolvedTarget::NoTarget),
    }
}

fn resolved_target_user_id(target: &ResolvedTarget) -> Option<OwnedUserId> {
    match target {
        ResolvedTarget::NoTarget | ResolvedTarget::ExplicitRoom => None,
        ResolvedTarget::RoomDefault(bot_user_id)
        | ResolvedTarget::ExplicitBot(bot_user_id)
        | ResolvedTarget::ReplyBot(bot_user_id) => Some(bot_user_id.clone()),
    }
}

#[cfg(test)]
fn clear_explicit_override_result(bound_bot_user_id: Option<&UserId>) -> ResolvedTarget {
    bound_bot_user_id
        .map(|bot_user_id| ResolvedTarget::RoomDefault(bot_user_id.to_owned()))
        .unwrap_or(ResolvedTarget::NoTarget)
}

fn format_target_chip_presentation(
    app_language: AppLanguage,
    resolved_target: &ResolvedTarget,
    bot_display_name: Option<&str>,
) -> TargetChipPresentation {
    let target_label = match resolved_target {
        ResolvedTarget::RoomDefault(bot_user_id)
        | ResolvedTarget::ExplicitBot(bot_user_id)
        | ResolvedTarget::ReplyBot(bot_user_id) => bot_display_name
            .map(str::trim)
            .filter(|display_name| !display_name.is_empty())
            .unwrap_or_else(|| bot_user_id.localpart())
            .to_string(),
        ResolvedTarget::NoTarget | ResolvedTarget::ExplicitRoom => String::new(),
    };

    match resolved_target {
        ResolvedTarget::NoTarget => TargetChipPresentation {
            visible: false,
            label: String::new(),
            subdued: false,
            dismissible: false,
        },
        ResolvedTarget::RoomDefault(_) => TargetChipPresentation {
            visible: true,
            label: tr_fmt(
                app_language,
                "room_input_bar.target.default",
                &[("display_name", &target_label)],
            ),
            subdued: true,
            dismissible: false,
        },
        ResolvedTarget::ExplicitBot(_) => TargetChipPresentation {
            visible: true,
            label: tr_fmt(
                app_language,
                "room_input_bar.target.to_bot",
                &[("display_name", &target_label)],
            ),
            subdued: false,
            dismissible: true,
        },
        ResolvedTarget::ExplicitRoom => TargetChipPresentation {
            visible: true,
            label: tr_key(app_language, "room_input_bar.target.to_room").to_string(),
            subdued: false,
            dismissible: true,
        },
        ResolvedTarget::ReplyBot(_) => TargetChipPresentation {
            visible: true,
            label: tr_fmt(
                app_language,
                "room_input_bar.target.reply_bot",
                &[("display_name", &target_label)],
            ),
            subdued: false,
            dismissible: false,
        },
    }
}

fn apply_target_menu_selection(
    selection: TargetMenuSelection,
    bound_bot_user_id: Option<&UserId>,
) -> ExplicitOverride {
    match selection {
        TargetMenuSelection::Room => ExplicitOverride::Room,
        TargetMenuSelection::BoundBot => bound_bot_user_id
            .map(|bot_user_id| ExplicitOverride::Bot(bot_user_id.to_owned()))
            .unwrap_or_default(),
    }
}

fn resolve_send_target(
    explicit_override: &ExplicitOverride,
    replying_to_sender: Option<&UserId>,
    bound_bot_user_id: Option<&UserId>,
    resolved_parent_bot_user_id: Option<&UserId>,
    known_bot_user_ids: &[OwnedUserId],
) -> ResolvedTarget {
    resolve_target(
        explicit_override,
        replying_to_sender,
        bound_bot_user_id,
        resolved_parent_bot_user_id,
        known_bot_user_ids,
    )
}

#[cfg(test)]
fn resolve_restored_target(
    explicit_override: &ExplicitOverride,
    replying_to_sender: Option<&UserId>,
    bound_bot_user_id: Option<&UserId>,
    resolved_parent_bot_user_id: Option<&UserId>,
    known_bot_user_ids: &[OwnedUserId],
) -> ResolvedTarget {
    resolve_target(
        explicit_override,
        replying_to_sender,
        bound_bot_user_id,
        resolved_parent_bot_user_id,
        known_bot_user_ids,
    )
}

fn restored_explicit_override(saved_state: &RoomInputBarState) -> ExplicitOverride {
    saved_state.explicit_override.clone()
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    mod.widgets.ICO_LOCATION_PERSON = crate_resource("self://resources/icons/location-person.svg")
    mod.widgets.ICO_MENU = crate_resource("self://resources/icons/menu.svg")
    mod.widgets.ICO_THREADS = crate_resource("self://resources/icons/double_chat.svg")
    mod.widgets.ICO_TRANSLATE = crate_resource("self://resources/icons/translate.svg")

    mod.widgets.TranslationLangItem = View {
        width: Fill, height: 36
        flow: Right
        align: Align{y: 0.5}
        padding: Inset{left: 12, right: 12}
        cursor: MouseCursor.Hand
        show_bg: true
        draw_bg +: {
            color: #0000
            hover: instance(0.0)
            pixel: fn() {
                return mix(self.color, #xF0F4FA, self.hover)
            }
        }
        animator: Animator {
            hover: {
                default: @off
                off: AnimatorState {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: { hover: 0.0 } }
                }
                on: AnimatorState {
                    from: {all: Forward {duration: 0.15}}
                    apply: { draw_bg: { hover: 1.0 } }
                }
            }
        }

        RoundedView {
            width: Fit, height: Fit
            padding: Inset{left: 5, right: 5, top: 2, bottom: 2}
            margin: Inset{right: 10}
            show_bg: true
            draw_bg +: {
                color: #xE8EEF8
                border_radius: 3.0
            }
            lang_code := Label {
                width: Fit, height: Fit
                draw_text +: {
                    color: #x555555
                    text_style: REGULAR_TEXT { font_size: 9 }
                }
                text: "en"
            }
        }

        lang_name := Label {
            width: Fill, height: Fit
            draw_text +: {
                color: #x333333
                text_style: REGULAR_TEXT { font_size: 11 }
            }
            text: "English"
        }
    }

    mod.widgets.RoomEmojiButton = mod.widgets.RobrixIconButton {
        spacing: 0
        text: ""
        margin: 0
        padding: Inset{left: 8, right: 8, top: 6, bottom: 6}
        icon_walk: Walk{width: 0, height: 0}
        draw_text +: {
            color: (COLOR_TEXT)
            color_hover: (COLOR_TEXT)
            color_down: (COLOR_TEXT)
            text_style: MESSAGE_TEXT_STYLE { font_size: 15.0 }
        }
        draw_bg +: {
            color: (COLOR_PRIMARY)
            color_hover: #F4F7FC
            color_down: #E8EEF8
            border_size: 1.0
            border_color: (COLOR_SECONDARY)
        }
    }

    mod.widgets.TargetChipButton = Button {
        width: Fit, height: Fit
        padding: Inset{left: 12, right: 12, top: 6, bottom: 6}
        draw_bg +: {
            color: #xF0F4FA
            color_hover: #xE0E8F0
            color_down: #xD0D8E8
            border_radius: 13.0
            border_size: 1.0
            border_color: (COLOR_SECONDARY)
        }
        draw_text +: {
            color: (COLOR_TEXT)
            color_hover: (COLOR_TEXT)
            color_down: (COLOR_TEXT)
            text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
        }
        text: "Target"
    }

    mod.widgets.TargetMenuButton = Button {
        width: Fit, height: Fit
        padding: Inset{left: 12, right: 12, top: 7, bottom: 7}
        draw_bg +: {
            color: (COLOR_PRIMARY)
            color_hover: #F4F7FC
            color_down: #E8EEF8
            border_radius: 7.0
            border_size: 1.0
            border_color: (COLOR_SECONDARY)
        }
        draw_text +: {
            color: (COLOR_TEXT)
            color_hover: (COLOR_TEXT)
            color_down: (COLOR_TEXT)
            text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
        }
        text: "Option"
    }


    mod.widgets.RoomInputBar = set_type_default() do #(RoomInputBar::register_widget(vm)) {
        ..mod.widgets.RoundedView

        width: Fill,
        height: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.75}}
        flow: Down,
        clip_x: false,
        clip_y: false,

        // These margins are a hack to make the borders of the RoomInputBar
        // line up with the boundaries of its parent widgets.
        // This only works if the border_color is the same as its parents,
        // which is currently `COLOR_SECONDARY`.
        margin: Inset{left: -4, right: -4, bottom: -4 }
        show_bg: true,
        draw_bg +: {
            color: (COLOR_PRIMARY)
            border_radius: 5.0
            border_color: (COLOR_SECONDARY)
            border_size: 2.0
            // shadow_color: #0006
            // shadow_radius: 0.0
            // shadow_offset: vec2(0.0,0.0)
        }

        target_indicator := View {
            visible: false
            width: Fill, height: Fit
            flow: Down
            padding: Inset{left: 6, right: 6, top: 6, bottom: 2}
            spacing: 4

            target_chip_row := View {
                width: Fit, height: Fit
                flow: Right
                align: Align{y: 0.5}
                spacing: 6

                target_chip_button := mod.widgets.TargetChipButton { text: "Target" }

                target_chip_dismiss_button := RobrixNeutralIconButton {
                    visible: false
                    width: Fit, height: Fit
                    spacing: 0
                    text: ""
                    padding: Inset{left: 6, right: 6, top: 6, bottom: 6}
                    draw_icon +: {
                        svg: (ICON_CLOSE)
                    }
                    icon_walk: Walk{width: 10, height: 10}
                }
            }

            target_menu_popup := RoundedView {
                visible: false
                width: Fit, height: Fit
                flow: Down
                padding: 4
                spacing: 4
                show_bg: true
                draw_bg +: {
                    color: #xF7F9FD
                    border_radius: 8.0
                    border_size: 1.0
                    border_color: (COLOR_SECONDARY)
                }

                target_menu_room_button := mod.widgets.TargetMenuButton { text: "To room" }
                target_menu_bound_bot_button := mod.widgets.TargetMenuButton {
                    visible: false
                    text: "Bot"
                }
            }
        }

        // The top-most element is a preview of the message that the user is replying to, if any.
        replying_preview := ReplyingPreview { }

        // Below that, display a preview of the current location that a user is about to send.
        location_preview := LocationPreview { }

        // Upload progress view (shown when a file upload is in progress)
        upload_progress_view := UploadProgressView { }

        // Translation preview: shows the translated text above the input bar.
        translation_preview := RoundedView {
            visible: false
            width: Fill, height: Fit
            flow: Right
            padding: Inset{left: 12, right: 8, top: 8, bottom: 8}
            align: Align{y: 0.5}
            spacing: 8
            show_bg: true
            draw_bg +: {
                color: #xF0F4FA
                border_radius: 4.0
            }

            translation_lang_badge := RoundedView {
                width: Fit, height: Fit
                padding: Inset{left: 6, right: 6, top: 2, bottom: 2}
                show_bg: true
                draw_bg +: {
                    color: #xE0E8F0
                    border_radius: 3.0
                }
                translation_lang_code := Label {
                    width: Fit, height: Fit
                    draw_text +: {
                        color: #x555555
                        text_style: REGULAR_TEXT { font_size: 9 }
                    }
                    text: "en"
                }
            }

            translation_preview_text := Label {
                width: Fill, height: Fit
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    color: #x333333
                    text_style: REGULAR_TEXT { font_size: 11 }
                }
                text: ""
            }

            translation_apply_button := Button {
                width: Fit, height: Fit
                padding: Inset{top: 4, bottom: 4, left: 10, right: 10}
                text: "Apply"
                draw_bg +: {
                    color: (COLOR_ACTIVE_PRIMARY)
                    color_hover: (COLOR_ACTIVE_PRIMARY_DARKER)
                    color_down: #0C5DAA
                    border_radius: 4.0
                }
                draw_text +: {
                    color: #fff
                    text_style: REGULAR_TEXT { font_size: 10 }
                }
            }

            translation_close_button := RobrixIconButton {
                width: Fit, height: Fit
                padding: 4
                spacing: 0
                draw_icon +: {
                    svg: (ICON_CLOSE)
                    color: #x999999
                }
                draw_bg +: {
                    color: #0000
                    color_hover: #xE0E0E0
                    color_down: #xD0D0D0
                }
                icon_walk: Walk{width: 12, height: 12}
                text: ""
            }
        }

        // Below that, display one of multiple possible views:
        // * the message input bar (buttons and message TextInput).
        // * a notice that the user can't send messages to this room.
        // * if this room was tombstoned, a "footer" view showing the successor room info.
        // * the EditingPane, which slides up as an overlay in front of the other views below.
        overlay_wrapper := View {
            width: Fill,
            height: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.75}}
            flow: Overlay,

            // Below that, display a view that holds the message input bar and send button.
            input_bar := View {
                width: Fill,
                height: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.75}}
                flow: Down
                padding: 6,
                spacing: 4

                more_actions_popup := View {
                    visible: false
                    width: Fill
                    height: Fit
                    flow: Right{wrap: true}
                    spacing: 6
                    align: Align{x: 0.0, y: 0.5}

                    room_info_card_button := RobrixIconButton {
                        width: Fit
                        align: Align{x: 0.0, y: 0.5}
                        margin: Inset{top: 1, bottom: 1}
                        padding: Inset{left: 10, right: 10, top: 8, bottom: 8}
                        spacing: 8
                        draw_icon +: {
                            svg: (ICON_INFO)
                            color: (COLOR_ACTIVE_PRIMARY_DARKER)
                        },
                        draw_bg +: {
                            color: (COLOR_BG_PREVIEW)
                            color_hover: #E0E8F0
                            color_down: #D0D8E8
                            border_size: 1.0
                            border_color: (COLOR_SECONDARY)
                        }
                        draw_text +: {
                            color: (COLOR_TEXT)
                            color_hover: (COLOR_TEXT)
                            color_down: (COLOR_TEXT)
                            text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
                        }
                        icon_walk: Walk{width: 20, height: 20}
                        text: "info",
                    }

                    location_card_button := RobrixIconButton {
                        width: Fit
                        align: Align{x: 0.0, y: 0.5}
                        margin: Inset{top: 1, bottom: 1}
                        padding: Inset{left: 10, right: 10, top: 8, bottom: 8}
                        spacing: 8
                        draw_icon +: {
                            svg: (mod.widgets.ICO_LOCATION_PERSON)
                            color: (COLOR_ACTIVE_PRIMARY_DARKER)
                        },
                        draw_bg +: {
                            color: (COLOR_BG_PREVIEW)
                            color_hover: #E0E8F0
                            color_down: #D0D8E8
                            border_size: 1.0
                            border_color: (COLOR_SECONDARY)
                        }
                        draw_text +: {
                            color: (COLOR_TEXT)
                            color_hover: (COLOR_TEXT)
                            color_down: (COLOR_TEXT)
                            text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
                        }
                        icon_walk: Walk{width: 20, height: 20}
                        text: "location",
                    }

                    threads_card_button := RobrixIconButton {
                        width: Fit
                        align: Align{x: 0.0, y: 0.5}
                        margin: Inset{top: 1, bottom: 1}
                        padding: Inset{left: 10, right: 10, top: 8, bottom: 8}
                        spacing: 8
                        draw_icon +: {
                            svg: (mod.widgets.ICO_THREADS)
                            color: (COLOR_ACTIVE_PRIMARY_DARKER)
                        },
                        draw_bg +: {
                            color: (COLOR_BG_PREVIEW)
                            color_hover: #E0E8F0
                            color_down: #D0D8E8
                            border_size: 1.0
                            border_color: (COLOR_SECONDARY)
                        }
                        draw_text +: {
                            color: (COLOR_TEXT)
                            color_hover: (COLOR_TEXT)
                            color_down: (COLOR_TEXT)
                            text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
                        }
                        icon_walk: Walk{width: 20, height: 20}
                        text: "threads",
                    }
                }

                emoji_picker_popup := View {
                    visible: false
                    width: Fit
                    height: Fit
                    flow: Right{wrap: true}
                    align: Align{x: 0.0, y: 0.5}
                    margin: Inset{left: 5, top: 1, bottom: 1}
                    padding: Inset{left: 0, right: 0, top: 0, bottom: 0}
                    spacing: 6

                    emoji_smile_button := mod.widgets.RoomEmojiButton { text: "😀" }
                    emoji_joy_button := mod.widgets.RoomEmojiButton { text: "😂" }
                    emoji_thumbsup_button := mod.widgets.RoomEmojiButton { text: "👍" }
                    emoji_heart_button := mod.widgets.RoomEmojiButton { text: "❤️" }
                    emoji_fire_button := mod.widgets.RoomEmojiButton { text: "🔥" }
                    emoji_party_button := mod.widgets.RoomEmojiButton { text: "🎉" }
                    emoji_think_button := mod.widgets.RoomEmojiButton { text: "🤔" }
                    emoji_clap_button := mod.widgets.RoomEmojiButton { text: "👏" }
                }

                input_row := View {
                    width: Fill,
                    height: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.75}}
                    flow: Right
                    // Bottom-align everything to ensure that buttons always stick to the bottom
                    // even when the mentionable_text_input box is very tall.
                    align: Align{y: 1.0},

                    // A checkbox that enables TSP signing for the outgoing message.
                    // If TSP is not enabled, this will be an empty invisible view.
                    tsp_sign_checkbox := TspSignAnycastCheckbox {
                        margin: Inset{bottom: 9, left: 6, right: 0}
                    }

                    // Attachment button for uploading files/images
                    send_attachment_button := RobrixIconButton {
                        margin: Inset{left: 3, right: 1, top: 4, bottom: 4}
                        spacing: 0,
                        draw_icon +: {
                            svg: (ICON_ADD_ATTACHMENT)
                            color: (COLOR_ACTIVE_PRIMARY_DARKER)
                        },
                        draw_bg +: {
                            color: (COLOR_BG_PREVIEW)
                            color_hover: #E0E8F0
                            color_down: #D0D8E8
                        }
                        icon_walk: Walk{width: 21, height: 21}
                        text: "",
                    }

                    emoji_picker_button := RobrixIconButton {
                        margin: Inset{left: 3, right: 1, top: 4, bottom: 4}
                        spacing: 0,
                        draw_icon +: {
                            svg: (ICON_ADD_REACTION)
                            color: (COLOR_ACTIVE_PRIMARY_DARKER)
                        },
                        draw_bg +: {
                            color: (COLOR_BG_PREVIEW)
                            color_hover: #E0E8F0
                            color_down: #D0D8E8
                        }
                        icon_walk: Walk{width: 19, height: 19}
                        text: "",
                    }

                    translate_button := RobrixIconButton {
                        margin: Inset{left: 1, right: 1, top: 4, bottom: 4}
                        spacing: 0,
                        draw_icon +: {
                            svg: (mod.widgets.ICO_TRANSLATE)
                            color: (COLOR_ACTIVE_PRIMARY_DARKER)
                        },
                        draw_bg +: {
                            color: (COLOR_BG_PREVIEW)
                            color_hover: #xE0E8F0
                            color_down: #xD0D8E8
                        }
                        icon_walk: Walk{width: 19, height: 19}
                        text: "",
                    }

                    mentionable_text_input := MentionableTextInput {
                        width: Fill,
                        height: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.75}}
                        margin: Inset {
                            top: 3, // add some space between the top border of the text input and the top border of this row
                            bottom: 5.75, // to line up the middle of the text input with the middle of the buttons
                            left: 3, right: 3 // to give a bit of breathing room between the text input and the buttons on the sides
                        },

                        persistent +: {
                            center +: {
                                text_input := RobrixTextInput {
                                    empty_text: "Write a message (in Markdown) ..."
                                    is_multiline: true,
                                }
                            }
                        }
                    }

                    send_message_button := RobrixPositiveIconButton {
                        visible: false,
                        // Disabled by default; enabled when text is inputted
                        enabled: false,
                        spacing: 0,
                        text: "",
                        margin: 4
                        draw_icon +: { svg: (ICON_SEND) }
                        icon_walk: Walk{width: 21, height: 21},
                    }

                    more_actions_button := RobrixIconButton {
                        spacing: 0,
                        text: "",
                        margin: 4
                        draw_icon +: { svg: (mod.widgets.ICO_MENU) }
                        draw_bg +: {
                            color: (COLOR_ACTIVE_PRIMARY)
                            color_hover: (COLOR_ACTIVE_PRIMARY_DARKER)
                            color_down: #0C5DAA
                        }
                        icon_walk: Walk{width: 19, height: 19},
                    }
                }
            }

            can_not_send_message_notice := SolidView {
                visible: false
                padding: 20
                align: Align{x: 0.5, y: 0.5}
                width: Fill, height: Fit

                show_bg: true
                draw_bg.color: (COLOR_SECONDARY)

                text := Label {
                    width: Fill,
                    flow: Flow.Right{wrap: true},
                    align: Align{x: 0.5, y: 0.5}
                    draw_text +: {
                        color: (COLOR_TEXT)
                        text_style: theme.font_italic {font_size: 12.2}
                    }
                    text: "You don't have permission to post to this room.",
                }
            }

            tombstone_footer := TombstoneFooter { }

            editing_pane := EditingPane { }
        }

        translation_lang_wrapper := RoundedView {
            visible: false
            width: 220, height: Fit
            flow: Down
            padding: Inset{top: 4, bottom: 4}
            show_bg: true
            new_batch: true
            draw_bg +: {
                color: (COLOR_PRIMARY)
                border_radius: 6.0
                border_size: 1.0
                border_color: #ddd
                shadow_color: #0003
                shadow_radius: 8.0
                shadow_offset: vec2(0.0, 2.0)
            }

            translation_lang_scroll := ScrollYView {
                width: Fill, height: 288
                flow: Down
                spacing: 0

                lang_en := mod.widgets.TranslationLangItem {}
                lang_zh := mod.widgets.TranslationLangItem {}
                lang_zh_tw := mod.widgets.TranslationLangItem {}
                lang_ja := mod.widgets.TranslationLangItem {}
                lang_ko := mod.widgets.TranslationLangItem {}
                lang_es := mod.widgets.TranslationLangItem {}
                lang_fr := mod.widgets.TranslationLangItem {}
                lang_de := mod.widgets.TranslationLangItem {}
                lang_ru := mod.widgets.TranslationLangItem {}
                lang_pt := mod.widgets.TranslationLangItem {}
                lang_ar := mod.widgets.TranslationLangItem {}
                lang_vi := mod.widgets.TranslationLangItem {}
                lang_th := mod.widgets.TranslationLangItem {}
                lang_id := mod.widgets.TranslationLangItem {}
                lang_ms := mod.widgets.TranslationLangItem {}
                lang_tr := mod.widgets.TranslationLangItem {}
                lang_hi := mod.widgets.TranslationLangItem {}
            }
        }
    }
}

/// Main component for message input with @mention support
#[derive(Script, ScriptHook, Widget)]
pub struct RoomInputBar {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[rust] app_language: AppLanguage,
    #[rust] app_language_initialized: bool,

    /// Whether the `ReplyingPreview` was visible when the `EditingPane` was shown.
    /// If true, when the `EditingPane` gets hidden, we need to re-show the `ReplyingPreview`.
    #[rust] was_replying_preview_visible: bool,
    /// Info about the message event that the user is currently replying to, if any.
    #[rust] replying_to: Option<(EventTimelineItem, EmbeddedEvent)>,
    /// The user's explicit target override for this room.
    #[rust] explicit_override: ExplicitOverride,
    /// Whether the location card is currently expanded.
    #[rust] is_location_card_expanded: bool,
    /// Whether the emoji picker popup is currently expanded.
    #[rust] is_emoji_picker_expanded: bool,
    /// Cached natural Fit height of the input_bar, used as the animation
    /// target when the editing pane is being hidden.
    #[rust] input_bar_natural_height: f64,
    /// The pending file load operation, if any. Contains the receiver channel
    /// for receiving the loaded file data from a background thread.
    #[rust] pending_file_load: Option<crate::shared::file_upload_modal::FileLoadReceiver>,

    // --- Translation state ---
    /// Whether real-time translation is currently active.
    #[rust] translation_active: bool,
    /// The target language code (e.g., "en", "zh", "ja").
    #[rust] translation_target_code: String,
    /// The most recent translation result.
    #[rust] translation_preview_text: Option<String>,
    /// Whether a translation HTTP request is currently in flight.
    #[rust] translation_request_pending: bool,
    /// Whether the target selection menu is currently expanded.
    #[rust] is_target_menu_visible: bool,
    /// Debounce timer for translation requests.
    #[rust] translation_debounce_timer: Timer,
    /// The last source text that was sent for translation.
    #[rust] translation_last_source: String,
    /// Whether the language selector popup is visible.
    #[rust] is_lang_popup_visible: bool,
    /// Cached translation config, updated from AppState when translation is activated.
    #[rust] translation_config: Option<translation::TranslationConfig>,
}

impl Widget for RoomInputBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }

        let room_screen_props = scope
            .props
            .get::<RoomScreenProps>()
            .expect("BUG: RoomScreenProps should be available in Scope::props for RoomInputBar");

        match event.hits(cx, self.view.view(cx, ids!(replying_preview.reply_preview_content)).area()) {
            // If the hit occurred on the replying message preview, jump to it.
            Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                if let Some(event_id) = self.replying_to.as_ref()
                    .and_then(|(event_tl_item, _)| event_tl_item.event_id().map(ToOwned::to_owned))
                {
                    cx.widget_action(
                        room_screen_props.room_screen_widget_uid, 
                        MessageAction::JumpToEvent(event_id),
                    );
                } else {
                    enqueue_popup_notification(
                        "BUG: couldn't find the message you're replying to.",
                        PopupKind::Error,
                        None,
                    );
                }
            }
            _ => {}
        }

        // Always read the latest translation config from global state.
        // Settings may update it at any time via set_global_config().
        self.translation_config = translation::get_global_config();

        if let Event::Actions(actions) = event {
            self.handle_actions(cx, actions, room_screen_props);
        }

        // Handle signal events for pending file loads from background threads.
        if let Event::Signal = event {
            if let Some(receiver) = &self.pending_file_load {
                let mut remove_receiver = false;
                match receiver.try_recv() {
                    Ok(Some(loaded_data)) => {
                        let file_data = convert_loaded_data_to_file_data(loaded_data);
                        Cx::post_action(FilePreviewerAction::Show(file_data));
                        remove_receiver = true;
                    }
                    Ok(None) => {
                        remove_receiver = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        remove_receiver = true;
                    }
                }
                if remove_receiver {
                    self.pending_file_load = None;
                    self.redraw(cx);
                }
            }
        }

        // Handle translation debounce timer firing.
        if let Event::Timer(te) = event {
            if self.translation_debounce_timer.is_timer(te).is_some() && self.translation_active {
                log!("Translation: debounce timer fired, translation_active={}", self.translation_active);
                let mentionable_text_input = self.mentionable_text_input(cx, ids!(mentionable_text_input));
                let source_text = mentionable_text_input.text().trim().to_string();
                log!("Translation: source_text='{}', last_source='{}'", source_text, self.translation_last_source);
                if !source_text.is_empty() && source_text != self.translation_last_source {
                    self.translation_last_source = source_text.clone();
                    self.translation_request_pending = true;

                    self.view
                        .label(cx, ids!(translation_preview_text))
                        .set_text(cx, tr_key(self.app_language, "room_input_bar.translation.preview.loading"));
                    self.view.view(cx, ids!(translation_preview)).set_visible(cx, true);
                    self.redraw(cx);

                    log!("Translation: config cached={}, target='{}'", self.translation_config.is_some(), self.translation_target_code);
                    if let Some(config) = &self.translation_config {
                        log!("Translation: config enabled={}, api_url='{}', model='{}'", config.enabled, config.api_base_url, config.model);
                        if config.is_configured() {
                            let target_code = self.translation_target_code.clone();
                            log!("Translation: sending request for '{}' -> '{}'", source_text, target_code);
                            translation::send_translation_request(
                                cx,
                                config,
                                &source_text,
                                &target_code,
                            );
                        } else {
                            log!("Translation: config not properly configured");
                        }
                    } else {
                        log!("Translation: no cached config!");
                    }
                }
            }
        }

        // Handle translation HTTP response.
        if let Event::NetworkResponses(responses) = event {
            for response in responses {
                if let NetworkResponse::HttpResponse { request_id, response } = response {
                    if *request_id == TRANSLATION_REQUEST_ID {
                        self.translation_request_pending = false;
                        match translation::parse_translation_response(response) {
                            Ok(translated_text) => {
                                self.translation_preview_text = Some(translated_text.clone());
                                self.view.label(cx, ids!(translation_preview_text)).set_text(cx, &translated_text);
                                self.view.view(cx, ids!(translation_preview)).set_visible(cx, true);
                            }
                            Err(e) => {
                                log!("Translation error: {e}");
                                self.view.label(cx, ids!(translation_preview_text)).set_text(
                                    cx,
                                    &tr_fmt(self.app_language, "room_input_bar.translation.preview.error", &[("error", &e)]),
                                );
                            }
                        }
                        self.redraw(cx);
                    }
                }
            }
        }

        self.sync_target_indicator(cx, room_screen_props);
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }

        // Shrink the input_bar's height as the editing pane slides in,
        // and grow it back as the editing pane slides out.
        // slide=1.0 → editing pane hidden → input_bar at full Fit height.
        // slide=0.0 → editing pane shown → input_bar at zero height.
        let slide = self.editing_pane(cx, ids!(editing_pane)).slide();
        let input_bar = self.view.view(cx, ids!(input_bar));

        // Remap slide through a steeper curve so the input_bar reaches
        // its full target height before the ExpDecay tail.
        let remapped = (slide as f64 * 1.25).min(1.0);
        if remapped >= 1.0 {
            // Input_bar has reached its full natural height: switch to Fit
            // so it can respond to content changes normally.
            // Update the cached height for future animations.
            let h = input_bar.area().rect(cx).size.y;
            if h > 0.0 {
                self.input_bar_natural_height = h;
            }
            if let Some(mut inner) = input_bar.borrow_mut() {
                inner.walk.height = Size::fit();
            }
        } else {
            let target = self.input_bar_natural_height;
            if let Some(mut inner) = input_bar.borrow_mut() {
                inner.walk.height = Size::Fixed((target * remapped).max(0.0));
            }
        }

        let width = self.view.area().rect(cx).size.x as f32;
        let show_room_info_card = !(width > 1.0 && width < ROOM_INFO_CARD_MOBILE_BREAKPOINT);
        self.button(cx, ids!(room_info_card_button)).set_visible(cx, show_room_info_card);

        self.view.draw_walk(cx, scope, walk)
    }
}

impl RoomInputBar {
    fn current_resolved_target(&self, room_screen_props: &RoomScreenProps) -> ResolvedTarget {
        resolve_send_target(
            &self.explicit_override,
            self.replying_to_sender(),
            room_screen_props.bound_bot_user_id.as_deref(),
            room_screen_props.resolved_parent_bot_user_id.as_deref(),
            &room_screen_props.known_bot_user_ids,
        )
    }

    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.app_language_initialized = true;
        self.sync_app_language(cx);
    }

    fn sync_app_language(&mut self, cx: &mut Cx) {
        self.text_input(cx, ids!(input_bar.input_row.mentionable_text_input.text_input))
            .set_empty_text(cx, tr_key(self.app_language, "room_input_bar.input.placeholder").to_string());
        self.button(cx, ids!(translation_apply_button))
            .set_text(cx, tr_key(self.app_language, "room_input_bar.translation.preview.apply"));
        if self.translation_active {
            if self.translation_request_pending {
                self.view
                    .label(cx, ids!(translation_preview_text))
                    .set_text(cx, tr_key(self.app_language, "room_input_bar.translation.preview.loading"));
            } else if self.translation_preview_text.is_none() {
                self.view
                    .label(cx, ids!(translation_preview_text))
                    .set_text(cx, tr_key(self.app_language, "room_input_bar.translation.preview.idle"));
            }
        }
        self.view.redraw(cx);
    }

    /// Handles a language being selected from the popup.
    fn on_language_selected(&mut self, cx: &mut Cx, code: &str) {
        self.translation_target_code = code.to_string();
        self.translation_active = true;
        self.is_lang_popup_visible = false;
        self.view.view(cx, ids!(translation_lang_wrapper)).set_visible(cx, false);

        // Show the language code in the preview badge
        self.view.label(cx, ids!(translation_lang_code)).set_text(cx, code);
        self.view
            .label(cx, ids!(translation_preview_text))
            .set_text(cx, tr_key(self.app_language, "room_input_bar.translation.preview.idle"));
        self.view.view(cx, ids!(translation_preview)).set_visible(cx, true);

        // Focus the text input
        self.text_input(cx, ids!(input_bar.input_row.mentionable_text_input.text_input)).set_key_focus(cx);
        self.redraw(cx);
    }

    fn replying_to_sender(&self) -> Option<&UserId> {
        self.replying_to
            .as_ref()
            .map(|(event_tl_item, _embedded_event)| event_tl_item.sender())
    }

    fn target_display_name_for_user(
        &self,
        room_screen_props: &RoomScreenProps,
        user_id: &UserId,
    ) -> Option<String> {
        room_screen_props
            .room_members
            .as_ref()
            .and_then(|members| {
                members
                    .iter()
                    .find(|member| member.user_id() == user_id)
                    .and_then(|member| {
                        member
                            .display_name()
                            .map(str::trim)
                            .filter(|display_name| !display_name.is_empty())
                            .map(ToOwned::to_owned)
                    })
            })
    }

    fn resolved_target_display_name(
        &self,
        room_screen_props: &RoomScreenProps,
        resolved_target: &ResolvedTarget,
    ) -> Option<String> {
        match resolved_target {
            ResolvedTarget::NoTarget | ResolvedTarget::ExplicitRoom => None,
            ResolvedTarget::RoomDefault(user_id)
            | ResolvedTarget::ExplicitBot(user_id)
            | ResolvedTarget::ReplyBot(user_id) => {
                self.target_display_name_for_user(room_screen_props, user_id.as_ref())
            }
        }
    }

    fn sync_target_indicator(&mut self, cx: &mut Cx, room_screen_props: &RoomScreenProps) {
        let resolved_target = self.current_resolved_target(room_screen_props);
        let bot_display_name = self.resolved_target_display_name(room_screen_props, &resolved_target);
        let presentation = format_target_chip_presentation(
            self.app_language,
            &resolved_target,
            bot_display_name.as_deref(),
        );

        self.view
            .view(cx, ids!(target_indicator))
            .set_visible(cx, presentation.visible);
        self.view
            .view(cx, ids!(target_menu_popup))
            .set_visible(cx, presentation.visible && self.is_target_menu_visible);

        if !presentation.visible {
            self.is_target_menu_visible = false;
            return;
        }

        let mut target_chip_button = self.button(cx, ids!(target_chip_button));
        target_chip_button.set_text(cx, &presentation.label);
        let (text_color, border_color, bg_color, hover_color, down_color): (Vec4, Vec4, Vec4, Vec4, Vec4) = if presentation.subdued {
            (
                COLOR_MESSAGE_NOTICE_TEXT,
                COLOR_BG_DISABLED,
                vec4(0.89, 0.89, 0.89, 1.0),
                vec4(0.84, 0.84, 0.84, 1.0),
                vec4(0.78, 0.78, 0.78, 1.0),
            )
        } else {
            (
                COLOR_ACTIVE_PRIMARY_DARKER,
                COLOR_BG_DISABLED,
                COLOR_BG_PREVIEW,
                COLOR_BG_PREVIEW_HOVER,
                vec4(0.77, 0.86, 0.96, 1.0),
            )
        };
        script_apply_eval!(cx, target_chip_button, {
            draw_bg +: {
                border_color: #(border_color),
                color: #(bg_color),
                color_hover: #(hover_color),
                color_down: #(down_color),
            }
            draw_text +: {
                color: #(text_color),
                color_hover: #(text_color),
                color_down: #(text_color),
            }
        });

        self.button(cx, ids!(target_chip_dismiss_button))
            .set_visible(cx, presentation.dismissible);
        self.button(cx, ids!(target_menu_room_button))
            .set_text(cx, tr_key(self.app_language, "room_input_bar.target.menu.room"));

        let bound_bot_option = room_screen_props.bound_bot_user_id.as_deref().map(|bound_bot_user_id| {
            self.target_display_name_for_user(room_screen_props, bound_bot_user_id)
                .unwrap_or_else(|| bound_bot_user_id.localpart().to_string())
        });
        self.button(cx, ids!(target_menu_bound_bot_button))
            .set_visible(cx, bound_bot_option.is_some());
        if let Some(bound_bot_option) = bound_bot_option {
            self.button(cx, ids!(target_menu_bound_bot_button)).set_text(
                cx,
                &tr_fmt(
                    self.app_language,
                    "room_input_bar.target.menu.bound_bot",
                    &[("display_name", &bound_bot_option)],
                ),
            );
        }
    }

    fn handle_actions(
        &mut self,
        cx: &mut Cx,
        actions: &Actions,
        room_screen_props: &RoomScreenProps,
    ) {
        let mentionable_text_input = self.mentionable_text_input(cx, ids!(mentionable_text_input));
        let text_input = mentionable_text_input.text_input_ref();

        if self.button(cx, ids!(target_chip_button)).clicked(actions) {
            self.is_target_menu_visible = !self.is_target_menu_visible;
            self.redraw(cx);
        }

        if self.button(cx, ids!(target_chip_dismiss_button)).clicked(actions) {
            self.explicit_override = self.explicit_override.cleared();
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        if self.button(cx, ids!(target_menu_room_button)).clicked(actions) {
            self.explicit_override = apply_target_menu_selection(
                TargetMenuSelection::Room,
                room_screen_props.bound_bot_user_id.as_deref(),
            );
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        if self.button(cx, ids!(target_menu_bound_bot_button)).clicked(actions) {
            self.explicit_override = apply_target_menu_selection(
                TargetMenuSelection::BoundBot,
                room_screen_props.bound_bot_user_id.as_deref(),
            );
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        // Clear the replying-to preview pane if the "cancel reply" button was clicked
        // or if the `Escape` key was pressed within the message input box.
        if self.button(cx, ids!(cancel_reply_button)).clicked(actions)
            || text_input.escaped(actions)
        {
            self.clear_replying_to(cx);
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        // Handle the more actions button being clicked.
        if self.button(cx, ids!(more_actions_button)).clicked(actions) {
            self.is_location_card_expanded = !self.is_location_card_expanded;
            self.view.view(cx, ids!(more_actions_popup)).set_visible(cx, self.is_location_card_expanded);
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        // Handle the emoji picker button being clicked.
        if self.button(cx, ids!(emoji_picker_button)).clicked(actions) {
            self.is_emoji_picker_expanded = !self.is_emoji_picker_expanded;
            self.view.view(cx, ids!(emoji_picker_popup)).set_visible(cx, self.is_emoji_picker_expanded);
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        // Handle the add attachment button being clicked.
        if self.button(cx, ids!(send_attachment_button)).clicked(actions) {
            log!("Add attachment button clicked; opening file picker...");
            self.is_target_menu_visible = false;
            self.open_file_picker(cx);
        }

        let picked_emoji = if self.button(cx, ids!(emoji_smile_button)).clicked(actions) {
            Some("😀")
        } else if self.button(cx, ids!(emoji_joy_button)).clicked(actions) {
            Some("😂")
        } else if self.button(cx, ids!(emoji_thumbsup_button)).clicked(actions) {
            Some("👍")
        } else if self.button(cx, ids!(emoji_heart_button)).clicked(actions) {
            Some("❤️")
        } else if self.button(cx, ids!(emoji_fire_button)).clicked(actions) {
            Some("🔥")
        } else if self.button(cx, ids!(emoji_party_button)).clicked(actions) {
            Some("🎉")
        } else if self.button(cx, ids!(emoji_think_button)).clicked(actions) {
            Some("🤔")
        } else if self.button(cx, ids!(emoji_clap_button)).clicked(actions) {
            Some("👏")
        } else {
            None
        };

        if let Some(emoji) = picked_emoji {
            let mut text = mentionable_text_input.text();
            text.push_str(emoji);
            mentionable_text_input.set_text(cx, &text);
            self.enable_send_message_button(cx, !text.trim().is_empty());
            submit_async_request(MatrixRequest::SendTypingNotice {
                room_id: room_screen_props.timeline_kind.room_id().clone(),
                typing: !text.is_empty(),
            });
            self.is_emoji_picker_expanded = false;
            self.view.view(cx, ids!(emoji_picker_popup)).set_visible(cx, false);
            self.text_input(cx, ids!(input_bar.input_row.mentionable_text_input.text_input)).set_key_focus(cx);
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        // Handle the translate button being clicked — toggle language selector popup.
        if self.button(cx, ids!(translate_button)).clicked(actions) {
            if self.translation_active {
                // Turn off translation
                self.translation_active = false;
                self.translation_preview_text = None;
                self.translation_request_pending = false;
                self.translation_last_source.clear();
                self.view.view(cx, ids!(translation_preview)).set_visible(cx, false);
                self.view.view(cx, ids!(translation_lang_wrapper)).set_visible(cx, false);
                self.is_lang_popup_visible = false;
                self.is_target_menu_visible = false;
                self.redraw(cx);
            } else {
                self.view.view(cx, ids!(translation_lang_wrapper)).set_visible(cx, false);
                self.is_lang_popup_visible = false;
                self.is_target_menu_visible = false;
                let button_rect = self.button(cx, ids!(translate_button)).area().clipped_rect(cx);
                if button_rect.size.x > 0.0 {
                    cx.widget_action(
                        room_screen_props.room_screen_widget_uid,
                        MessageAction::ToggleTranslationLangPopup { button_rect },
                    );
                }
            }
        }

        // Handle "Apply" button on translation preview — replace input text with translation.
        if self.button(cx, ids!(translation_apply_button)).clicked(actions) {
            if let Some(translated) = self.translation_preview_text.clone() {
                let outcome = compute_translation_apply_outcome(&translated);
                mentionable_text_input.set_text(cx, &outcome.input_text);
                self.enable_send_message_button(cx, !outcome.input_text.trim().is_empty());
                self.translation_preview_text = Some(outcome.preserved_preview_text.clone());
                self.translation_last_source = outcome.next_last_source;
                self.view.label(cx, ids!(translation_preview_text)).set_text(cx, &outcome.preserved_preview_text);
                self.view.view(cx, ids!(translation_preview)).set_visible(cx, outcome.keep_preview_visible);
                self.text_input(cx, ids!(input_bar.input_row.mentionable_text_input.text_input)).set_key_focus(cx);
                self.is_target_menu_visible = false;
                self.redraw(cx);
            }
        }

        // Handle close button on translation preview.
        if self.button(cx, ids!(translation_close_button)).clicked(actions) {
            self.translation_active = false;
            self.translation_preview_text = None;
            self.translation_request_pending = false;
            self.translation_last_source.clear();
            self.view.view(cx, ids!(translation_preview)).set_visible(cx, false);
            self.view.view(cx, ids!(translation_lang_wrapper)).set_visible(cx, false);
            self.is_lang_popup_visible = false;
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        // Handle the location card being clicked.
        if self.button(cx, ids!(location_card_button)).clicked(actions) {
            log!("Location card clicked; requesting current location...");
            self.is_location_card_expanded = false;
            self.view.view(cx, ids!(more_actions_popup)).set_visible(cx, false);
            self.is_target_menu_visible = false;
            if let Err(_e) = init_location_subscriber(cx) {
                error!("Failed to initialize location subscriber");
                enqueue_popup_notification(
                    "Failed to initialize location services.",
                    PopupKind::Error,
                    None,
                );
            }
            self.view.location_preview(cx, ids!(location_preview)).show();
            self.redraw(cx);
        }

        if self.button(cx, ids!(threads_card_button)).clicked(actions) {
            cx.widget_action(
                room_screen_props.room_screen_widget_uid,
                MessageAction::ShowThreadsPane,
            );
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        if self.button(cx, ids!(room_info_card_button)).clicked(actions) {
            cx.widget_action(
                room_screen_props.room_screen_widget_uid,
                MessageAction::ShowRoomInfoPane,
            );
            self.is_target_menu_visible = false;
            self.redraw(cx);
        }

        // Handle the send location button being clicked.
        if self.button(cx, ids!(location_preview.send_location_button)).clicked(actions) {
            let location_preview = self.location_preview(cx, ids!(location_preview));
            if let Some((coords, _system_time_opt)) = location_preview.get_current_data() {
                let geo_uri = format!("{}{},{}", utils::GEO_URI_SCHEME, coords.latitude, coords.longitude);
                let message = RoomMessageEventContent::new(
                    MessageType::Location(
                        LocationMessageEventContent::new(geo_uri.clone(), geo_uri)
                    )
                );
                let resolved_target = self.current_resolved_target(room_screen_props);
                let target_user_id = resolved_target_user_id(&resolved_target);
                let explicit_room = matches!(resolved_target, ResolvedTarget::ExplicitRoom);
                let replied_to = self.replying_to.take().and_then(|(event_tl_item, _emb)|
                    event_tl_item.event_id().map(|event_id| {
                        let enforce_thread = if room_screen_props.timeline_kind.thread_root_event_id().is_some() {
                            EnforceThread::Threaded(ReplyWithinThread::Yes)
                        } else {
                            EnforceThread::MaybeThreaded
                        };
                        Reply {
                            event_id: event_id.to_owned(),
                            enforce_thread,
                            add_mentions: AddMentions::Yes,
                        }
                    })
                ).or_else(||
                    room_screen_props.timeline_kind.thread_root_event_id().map(|thread_root_event_id|
                        Reply {
                            event_id: thread_root_event_id.clone(),
                            enforce_thread: EnforceThread::Threaded(ReplyWithinThread::No),
                            add_mentions: AddMentions::No,
                        }
                    )
                );
                submit_async_request(MatrixRequest::SendMessage {
                    timeline_kind: room_screen_props.timeline_kind.clone(),
                    message,
                    replied_to,
                    target_user_id,
                    explicit_room,
                    #[cfg(feature = "tsp")]
                    sign_with_tsp: self.is_tsp_signing_enabled(cx),
                });
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    MessageAction::MessageSubmittedLocally,
                );

                self.clear_replying_to(cx);
                location_preview.clear();
                location_preview.redraw(cx);
                self.is_target_menu_visible = false;
            }
        }

        // Handle the send message button being clicked or Cmd/Ctrl + Return being pressed.
        if self.button(cx, ids!(send_message_button)).clicked(actions)
            || text_input.returned(actions).is_some_and(|(_, m)| m.is_primary())
        {
            let entered_text = mentionable_text_input.text().trim().to_string();
            if !entered_text.is_empty() {
                if self.try_handle_bot_shortcut(cx, &entered_text, room_screen_props) {
                    self.clear_replying_to(cx);
                    mentionable_text_input.set_text(cx, "");
                    submit_async_request(MatrixRequest::SendTypingNotice {
                        room_id: room_screen_props.timeline_kind.room_id().clone(),
                        typing: false,
                    });
                    self.enable_send_message_button(cx, false);
                    self.is_target_menu_visible = false;
                    self.redraw(cx);
                    return;
                }
                let resolved_target = self.current_resolved_target(room_screen_props);
                let message = mentionable_text_input.create_message_with_mentions(&entered_text);
                let message_mentions_bot = message_mentions_known_bot(
                    &message,
                    room_screen_props.bound_bot_user_id.as_deref(),
                    room_screen_props.resolved_parent_bot_user_id.as_deref(),
                    &room_screen_props.room_bot_user_ids,
                    &room_screen_props.known_bot_user_ids,
                );
                let (target_user_id, explicit_room) =
                    routing_directives_for_message(&resolved_target, message_mentions_bot);
                let replied_to = self.replying_to.take().and_then(|(event_tl_item, _emb)|
                    event_tl_item.event_id().map(|event_id| {
                        let enforce_thread = if room_screen_props.timeline_kind.thread_root_event_id().is_some() {
                            EnforceThread::Threaded(ReplyWithinThread::Yes)
                        } else {
                            EnforceThread::MaybeThreaded
                        };
                        Reply {
                            event_id: event_id.to_owned(),
                            enforce_thread,
                            add_mentions: AddMentions::Yes,
                        }
                    })
                ).or_else(||
                    room_screen_props.timeline_kind.thread_root_event_id().map(|thread_root_event_id|
                        Reply {
                            event_id: thread_root_event_id.clone(),
                            enforce_thread: EnforceThread::Threaded(ReplyWithinThread::No),
                            add_mentions: AddMentions::No,
                        }
                    )
                );
                submit_async_request(MatrixRequest::SendMessage {
                    timeline_kind: room_screen_props.timeline_kind.clone(),
                    message,
                    replied_to,
                    target_user_id,
                    explicit_room,
                    #[cfg(feature = "tsp")]
                    sign_with_tsp: self.is_tsp_signing_enabled(cx),
                });
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    MessageAction::MessageSubmittedLocally,
                );

                self.clear_replying_to(cx);
                mentionable_text_input.set_text(cx, "");
                self.enable_send_message_button(cx, false);
                self.is_target_menu_visible = false;
            }
        }

        // If the user starts/stops typing in the message input box,
        // send a typing notice to the room and update the send_message_button state.
        let is_text_input_empty = if let Some(new_text) = text_input.changed(actions) {
            let is_empty = new_text.is_empty();
            submit_async_request(MatrixRequest::SendTypingNotice {
                room_id: room_screen_props.timeline_kind.room_id().clone(),
                typing: !is_empty,
            });

            // Trigger translation debounce if translation mode is active.
            if self.translation_active {
                let trimmed = new_text.trim().to_string();
                log!("Translation: text changed, trimmed='{}', last_source='{}'", trimmed, self.translation_last_source);
                if !trimmed.is_empty() && trimmed != self.translation_last_source {
                    cx.stop_timer(self.translation_debounce_timer);
                    self.translation_debounce_timer = cx.start_timeout(0.5);
                    log!("Translation: debounce timer started");
                }
            }

            is_empty
        } else {
            text_input.text().is_empty()
        };
        self.enable_send_message_button(cx, !is_text_input_empty);

        // Handle the user pressing the up arrow in an empty message input box
        // to edit their latest sent message.
        if is_text_input_empty {
            if let Some(KeyEvent {
                key_code: KeyCode::ArrowUp,
                modifiers: KeyModifiers { shift: false, control: false, alt: false, logo: false },
                ..
            }) = text_input.key_down_unhandled(actions) {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid, 
                    MessageAction::EditLatest,
                );
            }
        }

        // When the hide animation fully completes, restore the replying preview.
        if self.view.editing_pane(cx, ids!(editing_pane)).was_hidden(actions) {
            self.on_editing_pane_hidden(cx);
        }
    }

    /// Shows a preview of the given event that the user is currently replying to
    /// above the message input bar.
    ///
    /// If `grab_key_focus` is true, this will also automatically focus the keyboard
    /// on the message input box so that the user can immediately start typing their reply.
    fn show_replying_to(
        &mut self,
        cx: &mut Cx,
        replying_to: (EventTimelineItem, EmbeddedEvent),
        timeline_kind: &TimelineKind,
        grab_key_focus: bool,
    ) {
        // When the user clicks the reply button next to a message, we need to:
        // 1. Populate and show the ReplyingPreview, of course.
        let replying_preview = self.view(cx, ids!(replying_preview));
        let (replying_preview_username, _) = replying_preview
            .avatar(cx, ids!(reply_preview_content.reply_preview_avatar))
            .set_avatar_and_get_username(
                cx,
                timeline_kind,
                replying_to.0.sender(),
                Some(replying_to.0.sender_profile()),
                replying_to.0.event_id(),
                true,
            );

        replying_preview
            .label(cx, ids!(reply_preview_content.reply_preview_username))
            .set_text(cx, replying_preview_username.as_str());

        populate_preview_of_timeline_item(
            cx,
            &replying_preview.html_or_plaintext(cx, ids!(reply_preview_content.reply_preview_body)),
            self.app_language,
            replying_to.0.content(),
            replying_to.0.sender(),
            &replying_preview_username,
        );

        replying_preview.set_visible(cx, true);
        self.replying_to = Some(replying_to);
        self.is_target_menu_visible = false;
        self.view.view(cx, ids!(target_menu_popup)).set_visible(cx, false);

        // 2. Hide other views that are irrelevant to a reply, e.g.,
        //    the `EditingPane` would improperly cover up the ReplyPreview.
        self.editing_pane(cx, ids!(editing_pane)).force_reset_hide(cx);
        self.on_editing_pane_hidden(cx);
        // 3. Automatically focus the keyboard on the message input box
        //    so that the user can immediately start typing their reply
        //    without having to manually click on the message input box.
        if grab_key_focus {
            self.text_input(cx, ids!(input_bar.input_row.mentionable_text_input.text_input)).set_key_focus(cx);
        }
        self.button(cx, ids!(cancel_reply_button)).reset_hover(cx);
        self.redraw(cx);
    }

    /// Clears (and makes invisible) the preview of the message
    /// that the user is currently replying to.
    fn clear_replying_to(&mut self, cx: &mut Cx) {
        self.view(cx, ids!(replying_preview)).set_visible(cx, false);
        self.replying_to = None;
        self.is_target_menu_visible = false;
        self.view.view(cx, ids!(target_menu_popup)).set_visible(cx, false);
    }

    /// Shows the editing pane to allow the user to edit the given event.
    fn show_editing_pane(
        &mut self,
        cx: &mut Cx,
        behavior: ShowEditingPaneBehavior,
        timeline_kind: TimelineKind,
    ) {
        // Cache the input_bar's natural height before the animation shrinks it.
        let input_bar_height = self.view.view(cx, ids!(input_bar)).area().rect(cx).size.y;
        if input_bar_height > 0.0 {
            self.input_bar_natural_height = input_bar_height;
        }

        // Hide the replying preview and location preview while the editing
        // pane is shown. The input_bar is not hidden; instead it is slid out
        // of view in draw_walk using the EditingPane's slide value.
        let replying_preview = self.view.view(cx, ids!(replying_preview));
        self.was_replying_preview_visible = replying_preview.visible();
        replying_preview.set_visible(cx, false);
        self.view.location_preview(cx, ids!(location_preview)).clear();

        let editing_pane = self.view.editing_pane(cx, ids!(editing_pane));
        self.is_target_menu_visible = false;
        self.view.view(cx, ids!(target_menu_popup)).set_visible(cx, false);
        match behavior {
            ShowEditingPaneBehavior::ShowNew { event_tl_item } => {
                editing_pane.show(cx, event_tl_item, timeline_kind);
            }
            ShowEditingPaneBehavior::RestoreExisting { editing_pane_state } => {
                editing_pane.restore_state(cx, editing_pane_state, timeline_kind);
            }
        };

        self.redraw(cx);
    }

    /// This should be invoked after the EditingPane has been fully hidden.
    fn on_editing_pane_hidden(&mut self, cx: &mut Cx) {
        // Restore the replying_preview.
        if self.was_replying_preview_visible && self.replying_to.is_some() {
            self.view.view(cx, ids!(replying_preview)).set_visible(cx, true);
        }
        self.redraw(cx);
        // We don't need to do anything with the editing pane itself here,
        // because it has already been hidden by the time this function gets called.
    } 

    /// Updates (populates and shows or hides) this room's tombstone footer
    /// based on the given successor room details.
    fn update_tombstone_footer(
        &mut self,
        cx: &mut Cx,
        tombstoned_room_id: &OwnedRoomId,
        successor_room_details: Option<&SuccessorRoomDetails>,
    ) {
        let tombstone_footer = self.tombstone_footer(cx, ids!(tombstone_footer));
        let input_bar = self.view(cx, ids!(input_bar));

        if let Some(srd) = successor_room_details {
            tombstone_footer.show(cx, tombstoned_room_id, srd);
            input_bar.set_visible(cx, false);
        } else {
            tombstone_footer.hide(cx);
            input_bar.set_visible(cx, true);
        }
    }

    /// Sets the send_message_button to be shown/enabled and green, or hidden/disabled and gray.
    ///
    /// This should be called to update the button state when the message TextInput content changes.
    fn enable_send_message_button(&mut self, cx: &mut Cx, enable: bool) {
        let mut send_message_button = self.view.button(cx, ids!(send_message_button));
        let (fg_color, bg_color) = if enable {
            (COLOR_FG_ACCEPT_GREEN, COLOR_BG_ACCEPT_GREEN)
        } else {
            (COLOR_FG_DISABLED, COLOR_BG_DISABLED)
        };
        script_apply_eval!(cx, send_message_button, {
            visible: #(enable),
            enabled: #(enable),
            draw_icon.color: #(fg_color),
            draw_bg.color: #(bg_color),
        });
    }

    fn try_handle_bot_shortcut(
        &mut self,
        cx: &mut Cx,
        entered_text: &str,
        room_screen_props: &RoomScreenProps,
    ) -> bool {
        if !(entered_text == "/bot" || entered_text.starts_with("/bot ")) {
            return false;
        }

        let popup_message = if room_screen_props.timeline_kind.thread_root_event_id().is_some() {
            Some((
                "Bot commands are only supported in the main room timeline.",
                PopupKind::Warning,
            ))
        } else if entered_text != "/bot" {
            Some((
                "Only `/bot` is supported right now. Use `/bot` and choose an action from the room panel.",
                PopupKind::Info,
            ))
        } else if !room_screen_props.app_service_enabled {
            Some((
                "Enable App Service in Settings before using /bot.",
                PopupKind::Warning,
            ))
        } else {
            None
        };

        if let Some((message, kind)) = popup_message {
            enqueue_popup_notification(message, kind, Some(4.0));
        } else {
            cx.widget_action(
                room_screen_props.room_screen_widget_uid,
                MessageAction::ToggleAppServiceActions,
            );
        }

        true
    }

    /// Updates the visibility of select views based on the user's new power levels.
    ///
    /// This will show/hide the `input_bar` and the `can_not_send_message_notice` views.
    fn update_user_power_levels(
        &mut self,
        cx: &mut Cx,
        user_power_levels: UserPowerLevels,
    ) {
        let can_send = user_power_levels.can_send_message();
        self.view.view(cx, ids!(input_bar)).set_visible(cx, can_send);
        self.view.view(cx, ids!(can_not_send_message_notice)).set_visible(cx, !can_send);
    }

    /// Returns true if the TSP signing checkbox is checked, false otherwise.
    ///
    /// If TSP is not enabled, this will always return false.
    #[cfg(feature = "tsp")]
    fn is_tsp_signing_enabled(&self, cx: &mut Cx) -> bool {
        self.view.check_box(cx, ids!(tsp_sign_checkbox)).active(cx)
    }

    /// Opens the native file picker dialog to select a file for upload.
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    fn open_file_picker(&mut self, cx: &mut Cx) {
        // Run file dialog on main thread (required for non-windowed environments)
        let dialog = rfd::FileDialog::new()
            .set_title("Select file to upload")
            .add_filter("All files", &["*"])
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
            .add_filter("Documents", &["pdf", "doc", "docx", "txt", "rtf"]);

        if let Some(selected_file_path) = dialog.pick_file() {
            // Get file metadata
            let file_size = match std::fs::metadata(&selected_file_path) {
                Ok(metadata) => metadata.len(),
                Err(e) => {
                    makepad_widgets::error!("Failed to read file metadata: {e}");
                    enqueue_popup_notification(
                        format!("Unable to access file: {e}"),
                        PopupKind::Error,
                        None,
                    );
                    return;
                }
            };

            // Check for empty files
            if file_size == 0 {
                enqueue_popup_notification("Cannot upload empty file", PopupKind::Error, None);
                return;
            }

            // Detect the MIME type from the file extension
            let mime = mime_guess::from_path(&selected_file_path)
                .first_or_octet_stream();

            // Create channel for receiving loaded file data
            let (sender, receiver) = std::sync::mpsc::channel();
            self.pending_file_load = Some(receiver);

            // Spawn background thread to generate thumbnail (for images)
            let path_clone = selected_file_path.clone();
            let mime_clone = mime.clone();
            cx.spawn_thread(move || {
                // Generate thumbnail for images
                let (thumbnail, dimensions) = if crate::image_utils::is_displayable_image(mime_clone.as_ref()) {
                    match std::fs::read(&path_clone) {
                        Ok(data) => {
                            match crate::image_utils::generate_thumbnail(&data) {
                                Ok((thumb_data, width, height)) => (
                                    Some(ThumbnailData { data: thumb_data, width, height }),
                                    Some((width, height))
                                ),
                                Err(e) => {
                                    makepad_widgets::error!("Failed to generate thumbnail: {e}");
                                    (None, None)
                                }
                            }
                        }
                        Err(e) => {
                            makepad_widgets::error!("Failed to read file for thumbnail: {e}");
                            (None, None)
                        }
                    }
                } else {
                    (None, None)
                };

                let loaded_data = FileLoadedData {
                    metadata: FilePreviewerMetaData {
                        mime: mime_clone,
                        file_size,
                        file_path: path_clone,
                    },
                    thumbnail,
                    dimensions,
                };

                if sender.send(Some(loaded_data)).is_err() {
                    makepad_widgets::error!("Failed to send file data to UI: receiver dropped");
                }
                SignalToUI::set_ui_signal();
            });
        }
    }

    /// Shows a "not supported" message on mobile platforms.
    #[cfg(any(target_os = "ios", target_os = "android"))]
    fn open_file_picker(&mut self, _cx: &mut Cx) {
        enqueue_popup_notification(
            "File uploads are not yet supported on this platform.",
            PopupKind::Error,
            None,
        );
    }
}

impl RoomInputBarRef {
    pub fn activate_translation_language(&self, cx: &mut Cx, code: &str) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.on_language_selected(cx, code);
    }

    pub fn set_app_language(&self, cx: &mut Cx, app_language: AppLanguage) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_app_language(cx, app_language);
    }

    /// Shows a preview of the given event that the user is currently replying to
    /// above the message input bar.
    pub fn show_replying_to(
        &self,
        cx: &mut Cx,
        replying_to: (EventTimelineItem, EmbeddedEvent),
        timeline_kind: &TimelineKind,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show_replying_to(cx, replying_to, timeline_kind, true);
    }

    /// Shows the editing pane to allow the user to edit the given event.
    pub fn show_editing_pane(
        &self,
        cx: &mut Cx,
        event_tl_item: EventTimelineItem,
        timeline_kind: TimelineKind,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show_editing_pane(
            cx,
            ShowEditingPaneBehavior::ShowNew { event_tl_item },
            timeline_kind,
        );
    }

    /// Updates the visibility of select views based on the user's new power levels.
    ///
    /// This will show/hide the `input_bar` and the `can_not_send_message_notice` views.
    pub fn update_user_power_levels(
        &self,
        cx: &mut Cx,
        user_power_levels: UserPowerLevels,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.update_user_power_levels(cx, user_power_levels);
    }

    /// Updates this room's tombstone footer based on the given `tombstone_state`.
    pub fn update_tombstone_footer(
        &self,
        cx: &mut Cx,
        tombstoned_room_id: &OwnedRoomId,
        successor_room_details: Option<&SuccessorRoomDetails>,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.update_tombstone_footer(cx, tombstoned_room_id, successor_room_details);
    }

    /// Forwards the result of an edit request to the `EditingPane` widget
    /// within this `RoomInputBar`.
    pub fn handle_edit_result(
        &self,
        cx: &mut Cx,
        timeline_event_item_id: TimelineEventItemId,
        edit_result: Result<(), matrix_sdk_ui::timeline::Error>,
    ) {
        let Some(inner) = self.borrow_mut() else { return };
        inner.editing_pane(cx, ids!(editing_pane))
            .handle_edit_result(cx, timeline_event_item_id, edit_result);
    }

    /// Save a snapshot of the UI state of this `RoomInputBar`.
    pub fn save_state(&self) -> RoomInputBarState {
        let Some(inner) = self.borrow() else { return Default::default() };
        // Clear the location preview. We don't save this state because the
        // current location might change by the next time the user opens this same room.
        inner.child_by_path(ids!(location_preview)).as_location_preview().clear();
        RoomInputBarState {
            was_replying_preview_visible: inner.was_replying_preview_visible,
            replying_to: inner.replying_to.clone(),
            explicit_override: inner.explicit_override.clone(),
            editing_pane_state: inner.child_by_path(ids!(editing_pane)).as_editing_pane().save_state(),
            text_input_state: inner.child_by_path(ids!(input_bar.input_row.mentionable_text_input.text_input)).as_text_input().save_state(),
        }
    }

    /// Restore the UI state of this `RoomInputBar` from the given state snapshot.
    pub fn restore_state(
        &self,
        cx: &mut Cx,
        timeline_kind: TimelineKind,
        saved_state: RoomInputBarState,
        user_power_levels: UserPowerLevels,
        tombstone_info: Option<&SuccessorRoomDetails>,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        let explicit_override = restored_explicit_override(&saved_state);
        let RoomInputBarState {
            was_replying_preview_visible,
            text_input_state,
            replying_to,
            explicit_override: _,
            editing_pane_state,
        } = saved_state;

        // Note: we do *not* restore the location preview state here; see `save_state()`.

        // 0. Update select views based on user power levels from the RoomScreen (the `TimelineUiState`).
        //    This must happen before we restore the state of the `EditingPane`,
        //    because the call to `show_editing_pane()` might re-update the `input_bar`'s visibility.
        inner.update_user_power_levels(cx, user_power_levels);

        // 1. Restore the state of the TextInput within the MentionableTextInput.
        inner.text_input(cx, ids!(input_bar.input_row.mentionable_text_input.text_input))
            .restore_state(cx, text_input_state);
        let is_text_input_empty = inner.text_input(cx, ids!(input_bar.input_row.mentionable_text_input.text_input))
            .text()
            .is_empty();
        inner.enable_send_message_button(cx, !is_text_input_empty);
        inner.is_location_card_expanded = false;
        inner.view.view(cx, ids!(more_actions_popup)).set_visible(cx, false);
        inner.is_emoji_picker_expanded = false;
        inner.view.view(cx, ids!(emoji_picker_popup)).set_visible(cx, false);
        inner.is_target_menu_visible = false;
        inner.view.view(cx, ids!(target_menu_popup)).set_visible(cx, false);
        inner.is_lang_popup_visible = false;
        inner.view.view(cx, ids!(translation_lang_wrapper)).set_visible(cx, false);

        // 2. Restore the state of the replying-to preview.
        if let Some(replying_to) = replying_to {
            inner.show_replying_to(cx, replying_to, &timeline_kind, false);
        } else {
            inner.clear_replying_to(cx);
        }
        inner.was_replying_preview_visible = was_replying_preview_visible;
        inner.explicit_override = explicit_override;

        // 3. Restore the state of the editing pane.
        if let Some(editing_pane_state) = editing_pane_state {
            inner.show_editing_pane(
                cx,
                ShowEditingPaneBehavior::RestoreExisting { editing_pane_state },
                timeline_kind.clone(),
            );
        } else {
            inner.editing_pane(cx, ids!(editing_pane)).force_reset_hide(cx);
            inner.on_editing_pane_hidden(cx);
        }

        // 4. Restore the state of the tombstone footer.
        //    This depends on the `EditingPane` state, so it must be done after Step 3.
        inner.update_tombstone_footer(cx, timeline_kind.room_id(), tombstone_info);
    }

    /// Shows the upload progress view for a file upload.
    pub fn show_upload_progress(&self, cx: &mut Cx, file_name: &str) {
        let Some(inner) = self.borrow() else { return };
        inner.child_by_path(ids!(upload_progress_view))
            .as_upload_progress_view()
            .show(cx, file_name);
    }

    /// Hides the upload progress view.
    pub fn hide_upload_progress(&self, cx: &mut Cx) {
        let Some(inner) = self.borrow() else { return };
        inner.child_by_path(ids!(upload_progress_view))
            .as_upload_progress_view()
            .hide(cx);
    }

    /// Updates the upload progress.
    pub fn set_upload_progress(&self, cx: &mut Cx, current: u64, total: u64) {
        let Some(inner) = self.borrow() else { return };
        inner.child_by_path(ids!(upload_progress_view))
            .as_upload_progress_view()
            .set_progress(cx, current, total);
    }

    /// Sets the abort handle for the current upload.
    pub fn set_upload_abort_handle(&self, handle: tokio::task::AbortHandle) {
        let Some(inner) = self.borrow_mut() else { return };
        inner.child_by_path(ids!(upload_progress_view))
            .as_upload_progress_view()
            .set_abort_handle(handle);
    }

    /// Shows an upload error with retry option.
    pub fn show_upload_error(&self, cx: &mut Cx, error: &str, file_data: FileData) {
        let Some(inner) = self.borrow() else { return };
        inner.child_by_path(ids!(upload_progress_view))
            .as_upload_progress_view()
            .show_error(cx, error, file_data);
    }

    /// Handles a confirmed file upload from the file upload modal.
    ///
    /// This method:
    /// - Shows the upload progress view
    /// - Gets and clears any "replying to" state
    /// - Returns the reply metadata needed to submit the upload request
    pub fn handle_file_upload_confirmed(&self, cx: &mut Cx, file_name: &str) -> Option<Option<matrix_sdk::room::reply::Reply>> {
        use matrix_sdk::room::reply::{EnforceThread, Reply};

        let mut inner = self.borrow_mut()?;

        // Get the reply metadata if replying to a message
        let replied_to = inner
            .replying_to
            .take()
            .and_then(|(event_tl_item, _embedded_event)| {
                event_tl_item.event_id().map(|event_id| Reply {
                    event_id: event_id.to_owned(),
                    enforce_thread: EnforceThread::MaybeThreaded,
                    add_mentions: ruma::events::room::message::AddMentions::Yes,
                })
            });

        // Show the upload progress view
        inner.child_by_path(ids!(upload_progress_view))
            .as_upload_progress_view()
            .show(cx, file_name);

        // Clear the replying-to state
        inner.clear_replying_to(cx);

        Some(replied_to)
    }

    /// Returns whether TSP signing is enabled.
    #[cfg(feature = "tsp")]
    pub fn is_tsp_signing_enabled(&self, cx: &mut Cx) -> bool {
        let Some(inner) = self.borrow() else { return false };
        inner.is_tsp_signing_enabled(cx)
    }
}

/// Converts `FileLoadedData` from background thread to `FileData` for the modal.
fn convert_loaded_data_to_file_data(loaded: FileLoadedData) -> FileData {
    // Read the file data from the path
    let data = std::fs::read(&loaded.metadata.file_path).unwrap_or_default();
    let name = loaded.metadata.file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    FileData {
        path: loaded.metadata.file_path,
        name,
        mime_type: loaded.metadata.mime.to_string(),
        data,
        size: loaded.metadata.file_size,
        thumbnail: loaded.thumbnail,
    }
}

/// The saved UI state of a `RoomInputBar` widget.
#[derive(Default)]
pub struct RoomInputBarState {
    /// Whether or not the `replying_preview` widget was shown.
    was_replying_preview_visible: bool,
    /// The state of the `TextInput` within the `mentionable_text_input`.
    text_input_state: TextInputState,
    /// The event that the user is currently replying to, if any.
    replying_to: Option<(EventTimelineItem, EmbeddedEvent)>,
    /// The user's explicit target override for this room.
    explicit_override: ExplicitOverride,
    /// The state of the `EditingPane`, if any message was being edited.
    editing_pane_state: Option<EditingPaneState>,
}

/// Defines what to do when showing the `EditingPane` from the `RoomInputBar`.
enum ShowEditingPaneBehavior {
    /// Show a new edit session, e.g., when first clicking "edit" on a message.
    ShowNew {
        event_tl_item: EventTimelineItem,
    },
    /// Restore the state of an `EditingPane` that already existed, e.g., when
    /// reopening a room that had an `EditingPane` open when it was closed.
    RestoreExisting {
        editing_pane_state: EditingPaneState,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_user_id(user_id: &str) -> OwnedUserId {
        user_id.try_into().unwrap()
    }

    #[test]
    fn translation_popup_position_prefers_above_button() {
        let button_rect = Rect {
            pos: dvec2(40.0, 4.0),
            size: dvec2(32.0, 32.0),
        };
        let container_rect = Rect {
            pos: dvec2(0.0, 640.0),
            size: dvec2(1280.0, 64.0),
        };
        let pass_size = dvec2(1280.0, 800.0);

        let pos = compute_translation_popup_abs_pos(button_rect, container_rect, pass_size);

        assert!(pos.y < button_rect.pos.y);
        assert!(pos.x >= 8.0);
        assert!(pos.x + 220.0 <= container_rect.size.x - 8.0);
    }

    #[test]
    fn translation_popup_position_falls_below_when_not_enough_space_above() {
        let button_rect = Rect {
            pos: dvec2(40.0, 4.0),
            size: dvec2(32.0, 32.0),
        };
        let container_rect = Rect {
            pos: dvec2(0.0, 10.0),
            size: dvec2(1280.0, 64.0),
        };
        let pass_size = dvec2(1280.0, 800.0);

        let pos = compute_translation_popup_abs_pos(button_rect, container_rect, pass_size);

        assert!(pos.y > button_rect.pos.y);
    }

    #[test]
    fn translation_popup_position_clamps_to_right_edge() {
        let button_rect = Rect {
            pos: dvec2(1260.0, 4.0),
            size: dvec2(32.0, 32.0),
        };
        let container_rect = Rect {
            pos: dvec2(0.0, 640.0),
            size: dvec2(1280.0, 64.0),
        };
        let pass_size = dvec2(1280.0, 800.0);

        let pos = compute_translation_popup_abs_pos(button_rect, container_rect, pass_size);

        assert_eq!(pos.x + 220.0, container_rect.size.x - 8.0);
    }

    #[test]
    fn translation_popup_position_can_resolve_to_negative_local_y() {
        let button_rect = Rect {
            pos: dvec2(40.0, 4.0),
            size: dvec2(32.0, 32.0),
        };
        let container_rect = Rect {
            pos: dvec2(0.0, 640.0),
            size: dvec2(1280.0, 64.0),
        };
        let pass_size = dvec2(1280.0, 800.0);

        let popup_pos = compute_translation_popup_abs_pos(button_rect, container_rect, pass_size);

        assert!(popup_pos.y < button_rect.pos.y);
        assert!(popup_pos.y < 0.0);
    }

    #[test]
    fn translation_apply_keeps_session_open() {
        let outcome = compute_translation_apply_outcome("Hola mundo");

        assert_eq!(outcome.input_text, "Hola mundo");
        assert_eq!(outcome.preserved_preview_text, "Hola mundo");
        assert_eq!(outcome.next_last_source, "Hola mundo");
        assert!(outcome.keep_preview_visible);
    }

    #[test]
    fn test_reply_to_human_no_bot_targeting() {
        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");
        let reply_sender = test_user_id("@alice:127.0.0.1:8128");

        assert_eq!(
            resolve_target(
                &ExplicitOverride::None,
                Some(reply_sender.as_ref()),
                Some(bound_bot_user_id.as_ref()),
                Some(bound_bot_user_id.as_ref()),
                &[],
            ),
            ResolvedTarget::RoomDefault(bound_bot_user_id),
        );
    }

    #[test]
    fn test_reply_bot_overrides_explicit_room() {
        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");

        assert_eq!(
            resolve_target(
                &ExplicitOverride::Room,
                Some(bound_bot_user_id.as_ref()),
                Some(bound_bot_user_id.as_ref()),
                Some(bound_bot_user_id.as_ref()),
                &[],
            ),
            ResolvedTarget::ReplyBot(bound_bot_user_id),
        );
    }

    #[test]
    fn test_chip_dismiss_returns_to_room_default() {
        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");

        assert_eq!(
            clear_explicit_override_result(Some(bound_bot_user_id.as_ref())),
            ResolvedTarget::RoomDefault(bound_bot_user_id),
        );
    }

    #[test]
    fn test_chip_dismiss_explicit_room_to_room_default() {
        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");

        let explicit_override = ExplicitOverride::Room;
        let cleared_override = explicit_override.cleared();

        assert_eq!(cleared_override, ExplicitOverride::None);
        assert_eq!(
            resolve_target(
                &cleared_override,
                None,
                Some(bound_bot_user_id.as_ref()),
                Some(bound_bot_user_id.as_ref()),
                &[],
            ),
            ResolvedTarget::RoomDefault(bound_bot_user_id),
        );
    }

    #[test]
    fn test_chip_dismiss_no_bound_bot() {
        assert_eq!(clear_explicit_override_result(None), ResolvedTarget::NoTarget);
    }

    #[test]
    fn test_explicit_bot_with_reply_to_human() {
        let bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");
        let reply_sender = test_user_id("@alice:127.0.0.1:8128");

        assert_eq!(
            resolved_target_user_id(&resolve_send_target(
                &ExplicitOverride::Bot(bot_user_id.clone()),
                Some(reply_sender.as_ref()),
                Some(bot_user_id.as_ref()),
                Some(bot_user_id.as_ref()),
                &[],
            )),
            Some(bot_user_id),
        );
    }

    #[test]
    fn test_cancel_reply_clears_reply_bot() {
        let bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");

        assert_eq!(
            resolve_restored_target(
                &ExplicitOverride::Bot(bot_user_id.clone()),
                Some(bot_user_id.as_ref()),
                Some(bot_user_id.as_ref()),
                Some(bot_user_id.as_ref()),
                &[],
            ),
            ResolvedTarget::ReplyBot(bot_user_id.clone()),
        );
        assert_eq!(
            resolve_restored_target(
                &ExplicitOverride::Bot(bot_user_id.clone()),
                None,
                Some(bot_user_id.as_ref()),
                Some(bot_user_id.as_ref()),
                &[],
            ),
            ResolvedTarget::ExplicitBot(bot_user_id),
        );
    }

    #[test]
    fn test_explicit_override_persists_navigation() {
        let saved_state = RoomInputBarState {
            explicit_override: ExplicitOverride::Room,
            ..Default::default()
        };

        assert_eq!(
            restored_explicit_override(&saved_state),
            ExplicitOverride::Room,
        );
    }

    #[test]
    fn test_reply_bot_restores_with_replying_to() {
        let bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");

        assert_eq!(
            resolve_restored_target(
                &ExplicitOverride::None,
                Some(bot_user_id.as_ref()),
                Some(bot_user_id.as_ref()),
                Some(bot_user_id.as_ref()),
                &[],
            ),
            ResolvedTarget::ReplyBot(bot_user_id),
        );
    }

    #[test]
    fn test_target_chip_room_default() {
        let bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");

        assert_eq!(
            format_target_chip_presentation(
                AppLanguage::English,
                &ResolvedTarget::RoomDefault(bot_user_id),
                Some("BotFather"),
            ),
            TargetChipPresentation {
                visible: true,
                label: "Default: BotFather".to_string(),
                subdued: true,
                dismissible: false,
            },
        );
    }

    #[test]
    fn test_target_chip_hidden_no_bot() {
        assert_eq!(
            format_target_chip_presentation(
                AppLanguage::English,
                &ResolvedTarget::NoTarget,
                None,
            ),
            TargetChipPresentation {
                visible: false,
                label: String::new(),
                subdued: false,
                dismissible: false,
            },
        );
    }

    #[test]
    fn test_explicit_bot_via_chip_menu() {
        let bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");
        let explicit_override = apply_target_menu_selection(
            TargetMenuSelection::BoundBot,
            Some(bot_user_id.as_ref()),
        );

        assert_eq!(explicit_override, ExplicitOverride::Bot(bot_user_id.clone()));
        assert_eq!(
            format_target_chip_presentation(
                AppLanguage::English,
                &resolve_target(
                    &explicit_override,
                    None,
                    Some(bot_user_id.as_ref()),
                    Some(bot_user_id.as_ref()),
                    &[],
                ),
                Some("BotFather"),
            ),
            TargetChipPresentation {
                visible: true,
                label: "To BotFather".to_string(),
                subdued: false,
                dismissible: true,
            },
        );
    }

    #[test]
    fn test_explicit_room_via_chip_menu() {
        let bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");
        let explicit_override = apply_target_menu_selection(
            TargetMenuSelection::Room,
            Some(bot_user_id.as_ref()),
        );

        assert_eq!(explicit_override, ExplicitOverride::Room);
        assert_eq!(
            format_target_chip_presentation(
                AppLanguage::English,
                &resolve_target(
                    &explicit_override,
                    None,
                    Some(bot_user_id.as_ref()),
                    Some(bot_user_id.as_ref()),
                    &[],
                ),
                Some("BotFather"),
            ),
            TargetChipPresentation {
                visible: true,
                label: "To room".to_string(),
                subdued: false,
                dismissible: true,
            },
        );
    }

    #[test]
    fn test_text_mentions_known_bot_matches_localpart() {
        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");
        let child_bot_user_id = test_user_id("@octosbot_alexbot:127.0.0.1:8128");

        assert!(text_mentions_known_bot(
            "@octosbot_alexbot 你是谁",
            Some(bound_bot_user_id.as_ref()),
            Some(bound_bot_user_id.as_ref()),
            std::slice::from_ref(&child_bot_user_id),
            std::slice::from_ref(&child_bot_user_id),
        ));
    }

    #[test]
    fn test_message_mentions_room_member_bot_with_empty_known_bot_list() {
        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");
        let room_member_bot_user_id = test_user_id("@octosbot_alexbot:127.0.0.1:8128");
        let message = RoomMessageEventContent::text_plain("@octosbot_alexbot 你是谁");

        assert!(message_mentions_known_bot(
            &message,
            Some(bound_bot_user_id.as_ref()),
            Some(bound_bot_user_id.as_ref()),
            std::slice::from_ref(&room_member_bot_user_id),
            &[],
        ));
    }

    #[test]
    fn test_message_mentions_known_bot_prefers_structured_mentions() {
        use ruma::events::Mentions;

        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");
        let child_bot_user_id = test_user_id("@octosbot_alexbot:127.0.0.1:8128");
        let message = RoomMessageEventContent::text_plain("你好")
            .add_mentions(Mentions::with_user_ids([child_bot_user_id.clone()]));

        assert!(message_mentions_known_bot(
            &message,
            Some(bound_bot_user_id.as_ref()),
            Some(bound_bot_user_id.as_ref()),
            std::slice::from_ref(&child_bot_user_id),
            std::slice::from_ref(&child_bot_user_id),
        ));
    }

    #[test]
    fn test_message_bot_mention_suppresses_explicit_bot_target() {
        let bound_bot_user_id = test_user_id("@octosbot:127.0.0.1:8128");

        assert_eq!(
            routing_directives_for_message(
                &ResolvedTarget::ExplicitBot(bound_bot_user_id),
                true,
            ),
            (None, false),
        );
    }

    #[test]
    fn test_message_bot_mention_keeps_explicit_room_marker() {
        assert_eq!(
            routing_directives_for_message(&ResolvedTarget::ExplicitRoom, true),
            (None, true),
        );
    }
}
