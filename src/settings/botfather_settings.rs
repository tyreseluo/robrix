use makepad_widgets::*;
use robrix_botfather::RuntimeKind;

use crate::{
    botfather::{self, BotfatherAction, commands::help_text},
    login::login_screen::LoginAction,
    logout::logout_confirm_modal::LogoutAction,
};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::helpers::*;
    use crate::shared::icon_button::*;
    use crate::shared::styles::*;

    ICON_COLLAPSE = dep("crate://self/resources/icons/triangle_fill.svg")

    SettingsCard = <RoundedView> {
        width: Fill,
        height: Fit,
        flow: Down,
        spacing: 8,
        padding: 14,
        show_bg: true,
        draw_bg: {
            color: (COLOR_BG_PREVIEW)
            border_radius: 6.0,
            border_size: 1.0
            border_color: (COLOR_SECONDARY)
        }
    }

    SummaryLabel = <Label> {
        width: Fill,
        height: Fit,
        flow: RightWrap
        draw_text: {
            wrap: Word
            text_style: <REGULAR_TEXT>{font_size: 10.5}
            color: (COLOR_TEXT)
        }
        text: ""
    }

    SectionButton = <RobrixIconButton> {
        width: Fit
        padding: 11
        draw_bg: {
            color: (COLOR_BG_PREVIEW)
        }
        draw_icon: {
            svg_file: (ICON_LINK)
            color: (COLOR_TEXT)
        }
        icon_walk: { width: 14, height: 14 }
    }

    RuntimeToggleButton = <RobrixIconButton> {
        width: Fill
        padding: 12
        spacing: 10
        draw_bg: {
            color: (COLOR_BG_PREVIEW)
            border_radius: 6.0
            border_size: 1.0
            border_color: (COLOR_SECONDARY)
        }
        draw_icon: {
            svg_file: (ICON_COLLAPSE)
            rotation_angle: 90.0
            color: (COLOR_TEXT)
        }
        draw_text: {
            color: (COLOR_TEXT)
        }
        icon_walk: { width: 12, height: 12 }
    }

    RuntimeForm = <View> {
        visible: false
        width: Fill, height: Fit
        flow: Down
        spacing: 8
    }

    pub BotfatherSettings = {{BotfatherSettings}} {
        width: Fill, height: Fit
        flow: Down
        spacing: 12
        margin: {bottom: 24}

        <TitleLabel> {
            text: "BotFather Settings"
        }

        <SummaryLabel> {
            text: "BotFather now groups its controls by cards. Use Runtimes to configure transports, Bots to inspect what is installed, and Workspace for shared inventory and root binding."
        }

        section_selector = <View> {
            width: Fill, height: Fit
            flow: RightWrap
            spacing: 10

            runtimes_section_button = <SectionButton> {
                text: "Runtimes"
            }

            bots_section_button = <SectionButton> {
                draw_icon: {
                    svg_file: (ICON_INFO)
                    color: (COLOR_TEXT)
                }
                text: "Bots"
            }

            workspace_section_button = <SectionButton> {
                draw_icon: {
                    svg_file: (ICON_CHECKMARK)
                    color: (COLOR_TEXT)
                }
                text: "Workspace"
            }
        }

        runtimes_section = <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 12

            crew_runtime_card = <SettingsCard> {
                crew_runtime_toggle = <RuntimeToggleButton> {
                    text: "Crew Runtime"
                }

                crew_runtime_summary_label = <SummaryLabel> {}

                crew_runtime_form = <RuntimeForm> {
                    crew_endpoint_input = <SimpleTextInput> {
                        width: Fill, height: Fit
                        empty_text: "http://127.0.0.1:8000"
                    }

                    crew_auth_token_env_input = <SimpleTextInput> {
                        width: Fill, height: Fit
                        empty_text: "Optional bearer token env var, e.g. CREW_API_TOKEN"
                    }
                }
            }

            openclaw_runtime_card = <SettingsCard> {
                openclaw_runtime_toggle = <RuntimeToggleButton> {
                    text: "OpenClaw Runtime"
                }

                openclaw_runtime_summary_label = <SummaryLabel> {}

                openclaw_runtime_form = <RuntimeForm> {
                    openclaw_gateway_input = <SimpleTextInput> {
                        width: Fill, height: Fit
                        empty_text: "ws://127.0.0.1:24282/ws"
                    }

                    openclaw_auth_token_env_input = <SimpleTextInput> {
                        width: Fill, height: Fit
                        empty_text: "Optional gateway token env var"
                    }
                }
            }
        }

        bots_section = <View> {
            visible: false
            width: Fill, height: Fit
            flow: Down
            spacing: 12

            bots_inventory_card = <SettingsCard> {
                <SubsectionLabel> {
                    text: "Installed Bots"
                }

                bots_summary_label = <SummaryLabel> {}
            }

            bot_commands_card = <SettingsCard> {
                <SubsectionLabel> {
                    text: "Slash Commands"
                }

                command_hint_label = <SummaryLabel> {}
            }
        }

        workspace_section = <View> {
            visible: false
            width: Fill, height: Fit
            flow: Down
            spacing: 12

            workspace_card = <SettingsCard> {
                <SubsectionLabel> {
                    text: "Shared Workspace"
                }

                workspace_summary_label = <SummaryLabel> {}

                workspace_root_input = <SimpleTextInput> {
                    width: Fill, height: Fit
                    empty_text: "/path/to/workspace (optional)"
                }
            }
        }

        actions_card = <SettingsCard> {
            <SubsectionLabel> {
                text: "Actions"
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
                    text: "Save Runtime Profiles"
                }
            }

            status_label = <SummaryLabel> {}
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BotfatherSettingsSection {
    #[default]
    Runtimes,
    Bots,
    Workspace,
}

#[derive(Live, LiveHook, Widget)]
pub struct BotfatherSettings {
    #[deref]
    view: View,
    #[rust]
    selected_section: BotfatherSettingsSection,
    #[rust]
    crew_runtime_expanded: bool,
    #[rust]
    openclaw_runtime_expanded: bool,
}

impl Widget for BotfatherSettings {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if matches!(event, Event::Signal) {
            let _ = botfather::refresh_inventory_from_rooms_list(cx);
        }

        if let Event::Actions(actions) = event {
            let runtimes_section_button = self.view.button(ids!(runtimes_section_button));
            let bots_section_button = self.view.button(ids!(bots_section_button));
            let workspace_section_button = self.view.button(ids!(workspace_section_button));
            let refresh_inventory_button = self.view.button(ids!(refresh_inventory_button));
            let save_defaults_button = self.view.button(ids!(save_defaults_button));
            let crew_runtime_toggle = self.view.button(ids!(crew_runtime_toggle));
            let openclaw_runtime_toggle = self.view.button(ids!(openclaw_runtime_toggle));
            let crew_endpoint_input = self.view.text_input(ids!(crew_endpoint_input));
            let crew_auth_token_env_input = self.view.text_input(ids!(crew_auth_token_env_input));
            let openclaw_gateway_input = self.view.text_input(ids!(openclaw_gateway_input));
            let openclaw_auth_token_env_input =
                self.view.text_input(ids!(openclaw_auth_token_env_input));
            let workspace_root_input = self.view.text_input(ids!(workspace_root_input));

            if runtimes_section_button.clicked(actions) {
                self.set_section(cx, BotfatherSettingsSection::Runtimes);
            }
            if bots_section_button.clicked(actions) {
                self.set_section(cx, BotfatherSettingsSection::Bots);
            }
            if workspace_section_button.clicked(actions) {
                self.set_section(cx, BotfatherSettingsSection::Workspace);
            }
            if crew_runtime_toggle.clicked(actions) {
                self.crew_runtime_expanded = !self.crew_runtime_expanded;
                self.apply_runtime_cards_state(cx);
            }
            if openclaw_runtime_toggle.clicked(actions) {
                self.openclaw_runtime_expanded = !self.openclaw_runtime_expanded;
                self.apply_runtime_cards_state(cx);
            }

            if refresh_inventory_button.clicked(actions) {
                match botfather::refresh_inventory_from_rooms_list(cx) {
                    Ok(true) => {
                        self.populate(cx, None);
                        self.set_status(cx, "Bot inventory refreshed from the current room list.");
                    }
                    Ok(false) => self.set_status(cx, "Bot inventory was already up to date."),
                    Err(error) => self.set_status(cx, &error),
                }
            }

            if save_defaults_button.clicked(actions) {
                match botfather::save_default_profiles(
                    &crew_endpoint_input.text(),
                    &crew_auth_token_env_input.text(),
                    &openclaw_gateway_input.text(),
                    &openclaw_auth_token_env_input.text(),
                    &workspace_root_input.text(),
                ) {
                    Ok(()) => {
                        self.populate(cx, None);
                        self.set_status(cx, "Saved the default BotFather runtime profiles.");
                    }
                    Err(error) => self.set_status(cx, &error),
                }
            }

            for action in actions {
                if let Some(LoginAction::LoginSuccess) = action.downcast_ref() {
                    let _ = botfather::ensure_loaded_for_current_user();
                    self.populate(cx, None);
                    continue;
                }

                if let Some(LogoutAction::ClearAppState { .. }) = action.downcast_ref() {
                    self.clear(cx);
                    continue;
                }

                if let Some(BotfatherAction::StateChanged) = action.downcast_ref() {
                    self.populate(cx, None);
                    continue;
                }

                if let Some(BotfatherAction::Status(status)) = action.downcast_ref() {
                    self.set_status(cx, status);
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl BotfatherSettings {
    pub fn populate(&mut self, cx: &mut Cx, _selected_room_id: Option<String>) {
        let _ = botfather::ensure_loaded_for_current_user();
        let _ = botfather::refresh_inventory_from_rooms_list(cx);
        let defaults = botfather::default_config_form();

        self.view
            .text_input(ids!(crew_endpoint_input))
            .set_text(cx, &defaults.crew_endpoint);
        self.view
            .text_input(ids!(crew_auth_token_env_input))
            .set_text(cx, &defaults.crew_auth_token_env);
        self.view
            .text_input(ids!(openclaw_gateway_input))
            .set_text(cx, &defaults.openclaw_gateway_url);
        self.view
            .text_input(ids!(openclaw_auth_token_env_input))
            .set_text(cx, &defaults.openclaw_auth_token_env);
        self.view
            .text_input(ids!(workspace_root_input))
            .set_text(cx, &defaults.workspace_root);

        self.view
            .label(ids!(crew_runtime_summary_label))
            .set_text(cx, &botfather::runtime_summary(RuntimeKind::Crew));
        self.view
            .label(ids!(openclaw_runtime_summary_label))
            .set_text(cx, &botfather::runtime_summary(RuntimeKind::OpenClaw));
        self.view
            .label(ids!(bots_summary_label))
            .set_text(cx, &botfather::bots_overview());
        let command_help = format!(
            "{}\n\nexamples\n/bot create reviewer\n/bot bind reviewer\n/bot status\n\nplanned next\n/bot set-model <bot> <model>",
            help_text()
        );
        self.view
            .label(ids!(command_hint_label))
            .set_text(cx, &command_help);
        self.view
            .label(ids!(workspace_summary_label))
            .set_text(cx, &botfather::workspace_overview());

        self.apply_section_state(cx);
    }

    fn clear(&mut self, cx: &mut Cx) {
        for input in [
            ids!(crew_endpoint_input),
            ids!(crew_auth_token_env_input),
            ids!(openclaw_gateway_input),
            ids!(openclaw_auth_token_env_input),
            ids!(workspace_root_input),
        ] {
            self.view.text_input(input).set_text(cx, "");
        }

        for label in [
            ids!(crew_runtime_summary_label),
            ids!(openclaw_runtime_summary_label),
            ids!(bots_summary_label),
            ids!(command_hint_label),
            ids!(workspace_summary_label),
            ids!(status_label),
        ] {
            self.view.label(label).set_text(cx, "");
        }
    }

    fn set_status(&mut self, cx: &mut Cx, status: &str) {
        self.view.label(ids!(status_label)).set_text(cx, status);
    }

    fn set_section(&mut self, cx: &mut Cx, section: BotfatherSettingsSection) {
        self.selected_section = section;
        self.apply_section_state(cx);
    }

    fn apply_section_state(&mut self, cx: &mut Cx) {
        let runtimes_active = self.selected_section == BotfatherSettingsSection::Runtimes;
        let bots_active = self.selected_section == BotfatherSettingsSection::Bots;
        let workspace_active = self.selected_section == BotfatherSettingsSection::Workspace;

        self.view
            .view(ids!(runtimes_section))
            .set_visible(cx, runtimes_active);
        self.view
            .view(ids!(bots_section))
            .set_visible(cx, bots_active);
        self.view
            .view(ids!(workspace_section))
            .set_visible(cx, workspace_active);

        apply_section_button_style(
            &self.view.button(ids!(runtimes_section_button)),
            cx,
            runtimes_active,
        );
        apply_section_button_style(
            &self.view.button(ids!(bots_section_button)),
            cx,
            bots_active,
        );
        apply_section_button_style(
            &self.view.button(ids!(workspace_section_button)),
            cx,
            workspace_active,
        );
        self.apply_runtime_cards_state(cx);
    }

    fn apply_runtime_cards_state(&mut self, cx: &mut Cx) {
        self.view
            .view(ids!(crew_runtime_form))
            .set_visible(cx, self.crew_runtime_expanded);
        self.view
            .view(ids!(openclaw_runtime_form))
            .set_visible(cx, self.openclaw_runtime_expanded);

        apply_runtime_toggle_style(
            &self.view.button(ids!(crew_runtime_toggle)),
            cx,
            self.crew_runtime_expanded,
        );
        apply_runtime_toggle_style(
            &self.view.button(ids!(openclaw_runtime_toggle)),
            cx,
            self.openclaw_runtime_expanded,
        );
    }
}

fn apply_section_button_style(button: &ButtonRef, cx: &mut Cx, active: bool) {
    let (bg_color, fg_color) = if active {
        (
            crate::shared::styles::COLOR_ACTIVE_PRIMARY,
            crate::shared::styles::COLOR_PRIMARY,
        )
    } else {
        (
            crate::shared::styles::COLOR_BG_PREVIEW,
            vec4(0.109, 0.153, 0.298, 1.0),
        )
    };
    button.apply_over(
        cx,
        live! {
            draw_bg: {
                color: (bg_color)
            }
            draw_icon: {
                color: (fg_color)
            }
            draw_text: {
                color: (fg_color)
            }
        },
    );
}

fn apply_runtime_toggle_style(button: &ButtonRef, cx: &mut Cx, expanded: bool) {
    let (bg_color, fg_color, rotation_angle) = if expanded {
        (
            crate::shared::styles::COLOR_ACTIVE_PRIMARY,
            crate::shared::styles::COLOR_PRIMARY,
            180.0,
        )
    } else {
        (
            crate::shared::styles::COLOR_BG_PREVIEW,
            vec4(0.109, 0.153, 0.298, 1.0),
            90.0,
        )
    };
    button.apply_over(
        cx,
        live! {
            draw_bg: {
                color: (bg_color)
            }
            draw_icon: {
                color: (fg_color)
                rotation_angle: (rotation_angle)
            }
            draw_text: {
                color: (fg_color)
            }
        },
    );
}

impl BotfatherSettingsRef {
    pub fn populate(&self, cx: &mut Cx, selected_room_id: Option<String>) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.populate(cx, selected_room_id);
    }
}
