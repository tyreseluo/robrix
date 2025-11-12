use makepad_widgets::*;
use ruma::{OwnedRoomAliasId, OwnedRoomId, OwnedRoomOrAliasId};

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

        loading_status_spinner = <LoadingSpinner> {
            width: 60,
            height: 60,
            visible: true,
            draw_bg: {
                color: (COLOR_ACTIVE_PRIMARY)
                border_size: 4.0,
            }
        }

        loading_status_title = <Label> {
            width: Fill, height: Fit,
            align: {x: 0.5, y: 0.0},
            padding: {left: 5.0, right: 0.0}
            margin: {top: 10.0},
            flow: RightWrap,
            draw_text: {
                color: (TYPING_NOTICE_TEXT_COLOR),
            }
        }

        loading_status_subtitle = <Label> {
            width: Fill, height: Fit,
            align: {x: 0.5, y: 0.0},
            padding: {left: 5.0, right: 0.0}
            margin: {top: 5.0},
            flow: RightWrap,
            draw_text: {
                color: (TYPING_NOTICE_TEXT_COLOR),
            }
        }
    }
}

#[derive(Clone, Debug, DefaultNone)]
pub enum RoomLoadingScreenAction {
    Loading(OwnedRoomOrAliasId),
    Failed { error_message: String },
    None,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum LoadingStatus {
    #[default]
    Loading,
    Failed {
        error_message: String,
        details: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct LoadingTabState {
    pub room_id: Option<OwnedRoomId>,
    pub room_alias_id: Option<OwnedRoomAliasId>,
    pub status: LoadingStatus,
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
        self.set_status(cx);
        self.view.draw_walk(cx, scope, walk)
    }
}

impl LoadingScreen {
    fn set_status(&mut self, cx: &mut Cx) {
        let spinner = self.view(ids!(loading_status_spinner));
        let title = self.label(ids!(loading_status_title));
        let subtitle = self.label(ids!(loading_status_subtitle));

        match &self.status {
            LoadingStatus::Loading => {
                spinner.set_visible(cx, true);
            }
            LoadingStatus::Failed { error_message, details } => {
                spinner.set_visible(cx, false);
                title.set_text(cx, error_message);

                if let Some(details) = details {
                    subtitle.set_visible(cx, true);
                    subtitle.set_text(cx, details);
                } else {
                    subtitle.set_visible(cx, false);
                }
            }
        }
    }

    pub fn set_loading(&mut self, cx: &mut Cx) {
        if self.status != LoadingStatus::Loading {
            self.status = LoadingStatus::Loading;
            self.redraw(cx);
        }
    }

    pub fn set_loading_with_message(&mut self, cx: &mut Cx, title: &str, sub_title: Option<String>) {
        self.status = LoadingStatus::Loading;
        self.label(ids!(loading_status_title)).set_text(cx, title);
        if let Some(sub_title) = sub_title {
            self.label(ids!(loading_status_subtitle)).set_text(cx, &sub_title);
            self.label(ids!(loading_status_subtitle)).set_visible(cx, true);
        } else {
            self.label(ids!(loading_status_subtitle)).set_visible(cx, false);
        }
        self.redraw(cx);
    }

    pub fn set_failed(&mut self, cx: &mut Cx, error_message: &str, details: Option<String>) {
        self.status = LoadingStatus::Failed {
            error_message: error_message.to_string(),
            details,
        };
        self.redraw(cx);
    }

    pub fn set_title(&mut self, cx: &mut Cx, title: &str) {
        self.label(ids!(loading_status_title)).set_text(cx, title);
        self.redraw(cx);
    }

    pub fn set_subtitle(&mut self, cx: &mut Cx, subtitle: &str) {
        let subtitle_label = self.label(ids!(loading_status_subtitle));
        if subtitle.is_empty() {
            subtitle_label.set_visible(cx, false);
        } else {
            subtitle_label.set_visible(cx, true);
            subtitle_label.set_text(cx, subtitle);
        }
        self.redraw(cx);
    }

    pub fn status(&self) -> &LoadingStatus {
        &self.status
    }
}

impl LoadingScreenRef {
    pub fn set_loading(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_loading(cx);
        }
    }

    pub fn set_loading_with_message(&self, cx: &mut Cx, title: &str, sub_title: Option<String>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_loading_with_message(cx, title, sub_title);
        }
    }

    pub fn set_failed(&self, cx: &mut Cx, error_message: &str, details: Option<String>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_failed(cx, error_message, details);
        }
    }

    pub fn set_title(&self, cx: &mut Cx, title: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_title(cx, title);
        }
    }

    pub fn set_subtitle(&self, cx: &mut Cx, subtitle: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_subtitle(cx, subtitle);
        }
    }
}