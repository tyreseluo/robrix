use std::sync::Arc;

use crossbeam_channel::Sender;
use makepad_widgets::*;
use matrix_sdk_ui::Timeline;
use ruma::{OwnedRoomId, api::client::message::get_message_events, events::AnyTimelineEvent, room::JoinRuleSummary};
use tokio::sync::watch;

use crate::{home::room_screen::{RoomScreenWidgetRefExt, TimelineUpdate}, room::{FetchedRoomAvatar, FetchedRoomPreview}, shared::avatar::AvatarWidgetRefExt, sliding_sync::{BackwardsPaginateUntilEventRequest, MatrixRequest, get_client, submit_async_request}, utils};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::helpers::*;
    use crate::shared::styles::*;
    use crate::shared::avatar::Avatar;
    use crate::home::room_screen::RoomScreen;
    use crate::shared::icon_button::RobrixIconButton;

    RoomNotPreviewableView = <View> {
        width: Fill,
        height: Fill,
        flow: Down,
        align: {x: 0.5, y: 0.5}
        padding: {left: 20, right: 20, top: 50}
        spacing: 0,

        show_bg: true,
        draw_bg: {
            color: (COLOR_PRIMARY_DARKER),
        }

        <View> {
            width: Fill, height: Fit,
            flow: Down,
            align: {x: 0.5, y: 0.5},

            room_avatar = <Avatar> {
                width: 100,
                height: 100,

                text_view = { text = { draw_text: {
                    text_style: <TITLE_TEXT>{ font_size: 32.0 }
                }}}
            }

            room_name = <Label> {
                width: Fill, height: Fit,
                align: {x: 0.5, y: 0},
                text: ""
                margin: {top: 15}
                flow: RightWrap,
                draw_text: {
                    text_style: <TITLE_TEXT>{
                        font_size: 18,
                    },
                    color: #000
                    wrap: Word,
                }
            }

            members_count = <Label> {
                draw_text: {
                    text_style: <REGULAR_TEXT> {},
                    color: #999,
                }
                text: ""
            }
        }

        <View> {
            width: Fill, height: Fit,
            flow: Down,
            align: {x: 0.5, y: 0.5},
            padding: {left: 20.0, right: 20.0},

            <Label> {
                draw_text: {
                    text_style: <REGULAR_TEXT> {},
                    color: #999,
                }
                text: "You cannot preview this room"
            }
        }

        room_topic = <Label> {
            visible: false,
            width: Fill, height: Fit,
            padding: {left: 20.0, right: 20.0, top: 10.0},
            draw_text: {
                text_style: <REGULAR_TEXT> {},
                color: #ccc,
                wrap: Word,
            }
            text: ""
        }

        <View> {
            width: Fit, height: Fit,
            flow: Down,
            align: {x: 0.5, y: 0.5},
            spacing: 10.0,

            knock_button = <RobrixIconButton> {
                visible: false,
                width: 200.0,
                height: 40.0,
                draw_bg: {
                    color: #2196F3,
                }
                text: "Request to Join"
            }

            join_button = <RobrixIconButton> {
                visible: false,
                width: 100,
                align: {x: 0.5, y: 0.5}
                padding: 15,
                draw_bg: {
                    border_color: (COLOR_FG_ACCEPT_GREEN),
                    color: (COLOR_BG_ACCEPT_GREEN)
                }
                text: "Join Room"
                draw_text:{
                    color: (COLOR_FG_ACCEPT_GREEN),
                }
            }

            join_info = <Label> {
                visible: false,
                draw_text: {
                    text_style: <REGULAR_TEXT> {},
                    color: #999,
                }
                text: ""
            }
        }
    }

    pub PreviewScreen = {{PreviewScreen}}<ScrollXYView> {
        width: Fill, height: Fill,
        flow: Down,
        align: {x: 0.5, y: 0.5},
        spacing: 10.0,

        show_bg: true,
        draw_bg: {
            color: (COLOR_PRIMARY_DARKER),
        }

        room_screen = <RoomScreen> {
            visible: false,
        }

        not_previewable_view = <RoomNotPreviewableView> {
            visible: false,
        }
    }
}

pub struct PreviewRoomDetails {
    #[allow(unused)]
    pub room_id: OwnedRoomId,
    pub timeline: Arc<Timeline>,
    /// Background → UI updates
    pub timeline_update_sender: crossbeam_channel::Sender<PreviewTimelineUpdate>,
    /// UI → Background pagination/initial requests
    pub request_sender: watch::Sender<RoomPreviewRequest>,
    /// Background handler task (abort on drop)
    pub preview_handler_task: tokio::task::JoinHandle<()>,
}

impl Drop for PreviewRoomDetails {
    fn drop(&mut self) {
        log!("Dropping PreviewRoomDetails for room {}", self.room_id);
        self.preview_handler_task.abort();
    }
}

#[derive(Debug, Clone)]
pub enum RoomPreviewRequest {
    Idle,
    InitialLoad { limit: u16 },
    BackwaredPagination {
        room: String,
        limit: u16
    },
}

/// Updates that can be sent to a preview timeline.
#[derive(Debug, Clone)]
pub enum PreviewTimelineUpdate {
    /// Initial load of events for the preview.
    InitialEvents {
        events: Vec<AnyTimelineEvent>,
        prev_batch: Option<String>,
    },
    OlderEvents {
        events: Vec<AnyTimelineEvent>,
        prev_batch: Option<String>,
    },
    /// An error occurred while fetching preview events.
    FetchError {
        error: String,
    },
    /// Loading state changed.
    Loading(bool),
}

// pub async fn run_preview_timeline_task(
//     room_id: OwnedRoomId,
//     update_sender: Sender<TimelineUpdate>,
//     mut request_receiver: watch::Receiver<Vec<BackwardsPaginateUntilEventRequest>>,
// ) {
//     todo!()
// }

// pub async fn fetch_preview_room_events(
//     room_id: OwnedRoomId,
//     limit: u16,
//     update_sender: Sender<PreviewTimelineUpdate>,
// ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

//     let Some(client) = get_client() else {
//         let error_msg = "Matrix client is not available".to_string();
//         let _ = update_sender.send(PreviewTimelineUpdate::FetchError {
//             error: error_msg.clone()
//         });
//         return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_msg)));
//     };

//     log!("Fetching preview events for room: {}", room_id);

//     // Send loading state
//     let _ = update_sender.send(PreviewTimelineUpdate::Loading(true));

//     // Build the request to fetch room events
//     let mut request = get_message_events::v3::Request::backward(
//         room_id.clone(),
//     );
//     request.limit = limit.into();

//     match client.send(request).await {
//         Ok(response) => {
//             log!("Successfully fetched {} events for preview room {}",
//                 response.chunk.len(), room_id);

//             // Convert the raw events into AnyTimelineEvent
//             let events: Vec<AnyTimelineEvent> = response.chunk
//                 .into_iter()
//                 .filter_map(|raw_event| {
//                     match raw_event.deserialize() {
//                         Ok(event) => Some(event),
//                         Err(e) => {
//                             log!("Failed to deserialize event: {:?}", e);
//                             None
//                         }
//                     }
//                 })
//                 .collect();

//             // Send the events to the UI
//             let _ = update_sender.send(PreviewTimelineUpdate::InitialEvents { events });
//             let _ = update_sender.send(PreviewTimelineUpdate::Loading(false));

//             Ok(())
//         }
//         Err(e) => {
//             error!("Failed to fetch preview events for room {}: {:?}", room_id, e);
//             let error_msg = format!("Failed to fetch room events: {}", e);
//             let _ = update_sender.send(PreviewTimelineUpdate::FetchError {
//                 error: error_msg.clone()
//             });
//             let _ = update_sender.send(PreviewTimelineUpdate::Loading(false));

//             Err(Box::new(e))
//         }
//     }
// }

/// Converts raw Matrix events into TimelineItems for display.
///
/// This is a simplified version that creates basic timeline items
/// without all the features of a full timeline (no reactions, edits, etc.)
// pub fn convert_events_to_timeline_items(
//     events: Vec<AnyTimelineEvent>,
//     room_id: &OwnedRoomId,
// ) -> Vector<Arc<TimelineItem>> {
//     let mut items = Vector::new();

//     // Add a timeline start marker
//     items.push_back(Arc::new(
//         TimelineItemKind::Virtual(VirtualTimelineItem::TimelineStart)
//     ));

//     // Process events in chronological order (oldest first)
//     for event in events.into_iter().rev() {
//         match event {
//             AnyTimelineEvent::MessageLike(msg_like) => {
//                 // Try to create a timeline item from the message-like event
//                 // Note: This is simplified - in a real implementation you'd need
//                 // to properly handle all event types and create proper EventTimelineItems

//                 // For now, we'll create a simplified text representation
//                 // In a full implementation, you'd use matrix_sdk_ui::timeline::TimelineBuilder
//                 log!("Processing message-like event: {:?}", msg_like);

//                 // TODO: Convert AnyMessageLikeEvent to TimelineItem
//                 // This requires using the Matrix SDK's timeline builder APIs
//                 // which may not be directly accessible for preview mode.
//                 // We might need to create a minimal TimelineItem wrapper.
//             }
//             AnyTimelineEvent::State(state) => {
//                 log!("Processing state event: {:?}", state);
//                 // TODO: Handle state events
//             }
//         }
//     }

//     // TODO:
//     items
// }

#[derive(Live, LiveHook, Widget)]
pub struct PreviewScreen {
    #[deref] view: View,
}

impl Widget for PreviewScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl PreviewScreen {}

impl PreviewScreenRef {
    pub fn show_room_can_preview(&mut self, cx: &mut Cx, preview_data: &FetchedRoomPreview) {
        log!("Setting displayed room preview: {:?}", preview_data);

        self.view(ids!(not_previewable_view)).set_visible(cx, false);

        let room_screen = self.room_screen(ids!(room_screen));
        room_screen.set_visible(cx, true);

        room_screen.set_displayed_preview_room(
            cx,
            preview_data.room_id.clone(),
            preview_data.name.clone()
        );
        self.redraw(cx);
    }

    pub fn show_room_can_not_preview(&mut self, cx: &mut Cx, preview_data: &FetchedRoomPreview) {
        log!("Setting displayed room can not preview: {:?}", preview_data);

        self.view(ids!(room_screen)).set_visible(cx, false);

        let not_previewable_view = self.view(ids!(not_previewable_view));
        not_previewable_view.set_visible(cx, true);

        let avatar = not_previewable_view.avatar(ids!(room_avatar));
        match &preview_data.room_avatar {
            FetchedRoomAvatar::Text(text) => {
                avatar.show_text(cx, None, None, &text);
            }
            FetchedRoomAvatar::Image(img_data) => {
                let _ = avatar.show_image(cx, None, |cx, img| {
                    utils::load_png_or_jpg(&img, cx, &img_data)
                });
            }
        }

        let room_name = preview_data.name.as_deref().unwrap_or("Unnamed Room");
        not_previewable_view.label(ids!(room_name))
            .set_text(cx, room_name);

        let room_id_text = if let Some(alias) = &preview_data.canonical_alias {
            alias.to_string()
        } else {
            preview_data.room_id.to_string()
        };
        not_previewable_view.label(ids!(room_id))
            .set_text(cx, &room_id_text);

        let members_text = if let Some(active) = preview_data.num_active_members {
            format!("{} active members", active)
        } else {
            format!("{} members", preview_data.num_joined_members)
        };
        not_previewable_view.label(ids!(members_count))
            .set_text(cx, &members_text);

        if let Some(topic) = &preview_data.topic {
            let topic_label = not_previewable_view.label(ids!(room_topic));
            topic_label.set_visible(cx, true);
            topic_label.set_text(cx, topic);
        }

        self.configure_join_buttons(cx, not_previewable_view, &preview_data.join_rule);

        self.redraw(cx);
    }

    fn configure_join_buttons(&mut self, cx: &mut Cx, not_previewable_view: ViewRef, join_rule: &Option<JoinRuleSummary>) {
        not_previewable_view.button(ids!(knock_button)).set_visible(cx, false);
        not_previewable_view.button(ids!(join_button)).set_visible(cx, false);
        not_previewable_view.label(ids!(join_info)).set_visible(cx, false);

        if let Some(rule) = join_rule {
            match rule {
                JoinRuleSummary::Public => {
                    not_previewable_view.button(ids!(join_button)).set_visible(cx, true);
                }
                JoinRuleSummary::Knock => {
                    not_previewable_view.button(ids!(knock_button)).set_visible(cx, true);
                }
                JoinRuleSummary::KnockRestricted(_) => {
                    not_previewable_view.button(ids!(knock_button)).set_visible(cx, true);
                }
                JoinRuleSummary::Restricted(_) => {
                    not_previewable_view.button(ids!(join_button)).set_visible(cx, true);
                    let info_label = not_previewable_view.label(ids!(join_info));
                    info_label.set_visible(cx, true);
                    info_label.set_text(cx, "You may need to meet certain conditions to join");
                }
                JoinRuleSummary::Invite => {
                    let info_label = not_previewable_view.label(ids!(join_info));
                    info_label.set_visible(cx, true);
                    info_label.set_text(cx, "You need an invitation to join this room");
                }
                JoinRuleSummary::Private => {
                    let info_label = not_previewable_view.label(ids!(join_info));
                    info_label.set_visible(cx, true);
                    info_label.set_text(cx, "This is a private room");
                }
                _ => {
                    let info_label = not_previewable_view.label(ids!(join_info));
                    info_label.set_visible(cx, true);
                    info_label.set_text(cx, "Contact the room administrator for access");
                }
            }
        } else {
            let info_label = not_previewable_view.label(ids!(join_info));
            info_label.set_visible(cx, true);
            info_label.set_text(cx, "Join rules unknown");
        }
    }
}