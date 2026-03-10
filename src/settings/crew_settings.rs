use makepad_widgets::*;

use crate::{
    crew::{self, CrewSettingsAction},
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

    pub CrewSettings = {{CrewSettings}} {
        width: Fill, height: Fit
        flow: Down
        spacing: 10

        <TitleLabel> {
            text: "Crew Settings"
        }

        <Label> {
            width: Fill, height: Fit
            flow: RightWrap
            draw_text: {
                wrap: Word
                text_style: <REGULAR_TEXT>{font_size: 10.5}
                color: (COLOR_TEXT)
            }
            text: "Configure the shared Crew endpoint, auth token env var, and workspace root used by Robrix. Room-specific bind, healthcheck, and prompt execution now live in each room page."
        }

        <SubsectionLabel> {
            text: "Default Crew Endpoint"
        }

        endpoint_input = <SimpleTextInput> {
            width: Fill, height: Fit
            empty_text: "http://127.0.0.1:8000"
        }

        auth_token_env_input = <SimpleTextInput> {
            width: Fill, height: Fit
            empty_text: "Optional bearer token env var, e.g. CREW_API_TOKEN"
        }

        workspace_root_input = <SimpleTextInput> {
            width: Fill, height: Fit
            empty_text: "/path/to/workspace (optional)"
        }

        <View> {
            width: Fill, height: Fit
            flow: RightWrap
            spacing: 10

            refresh_inventory_button = <RobrixIconButton> {
                width: Fit
                padding: 12
                draw_bg: {
                    color: (COLOR_SECONDARY)
                }
                draw_icon: {
                    svg_file: (ICON_ROTATE_CW)
                    color: (COLOR_TEXT)
                }
                icon_walk: { width: 14, height: 14 }
                text: "Refresh Inventory"
            }

            save_defaults_button = <RobrixIconButton> {
                width: Fit
                padding: 12
                draw_bg: {
                    color: (COLOR_ACTIVE_PRIMARY)
                }
                draw_icon: {
                    svg_file: (ICON_CHECKMARK)
                    color: (COLOR_PRIMARY)
                }
                draw_text: {
                    color: (COLOR_PRIMARY)
                }
                icon_walk: { width: 14, height: 14 }
                text: "Save Default Profile"
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
pub struct CrewSettings {
    #[deref]
    view: View,
}

impl Widget for CrewSettings {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if matches!(event, Event::Signal) {
            let _ = crew::refresh_inventory_from_rooms_list(cx);
        }

        if let Event::Actions(actions) = event {
            let refresh_inventory_button = self.view.button(ids!(refresh_inventory_button));
            let save_defaults_button = self.view.button(ids!(save_defaults_button));
            let endpoint_input = self.view.text_input(ids!(endpoint_input));
            let auth_token_env_input = self.view.text_input(ids!(auth_token_env_input));
            let workspace_root_input = self.view.text_input(ids!(workspace_root_input));

            if refresh_inventory_button.clicked(actions) {
                match crew::refresh_inventory_from_rooms_list(cx) {
                    Ok(true) => {
                        self.set_status(cx, "Crew inventory refreshed from the current room list.");
                    }
                    Ok(false) => self.set_status(cx, "Crew inventory was already up to date."),
                    Err(error) => self.set_status(cx, &error),
                }
            }

            if save_defaults_button.clicked(actions) {
                match crew::save_default_profile(
                    &endpoint_input.text(),
                    &auth_token_env_input.text(),
                    &workspace_root_input.text(),
                ) {
                    Ok(()) => {
                        self.populate(cx, None);
                        self.set_status(cx, "Saved the default Crew profile.");
                    }
                    Err(error) => self.set_status(cx, &error),
                }
            }

            for action in actions {
                if let Some(LoginAction::LoginSuccess) = action.downcast_ref() {
                    let _ = crew::ensure_loaded_for_current_user();
                    self.populate(cx, None);
                    continue;
                }

                if let Some(LogoutAction::ClearAppState { .. }) = action.downcast_ref() {
                    self.view.text_input(ids!(endpoint_input)).set_text(cx, "");
                    self.view
                        .text_input(ids!(auth_token_env_input))
                        .set_text(cx, "");
                    self.view
                        .text_input(ids!(workspace_root_input))
                        .set_text(cx, "");
                    self.view.label(ids!(status_label)).set_text(cx, "");
                    continue;
                }

                if let Some(CrewSettingsAction::StateChanged) = action.downcast_ref() {
                    self.populate(cx, None);
                    continue;
                }

                if let Some(CrewSettingsAction::Status(status)) = action.downcast_ref() {
                    self.set_status(cx, status);
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl CrewSettings {
    pub fn populate(&mut self, cx: &mut Cx, _selected_room_id: Option<String>) {
        let _ = crew::ensure_loaded_for_current_user();
        let _ = crew::refresh_inventory_from_rooms_list(cx);
        let defaults = crew::default_config_form();
        self.view
            .text_input(ids!(endpoint_input))
            .set_text(cx, &defaults.endpoint);
        self.view
            .text_input(ids!(auth_token_env_input))
            .set_text(cx, &defaults.auth_token_env);
        self.view
            .text_input(ids!(workspace_root_input))
            .set_text(cx, &defaults.workspace_root);
    }

    fn set_status(&mut self, cx: &mut Cx, status: &str) {
        self.view.label(ids!(status_label)).set_text(cx, status);
    }
}

impl CrewSettingsRef {
    pub fn populate(&self, cx: &mut Cx, selected_room_id: Option<String>) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.populate(cx, selected_room_id);
    }
}
