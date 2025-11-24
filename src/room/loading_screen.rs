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

#[derive(Clone, Debug, DefaultNone)]
pub enum RoomLoadingScreenAction {
    Loading(LoadingType),
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum LoadingStatus {
    #[default]
    Loading,
    Success,
    Failed {
        title: Option<String>,
        details: Option<String>,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadingType {
    /// Preview (a room or space).
    Preview {
        room_or_alias_id: OwnedRoomOrAliasId,
        via: Vec<OwnedServerName>,
    },
}

#[derive(Clone, Debug)]
pub struct LoadingTabState {
    /// The ID of the room, if it exists.
    pub room_id: Option<OwnedRoomId>,
    /// The alias of the room, if it exists.
    pub room_alias_id: Option<OwnedRoomAliasId>,
    /// The servers that the room is known to be on, if it exists.
    pub via: Vec<OwnedServerName>,
    /// The type of loading screen.
    pub loading_type: LoadingType,
    /// The status of the loading screen.
    pub status: LoadingStatus
}

#[derive(Live, LiveHook, Widget)]
pub struct LoadingScreen {
    #[deref] view: View,
    
    #[rust] status: LoadingStatus,
}

impl Widget for LoadingScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let loading_spinner_ref = self.view.view(ids!(loading_spinner));
        let title_ref = self.view.label(ids!(title));
        let details_ref = self.view.label(ids!(details));
        
        match &self.status {
            LoadingStatus::Loading => {
                title_ref.set_text(cx, "Loading...");
                details_ref.set_text(cx, "Please wait while we load the room.");
            }
            LoadingStatus::Failed { 
                title,
                details,
            } => {
                loading_spinner_ref.set_visible(cx, false);
                if let Some(title) = title {
                    title_ref.set_text(cx, &title);
                } else {
                    title_ref.set_text(cx, "Loading Failed");
                }
                if let Some(details) = details {
                    details_ref.set_text(cx, &details);
                } else {
                    details_ref.set_text(cx, "An error occurred while loading the room.");
                }
            }
            _ => {}
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl LoadingScreen {
    pub fn set_error_status(&mut self, cx: &mut Cx, title: Option<String>, details: Option<String>) {
        self.status = LoadingStatus::Failed { 
            title,
            details,
        };
        self.redraw(cx);
    }
}

impl LoadingScreenRef {
    pub fn set_error_status(&mut self, cx: &mut Cx, title: Option<String>, details: Option<String>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_error_status(cx, title, details);
        }
    }
}