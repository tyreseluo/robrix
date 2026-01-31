
use makepad_widgets::*;

use crate::{
    home::navigation_tab_bar::{NavigationBarAction, get_own_profile},
    profile::user_profile::UserProfile,
    settings::account_settings::AccountSettingsWidgetExt,
    shared::styles::{COLOR_ACTIVE_PRIMARY, COLOR_PRIMARY},
};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::helpers::*;
    use crate::shared::styles::*;
    use crate::shared::icon_button::*;
    use crate::shared::confirmation_modal::*;
    use crate::settings::account_settings::AccountSettings;
    use link::tsp_link::TspSettingsScreen;
    use link::tsp_link::CreateWalletModal;
    use link::tsp_link::CreateDidModal;
    use link::robit_link::RobitSettingsScreen;

    // The main, top-level settings screen widget.
    pub SettingsScreen = {{SettingsScreen}} {
        width: Fill, height: Fill,
        flow: Overlay

        <View> {
            padding: {top: 5, left: 15, right: 15, bottom: 0},
            flow: Down

            // The settings header shows a title, with a close button to the right.
            settings_header = <View> {
                flow: Right,
                align: {x: 1.0, y: 0.5},
                width: Fill, height: Fit
                margin: {left: 5, right: 5}
                spacing: 10,

                settings_header_title = <TitleLabel> {
                    margin: {top: 4} // line up with the close button
                    text: "All Settings"
                    draw_text: {
                        text_style: {font_size: 18},
                    }
                }

                // The "X" close button on the top right
                close_button = <RobrixIconButton> {
                    width: Fit,
                    height: Fit,
                    align: {x: 1.0, y: 0.0},
                    spacing: 0,
                    margin: {top: 4.5} // vertically align with the title
                    padding: 15,

                    draw_bg: {
                        color: (COLOR_SECONDARY)
                    }
                    draw_icon: {
                        svg_file: (ICON_CLOSE),
                        fn get_color(self) -> vec4 {
                            return #x0;
                        }
                    }
                    icon_walk: {width: 14, height: 14}
                }
            }

            // Make sure the dividing line is aligned with the close_button
            <LineH> { padding: 10, margin: {top: 10, right: 2} }

            <ScrollXYView> {
                width: Fill, height: Fill
                flow: Down

                // The account settings section.
                account_settings = <AccountSettings> {}

                <LineH> { width: 400, padding: 10, margin: {top: 20, bottom: 5} }

                // The TSP wallet settings section.
                tsp_settings_screen = <TspSettingsScreen> {}

                <LineH> { width: 400, padding: 10, margin: {top: 20, bottom: 5} }

                // The Robit settings section.
                robit_settings_screen = <RobitSettingsScreen> {}

                <LineH> { width: 400, padding: 10, margin: {top: 20, bottom: 5} }

                // Add other settings sections here as needed.
                // Don't forget to add a `show()` fn to those settings sections
                // and call them in `SettingsScreen::show()`.
            }
        }

        // We want all modals to appear in front of the settings screen.
        create_wallet_modal = <Modal> {
            content: {
                create_wallet_modal_inner = <CreateWalletModal> {}
            }
        }

        create_did_modal = <Modal> {
            content: {
                create_did_modal_inner = <CreateDidModal> {}
            }
        }

        remove_delete_wallet_modal = <Modal> {
            content: {
                remove_delete_wallet_modal_inner = <NegativeConfirmationModal> { }
            }
        }

        // A simple modal shown when clicking the Robit button.
        robit_modal = <Modal> {
            content: {
                robit_modal_inner = <RoundedView> {
                    width: 720,
                    height: 520,
                    flow: Overlay,
                    align: {x: 0.5},
                    padding: 0,
                    spacing: 0,

                    show_bg: true,
                    draw_bg: {
                        color: #FFFFFF
                    }
                    margin: 0

                    robit_modal_body = <View> {
                        width: Fill,
                        height: Fill,
                        flow: Right,
                        spacing: 0,

                        robit_modal_nav = <RoundedView> {
                            width: 200,
                            height: Fill,
                            flow: Down,
                            spacing: 8,
                            padding: 10,

                            show_bg: true,
                            draw_bg: {
                                color: (COLOR_SECONDARY),
                            }

                            robit_nav_all_button = <RobrixIconButton> {
                                width: Fill,
                                padding: {top: 8, bottom: 8, left: 10, right: 10}
                                draw_bg: {
                                    color: (COLOR_SECONDARY)
                                }
                                draw_text: {
                                    color: (MESSAGE_TEXT_COLOR)
                                    text_style: <REGULAR_TEXT> {font_size: 12}
                                }
                                text: "All"
                            }

                            robit_nav_agent_button = <RobrixIconButton> {
                                width: Fill,
                                padding: {top: 8, bottom: 8, left: 10, right: 10}
                                draw_bg: {
                                    color: (COLOR_SECONDARY)
                                }
                                draw_text: {
                                    color: (MESSAGE_TEXT_COLOR)
                                    text_style: <REGULAR_TEXT> {font_size: 12}
                                }
                                text: "Agents Config"
                            }

                            robit_nav_about_button = <RobrixIconButton> {
                                width: Fill,
                                padding: {top: 8, bottom: 8, left: 10, right: 10}
                                draw_bg: {
                                    color: (COLOR_SECONDARY)
                                }
                                draw_text: {
                                    color: (MESSAGE_TEXT_COLOR)
                                    text_style: <REGULAR_TEXT> {font_size: 12}
                                }
                                text: "About"
                            }
                        }

                        robit_modal_content = <View> {
                            width: Fill,
                            height: Fill,
                            flow: Down,
                            spacing: 10,
                            padding: {top: 20, right: 20, bottom: 20, left: 20},

                            robit_content_all = <View> {
                                width: Fill,
                                height: Fit,
                                flow: Down,
                                spacing: 8,

                                <TitleLabel> {
                                    text: "All"
                                }

                                <Label> {
                                    width: Fill,
                                    draw_text: {
                                        text_style: <REGULAR_TEXT>{
                                            font_size: 13,
                                        },
                                        color: #000000,
                                        wrap: Word
                                    },
                                    text: "All 页面内容占位。"
                                }
                            }

                            robit_content_agent = <View> {
                                visible: false
                                width: Fill,
                                height: Fit,
                                flow: Down,
                                spacing: 8,

                                <TitleLabel> {
                                    text: "Agent 配置页面"
                                }

                                <Label> {
                                    width: Fill,
                                    draw_text: {
                                        text_style: <REGULAR_TEXT>{
                                            font_size: 13,
                                        },
                                        color: #000000,
                                        wrap: Word
                                    },
                                    text: "Agent 配置页面内容占位。"
                                }
                            }

                            robit_content_about = <View> {
                                visible: false
                                width: Fill,
                                height: Fit,
                                flow: Down,
                                spacing: 8,

                                <TitleLabel> {
                                    text: "About Robit"
                                }

                                <Label> {
                                    width: Fill,
                                    draw_text: {
                                        text_style: <REGULAR_TEXT>{
                                            font_size: 13,
                                        },
                                        color: #000000,
                                        wrap: Word
                                    },
                                    text: "Version: 0.1.0"
                                }

                                <Label> {
                                    width: Fill,
                                    draw_text: {
                                        text_style: <REGULAR_TEXT>{
                                            font_size: 13,
                                        },
                                        color: #000000,
                                        wrap: Word
                                    },
                                    text: "Source: ../robit (local workspace)"
                                }

                                <Label> {
                                    width: Fill,
                                    draw_text: {
                                        text_style: <REGULAR_TEXT>{
                                            font_size: 13,
                                        },
                                        color: #000000,
                                        wrap: Word
                                    },
                                    text: "License: MIT"
                                }
                            }
                        }
                    }

                    robit_modal_close_layer = <View> {
                        width: Fill,
                        height: Fit,
                        flow: Right,
                        align: {x: 1.0, y: 0.0},
                        padding: {top: 8, right: 8},

                        robit_modal_close_button = <RobrixIconButton> {
                            width: Fit, height: Fit,
                            padding: 6,
                            draw_bg: {
                                color: #00000000,
                                color_hover: #00000000,
                                border_size: 0.0
                            },
                            draw_icon: {
                                svg_file: (ICON_CLOSE),
                                color: #000000
                            }
                            icon_walk: {width: 12, height: 12}
                        }
                    }
                }
            }
        }
    }
}


/// The top-level widget showing all app and user settings/preferences.
#[derive(Live, LiveHook, Widget)]
pub struct SettingsScreen {
    #[deref] view: View,
    #[rust] robit_modal_tab: RobitModalTab,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
enum RobitModalTab {
    #[default]
    All,
    Agent,
    About,
}

impl Widget for SettingsScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Close the pane if:
        // 1. The close button is clicked,
        // 2. The back navigational gesture/action occurs (e.g., Back on Android),
        // 3. The escape key is pressed if this pane has key focus,
        // 4. The back mouse button is clicked within this view.
        let area = self.view.area();
        let close_pane = {
            matches!(
                event,
                Event::Actions(actions) if self.button(ids!(close_button)).clicked(actions)
            )
            || event.back_pressed()
            || match event.hits(cx, area) {
                Hit::KeyUp(key) => key.key_code == KeyCode::Escape,
                Hit::FingerDown(_fde) => {
                    cx.set_key_focus(area);
                    false
                }
                _ => false,
            }
        };
        if close_pane {
            cx.action(NavigationBarAction::CloseSettings);
        }

        if let Event::Actions(actions) = event {
            if self.view.button(ids!(robit_settings_screen.robit_open_button)).clicked(actions) {
                self.set_robit_modal_tab(cx, RobitModalTab::All);
                self.view.modal(ids!(robit_modal)).open(cx);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_nav.robit_nav_all_button)).clicked(actions) {
                self.set_robit_modal_tab(cx, RobitModalTab::All);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_nav.robit_nav_agent_button)).clicked(actions) {
                self.set_robit_modal_tab(cx, RobitModalTab::Agent);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_nav.robit_nav_about_button)).clicked(actions) {
                self.set_robit_modal_tab(cx, RobitModalTab::About);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_close_layer.robit_modal_close_button)).clicked(actions)
                || actions.iter().any(|a| matches!(a.downcast_ref(), Some(ModalAction::Dismissed)))
            {
                self.view.modal(ids!(robit_modal)).close(cx);
            }
        }

        #[cfg(feature = "tsp")]
        if let Event::Actions(actions) = event {
            use crate::shared::confirmation_modal::ConfirmationModalWidgetExt;
            use crate::tsp::{
                create_did_modal::CreateDidModalAction,
                create_wallet_modal::CreateWalletModalAction,
                wallet_entry::TspWalletEntryAction,
            };

            for action in actions {
                // Handle the create wallet modal being opened or closed.
                match action.downcast_ref() {
                    Some(CreateWalletModalAction::Open) => {
                        use crate::tsp::create_wallet_modal::CreateWalletModalWidgetExt;
                        self.view.create_wallet_modal(ids!(create_wallet_modal_inner)).show(cx);
                        self.view.modal(ids!(create_wallet_modal)).open(cx);
                    }
                    Some(CreateWalletModalAction::Close) => {
                        self.view.modal(ids!(create_wallet_modal)).close(cx);
                    }
                    None => { }
                }

                // Handle the create DID modal being opened or closed.
                match action.downcast_ref() {
                    Some(CreateDidModalAction::Open) => {
                        use crate::tsp::create_did_modal::CreateDidModalWidgetExt;
                        self.view.create_did_modal(ids!(create_did_modal_inner)).show(cx);
                        self.view.modal(ids!(create_did_modal)).open(cx);
                    }
                    Some(CreateDidModalAction::Close) => {
                        self.view.modal(ids!(create_did_modal)).close(cx);
                    }
                    None => { }
                }

                // Handle a request to show a TSP wallet confirmation modal.
                if let Some(TspWalletEntryAction::ShowConfirmationModal(content_opt)) = action.downcast_ref() {
                    if let Some(content) = content_opt.borrow_mut().take() {
                        self.view.confirmation_modal(ids!(remove_delete_wallet_modal_inner)).show(cx, content);
                        self.view.modal(ids!(remove_delete_wallet_modal)).open(cx);
                    }
                }
            }

            if let Some(_accepted) = self.view.confirmation_modal(ids!(remove_delete_wallet_modal_inner)).closed(actions) {
                self.view.modal(ids!(remove_delete_wallet_modal)).close(cx);
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl SettingsScreen {
    fn set_robit_modal_tab(&mut self, cx: &mut Cx, tab: RobitModalTab) {
        self.robit_modal_tab = tab;

        let show_all = tab == RobitModalTab::All;
        let show_agent = tab == RobitModalTab::Agent;
        let show_about = tab == RobitModalTab::About;

        self.view
            .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_content.robit_content_all))
            .set_visible(cx, show_all);
        self.view
            .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_content.robit_content_agent))
            .set_visible(cx, show_agent);
        self.view
            .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_content.robit_content_about))
            .set_visible(cx, show_about);

        self.update_robit_modal_nav_styles(cx);
    }

    fn update_robit_modal_nav_styles(&mut self, cx: &mut Cx) {
        let unselected_bg = vec4(0.89, 0.89, 0.89, 1.0);
        let unselected_text = vec4(0.2, 0.2, 0.2, 1.0);
        let selected_bg = COLOR_ACTIVE_PRIMARY;
        let selected_text = COLOR_PRIMARY;

        let mut apply_style = |button_id, selected: bool| {
            let (bg, text) = if selected {
                (selected_bg, selected_text)
            } else {
                (unselected_bg, unselected_text)
            };
            self.view.button(button_id).apply_over(
                cx,
                live! {
                    draw_bg: { color: (bg) }
                    draw_text: { color: (text) }
                },
            );
        };

        apply_style(
            ids!(robit_modal_inner.robit_modal_body.robit_modal_nav.robit_nav_all_button),
            self.robit_modal_tab == RobitModalTab::All,
        );
        apply_style(
            ids!(robit_modal_inner.robit_modal_body.robit_modal_nav.robit_nav_agent_button),
            self.robit_modal_tab == RobitModalTab::Agent,
        );
        apply_style(
            ids!(robit_modal_inner.robit_modal_body.robit_modal_nav.robit_nav_about_button),
            self.robit_modal_tab == RobitModalTab::About,
        );
    }
    /// Fetches the current user's profile and uses it to populate the settings screen.
    pub fn populate(&mut self, cx: &mut Cx, own_profile: Option<UserProfile>) {
        let Some(profile) = own_profile.or_else(|| get_own_profile(cx)) else {
            error!("Failed to get own profile for settings screen.");
            return;
        };
        self.view.account_settings(ids!(account_settings)).populate(cx, profile);
        self.view.button(ids!(close_button)).reset_hover(cx);
        cx.set_key_focus(self.view.area());
        self.redraw(cx);
    }
}

impl SettingsScreenRef {
    /// See [`SettingsScreen::populate()`].
    pub fn populate(&self, cx: &mut Cx, own_profile: Option<UserProfile>) {
        let Some(mut inner) = self.borrow_mut() else { return; };
        inner.populate(cx, own_profile);
    }
}
