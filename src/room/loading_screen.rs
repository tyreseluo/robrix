use makepad_widgets::*;
use ruma::{OwnedRoomAliasId, OwnedRoomId, OwnedRoomOrAliasId, OwnedServerName};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::helpers::*;
    use crate::shared::styles::*;


    pub LoadingScreen = {{LoadingScreen}}<ScrollXYView> {
        width: Fill, height: Fill,
        flow: Down,
        align: {x: 0.5, y: 0.5},
        spacing: 10.0,

        show_bg: true,
        draw_bg: {
            color: (COLOR_PRIMARY_DARKER),
        }

        loading_spinner = <LoadingSpinner> {
            width: 60,
            height: 60,
            visible: true,
            draw_bg: {
                color: (COLOR_ACTIVE_PRIMARY)
                border_size: 4.0,
            }
        }

        title = <Label> {
            width: Fill, height: Fit,
            align: {x: 0.5, y: 0.0},
            padding: {left: 5.0, right: 0.0}
            margin: {top: 10.0},
            flow: RightWrap,
            draw_text: {
                color: (TYPING_NOTICE_TEXT_COLOR),
            }
            text: "Loading..."
        }

        details = <Label> {
            width: Fill, height: Fit,
            align: {x: 0.5, y: 0.0},
            padding: {left: 5.0, right: 0.0}
            margin: {top: 5.0},
            flow: RightWrap,
            draw_text: {
                color: (TYPING_NOTICE_TEXT_COLOR),
            }
            text: "Temporarily unavailable."
        }
    }
}

pub struct LoadingTabState {
    pub room_id: Option<OwnedRoomId>,
    pub room_alias_id: Option<OwnedRoomAliasId>,
    pub loading_type: LoadingType,
    pub loading_status: LoadingStatus,
    pub via: Vec<OwnedServerName>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum LoadingStatus {
    Loading,
    Requested,
    Loaded,
    Failed {
        title: Option<String>,
        details: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub enum LoadingType {
    /// Loading a specific room by its ID or alias.
    Room {
        room_or_alias_id: OwnedRoomOrAliasId,
        via: Vec<OwnedServerName>,
    },
    // Other loading types can be added here in the future.
}

#[derive(Clone, Debug, DefaultNone)]
pub enum RoomLoadingScreenAction {
    /// Show the loading screen for the given type.
    Loading(LoadingType),
    /// Hide the loading screen.
    Hide,
    /// Indicate that loading the room failed.
    Failed {
        title: Option<String>,
        details: Option<String>,
    },
    None,
}

#[derive(Live, LiveHook, Widget)]
pub struct LoadingScreen {
    #[deref] view: View,
}

impl Widget for LoadingScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}