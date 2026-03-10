use makepad_widgets::*;

use crate::{
    crew::{self, CrewSettingsAction},
    home::room_screen::RoomScreenProps,
    login::login_screen::LoginAction,
    logout::logout_confirm_modal::LogoutAction,
};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::helpers::*;
    use crate::shared::styles::*;
    use crate::shared::icon_button::*;

    pub CrewRoomPanel = {{CrewRoomPanel}} {
        width: Fill, height: Fit
        flow: Down
        spacing: 10
        margin: {left: 6, right: 6, top: 6, bottom: 16}
        padding: 12

        header = <View> {
            width: Fill, height: Fit
            flow: Right
            align: {x: 1.0, y: 0.5}
            spacing: 10

            <Label> {
                width: Fill, height: Fit
                draw_text: {
                    text_style: <REGULAR_TEXT>{font_size: 11.0}
                    color: (COLOR_TEXT)
                }
                text: "Crew Room Panel"
            }

            close_button = <RobrixIconButton> {
                width: Fit
                padding: 9
                spacing: 0
                draw_bg: {
                    color: (COLOR_SECONDARY)
                }
                draw_icon: {
                    svg_file: (ICON_CLOSE)
                    color: (COLOR_TEXT)
                }
                icon_walk: { width: 12, height: 12 }
                text: ""
            }
        }

        show_bg: true
        draw_bg: {
            color: (COLOR_BG_PREVIEW)
            border_radius: 4.0
            border_size: 1.0
            border_color: (COLOR_SECONDARY)
        }

        <Label> {
            width: Fill, height: Fit
            flow: RightWrap
            draw_text: {
                wrap: Word
                text_style: <REGULAR_TEXT>{font_size: 10.5}
                color: (COLOR_TEXT)
            }
            text: "This panel now only manages room binding and endpoint checks. Crew replies default to the current room in the runtime flow. Thread-specific send is not exposed in this UI."
        }

        binding_summary_label = <Label> {
            width: Fill, height: Fit
            flow: RightWrap
            draw_text: {
                wrap: Word
                text_style: <REGULAR_TEXT>{font_size: 10.5}
                color: (COLOR_TEXT)
            }
            text: "No Crew binding resolved yet."
        }

        <View> {
            width: Fill, height: Fit
            flow: RightWrap
            spacing: 10

            bind_room_button = <RobrixIconButton> {
                width: Fit
                padding: 12
                draw_bg: {
                    color: (COLOR_BG_ACCEPT_GREEN)
                    border_color: (COLOR_FG_ACCEPT_GREEN)
                }
                draw_icon: {
                    svg_file: (ICON_LINK)
                    color: (COLOR_FG_ACCEPT_GREEN)
                }
                draw_text: {
                    color: (COLOR_FG_ACCEPT_GREEN)
                }
                icon_walk: { width: 14, height: 14 }
                text: "Bind Room"
            }

            unbind_room_button = <RobrixIconButton> {
                width: Fit
                padding: 12
                draw_bg: {
                    color: (COLOR_BG_DANGER_RED)
                    border_color: (COLOR_FG_DANGER_RED)
                }
                draw_icon: {
                    svg_file: (ICON_FORBIDDEN)
                    color: (COLOR_FG_DANGER_RED)
                }
                draw_text: {
                    color: (COLOR_FG_DANGER_RED)
                }
                icon_walk: { width: 14, height: 14 }
                text: "Unbind Room"
            }

            healthcheck_button = <RobrixIconButton> {
                width: Fit
                padding: 12
                draw_bg: {
                    color: (COLOR_SECONDARY)
                }
                draw_icon: {
                    svg_file: (ICON_INFO)
                    color: (COLOR_TEXT)
                }
                icon_walk: { width: 14, height: 14 }
                text: "Healthcheck"
            }
        }

        status_label = <Label> {
            width: Fill, height: Fit
            flow: RightWrap
            draw_text: {
                wrap: Word
                text_style: <REGULAR_TEXT>{font_size: 10.5}
                color: (COLOR_TEXT)
            }
            text: ""
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct CrewRoomPanel {
    #[deref]
    view: View,
}

#[derive(Clone, Debug, DefaultNone)]
pub enum CrewRoomPanelAction {
    CloseRequested,
    None,
}

impl Widget for CrewRoomPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if matches!(event, Event::Signal) {
            let _ = crew::ensure_loaded_for_current_user();
            let _ = crew::refresh_inventory_from_rooms_list(cx);
            self.refresh_room_state(cx, current_room_id(scope));
        }

        if let Event::Actions(actions) = event {
            let current_room_id = current_room_id(scope);
            let bind_room_button = self.view.button(ids!(bind_room_button));
            let unbind_room_button = self.view.button(ids!(unbind_room_button));
            let healthcheck_button = self.view.button(ids!(healthcheck_button));
            let close_button = self.view.button(ids!(close_button));

            if bind_room_button.clicked(actions) {
                match current_room_id.as_deref() {
                    Some(room_id) => match crew::bind_room_to_default(room_id) {
                        Ok(()) => {
                            self.refresh_room_state(cx, current_room_id.clone());
                            self.set_status(
                                cx,
                                "Bound the current room to the default Crew profile.",
                            );
                        }
                        Err(error) => self.set_status(cx, &error),
                    },
                    None => self.set_status(cx, "This room is not ready for Crew yet."),
                }
            }

            if unbind_room_button.clicked(actions) {
                match current_room_id.as_deref() {
                    Some(room_id) => match crew::unbind_room(room_id) {
                        Ok(()) => {
                            self.refresh_room_state(cx, current_room_id.clone());
                            self.set_status(cx, "Removed the Crew room binding.");
                        }
                        Err(error) => self.set_status(cx, &error),
                    },
                    None => self.set_status(cx, "This room is not ready for Crew yet."),
                }
            }

            if healthcheck_button.clicked(actions) {
                match current_room_id.clone() {
                    Some(room_id) => match crew::run_room_healthcheck(room_id) {
                        Ok(()) => self.set_status(cx, "Running Crew healthcheck..."),
                        Err(error) => self.set_status(cx, &error),
                    },
                    None => self.set_status(cx, "This room is not ready for Crew yet."),
                }
            }

            if close_button.clicked(actions) {
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    CrewRoomPanelAction::CloseRequested,
                );
            }

            for action in actions {
                if let Some(LoginAction::LoginSuccess) = action.downcast_ref() {
                    let _ = crew::ensure_loaded_for_current_user();
                    self.refresh_room_state(cx, current_room_id.clone());
                    continue;
                }

                if let Some(LogoutAction::ClearAppState { .. }) = action.downcast_ref() {
                    self.clear(cx);
                    continue;
                }

                if let Some(CrewSettingsAction::StateChanged) = action.downcast_ref() {
                    self.refresh_room_state(cx, current_room_id.clone());
                    continue;
                }

                if let Some(CrewSettingsAction::Status(status)) = action.downcast_ref() {
                    self.set_status(cx, status);
                    continue;
                }

                if let Some(CrewSettingsAction::HealthcheckFinished { room_id, result }) =
                    action.downcast_ref()
                {
                    if current_room_id.as_deref() == Some(room_id.as_str()) {
                        match result {
                            Ok(message) => self.set_status(cx, message),
                            Err(error) => self.set_status(cx, error),
                        }
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl CrewRoomPanel {
    fn refresh_room_state(&mut self, cx: &mut Cx, room_id: Option<String>) {
        match room_id {
            Some(room_id) => {
                self.view
                    .label(ids!(binding_summary_label))
                    .set_text(cx, &crew::describe_room_binding(&room_id));
            }
            None => {
                self.view.label(ids!(binding_summary_label)).set_text(
                    cx,
                    "This room is not ready yet. Open it again after Robrix finishes loading.",
                );
            }
        }
    }

    fn clear(&mut self, cx: &mut Cx) {
        self.view
            .label(ids!(binding_summary_label))
            .set_text(cx, "No Crew binding resolved yet.");
        self.view.label(ids!(status_label)).set_text(cx, "");
    }

    fn set_status(&mut self, cx: &mut Cx, status: &str) {
        self.view.label(ids!(status_label)).set_text(cx, status);
    }
}

fn current_room_id(scope: &mut Scope) -> Option<String> {
    scope
        .props
        .get::<RoomScreenProps>()
        .map(|props| props.room_name_id.room_id().to_string())
}
