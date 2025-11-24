use makepad_widgets::*;

use crate::{home::room_screen::RoomScreenWidgetRefExt, room::FetchedRoomPreview};


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

        not_previewable_view = <RoomNotPreviewableView> {}
        
        room_screen_wrapper = <View> {
            visible: false,
            room_screen = <RoomScreen> {}
        }
    }
}

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

impl PreviewScreen {
    pub fn set_displayed_preview(&mut self, cx: &mut Cx, room_preview_info: FetchedRoomPreview) {
        let room_screen_wrapper_ref = self.view.view(ids!(room_screen_wrapper));
        let not_previewable_view_ref = self.view.view(ids!(not_previewable_view));
    
        // Least privilege, default to false if not specified, same with Matrix Spec.
        let is_world_readable = room_preview_info.is_world_readable.unwrap_or(false);
        log!("is world readable: {}", is_world_readable);
        
        if is_world_readable {
            room_screen_wrapper_ref.set_visible(cx, true);
            room_screen_wrapper_ref.room_screen(ids!(room_screen)).set_displayed_room(
                cx, 
                room_preview_info.room_id.clone(), 
                room_preview_info.name.clone()
            );
            not_previewable_view_ref.set_visible(cx, false);
        } else {
            room_screen_wrapper_ref.set_visible(cx, false);
            not_previewable_view_ref.set_visible(cx, true);
        }
        self.redraw(cx);
    }
}

impl PreviewScreenRef {
    pub fn set_displayed_preview(&mut self, cx: &mut Cx, room_preview_info: FetchedRoomPreview) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_displayed_preview(cx, room_preview_info);
        }
    }
}
