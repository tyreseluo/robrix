
use makepad_widgets::*;
use serde::Deserialize;
use tokio::runtime::Builder;

use crate::{
    home::navigation_tab_bar::{NavigationBarAction, get_own_profile},
    profile::user_profile::UserProfile,
    settings::account_settings::AccountSettingsWidgetExt,
    shared::styles::{
        COLOR_ACTIVE_PRIMARY,
        COLOR_PRIMARY,
        COLOR_FG_ACCEPT_GREEN,
        COLOR_FG_DANGER_RED,
        COLOR_FG_DISABLED,
    },
};

const ROBIT_MODEL_ROW_COUNT: usize = 10;

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

    IMG_PROVIDER_DEEPSEEK = dep("crate://self/resources/img/providers/deepseek.png")
    IMG_SYNC_REFRESH = dep("crate://self/resources/img/refresh_icon.png")

    RobitSwitch = <Toggle> {
        // U+200e to avoid rendering a label.
        text: "‎"
        draw_bg: {
            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let pill_padding = 2.0;
                let pill_color_off = #D9D9D9;
                let pill_color_on = #429E92;

                let pill_radius = self.rect_size.y * 0.5;
                let ball_radius = pill_radius - pill_padding;

                sdf.circle(pill_radius, pill_radius, pill_radius);
                sdf.fill(mix(pill_color_off, pill_color_on, self.active));

                sdf.circle(self.rect_size.x - pill_radius, pill_radius, pill_radius);
                sdf.fill(mix(pill_color_off, pill_color_on, self.active));

                sdf.rect(pill_radius, 0.0, self.rect_size.x - 2.0 * pill_radius, self.rect_size.y);
                sdf.fill(mix(pill_color_off, pill_color_on, self.active));

                sdf.circle(
                    pill_padding + ball_radius + self.active * (self.rect_size.x - 2.0 * ball_radius - 2.0 * pill_padding),
                    pill_radius,
                    ball_radius
                );
                sdf.fill(#fff);

                return sdf.result;
            }
        }
    }

    RobitModelRow = <RoundedView> {
        visible: false
        width: Fill,
        height: Fit,
        flow: Right,
        spacing: 8,
        padding: {top: 6, bottom: 6, left: 8, right: 8},

        show_bg: true,
        draw_bg: {
            color: #FFFFFF,
            border_radius: 4.0
        }

        model_name = <Label> {
            width: Fill,
            draw_text: {
                text_style: <REGULAR_TEXT>{font_size: 11},
                color: #000000
            },
            text: "model-name"
        }

        model_toggle = <RoundedView> {
            width: Fit,
            height: Fit,
            padding: {top: 4, bottom: 4, left: 10, right: 10},
            show_bg: true,
            draw_bg: {
                color: (COLOR_BG_ACCEPT_GREEN),
                border_radius: 10.0
            }

            <Label> {
                draw_text: {
                    text_style: <REGULAR_TEXT>{font_size: 10},
                    color: (COLOR_FG_ACCEPT_GREEN)
                },
                text: "Enabled"
            }
        }
    }

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
                        flow: Overlay,
                        spacing: 0,
                        robit_modal_main = <View> {
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

                                robit_nav_close_button = <RobrixIconButton> {
                                    width: Fit,
                                    height: Fit,
                                    margin: {bottom: 4},
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
                                    text: "Provider Settings"
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
                                    text: "About Robit"
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
                                    height: Fill,
                                    flow: Down,
                                    spacing: 12,

                                    all_toolbar = <View> {
                                        width: Fill,
                                        height: Fit,
                                        flow: Right,
                                        align: {y: 0.5},
                                        padding: {bottom: 6},

                                        <FillerX> {}

                                        new_robit_button = <RobrixIconButton> {
                                            padding: {top: 8, bottom: 8, left: 12, right: 12}
                                            draw_bg: {
                                                color: (COLOR_ACTIVE_PRIMARY)
                                            }
                                            draw_icon: {
                                                svg_file: (ICON_ADD)
                                                color: (COLOR_PRIMARY)
                                            }
                                            draw_text: {
                                                color: (COLOR_PRIMARY)
                                                text_style: <REGULAR_TEXT> {font_size: 12}
                                            }
                                            icon_walk: {width: 12, height: 12}
                                            text: "New Robit"
                                        }
                                    }

                                    all_scroll = <ScrollXYView> {
                                        width: Fill,
                                        height: Fill,
                                        flow: Down,
                                        spacing: 12,

                                        workspace_card_design = <RoundedView> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 8,
                                            padding: 12,

                                            show_bg: true,
                                            draw_bg: {
                                                color: #F6F6F6,
                                                border_radius: 8.0
                                            }

                                            workspace_header = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 8,
                                                align: {y: 0.5},
                                                cursor: Hand,

                                                <Label> {
                                                    width: 14,
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #333333
                                                    },
                                                    text: "▾"
                                                }

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "Design Ops"
                                                }

                                                <Label> {
                                                    margin: {left: 6},
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "3 rooms"
                                                }

                                                <FillerX> {}

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "Default: gpt-4.1-mini"
                                                }

                                                workspace_delete_button = <RobrixIconButton> {
                                                    padding: 6,
                                                    draw_bg: {
                                                        color: #00000000,
                                                        color_hover: #00000000,
                                                        border_size: 0.0
                                                    },
                                                    draw_icon: {
                                                        svg_file: (ICON_TRASH),
                                                        color: (COLOR_FG_DANGER_RED)
                                                    }
                                                    icon_walk: {width: 12, height: 12}
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            workspace_rooms = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 4,

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#product-briefs"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4.1-mini ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#design-reviews"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4.1 ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: true

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#ux-research"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4o-mini ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }
                                            }
                                        }

                                        workspace_card_sales = <RoundedView> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 8,
                                            padding: 12,

                                            show_bg: true,
                                            draw_bg: {
                                                color: #F6F6F6,
                                                border_radius: 8.0
                                            }

                                            workspace_header = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 8,
                                                align: {y: 0.5},
                                                cursor: Hand,

                                                <Label> {
                                                    width: 14,
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #333333
                                                    },
                                                    text: "▾"
                                                }

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "Sales Hub"
                                                }

                                                <Label> {
                                                    margin: {left: 6},
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "3 rooms"
                                                }

                                                <FillerX> {}

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "Default: gpt-4o-mini"
                                                }

                                                workspace_delete_button = <RobrixIconButton> {
                                                    padding: 6,
                                                    draw_bg: {
                                                        color: #00000000,
                                                        color_hover: #00000000,
                                                        border_size: 0.0
                                                    },
                                                    draw_icon: {
                                                        svg_file: (ICON_TRASH),
                                                        color: (COLOR_FG_DANGER_RED)
                                                    }
                                                    icon_walk: {width: 12, height: 12}
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            workspace_rooms = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 4,

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#pipeline-qa"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4o-mini ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#partner-requests"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4o ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#pricing-ops"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4.1-mini ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }
                                            }
                                        }

                                        workspace_card_growth = <RoundedView> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 8,
                                            padding: 12,

                                            show_bg: true,
                                            draw_bg: {
                                                color: #F6F6F6,
                                                border_radius: 8.0
                                            }

                                            workspace_header = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 8,
                                                align: {y: 0.5},
                                                cursor: Hand,

                                                <Label> {
                                                    width: 14,
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #333333
                                                    },
                                                    text: "▸"
                                                }

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "Growth"
                                                }

                                                <Label> {
                                                    margin: {left: 6},
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "4 rooms"
                                                }

                                                <FillerX> {}

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "Default: gpt-4o"
                                                }

                                                workspace_delete_button = <RobrixIconButton> {
                                                    padding: 6,
                                                    draw_bg: {
                                                        color: #00000000,
                                                        color_hover: #00000000,
                                                        border_size: 0.0
                                                    },
                                                    draw_icon: {
                                                        svg_file: (ICON_TRASH),
                                                        color: (COLOR_FG_DANGER_RED)
                                                    }
                                                    icon_walk: {width: 12, height: 12}
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            workspace_rooms = <View> {
                                                visible: false
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 4,

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#growth-experiments"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4o ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }
                                            }
                                        }

                                        workspace_card_platform = <RoundedView> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 8,
                                            padding: 12,

                                            show_bg: true,
                                            draw_bg: {
                                                color: #F6F6F6,
                                                border_radius: 8.0
                                            }

                                            workspace_header = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 8,
                                                align: {y: 0.5},
                                                cursor: Hand,

                                                <Label> {
                                                    width: 14,
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #333333
                                                    },
                                                    text: "▾"
                                                }

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "Platform"
                                                }

                                                <Label> {
                                                    margin: {left: 6},
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "3 rooms"
                                                }

                                                <FillerX> {}

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "Default: gpt-4.1"
                                                }

                                                workspace_delete_button = <RobrixIconButton> {
                                                    padding: 6,
                                                    draw_bg: {
                                                        color: #00000000,
                                                        color_hover: #00000000,
                                                        border_size: 0.0
                                                    },
                                                    draw_icon: {
                                                        svg_file: (ICON_TRASH),
                                                        color: (COLOR_FG_DANGER_RED)
                                                    }
                                                    icon_walk: {width: 12, height: 12}
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            workspace_rooms = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 4,

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#infra-alerts"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4.1 ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#release-notes"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4.1-mini ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#oncall"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4o ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }
                                            }
                                        }

                                        workspace_card_unassigned = <RoundedView> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 8,
                                            padding: 12,

                                            show_bg: true,
                                            draw_bg: {
                                                color: #F6F6F6,
                                                border_radius: 8.0
                                            }

                                            workspace_header = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 8,
                                                align: {y: 0.5},

                                                <Label> {
                                                    width: 14,
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #333333
                                                    },
                                                    text: "•"
                                                }

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "Standalone Rooms"
                                                }

                                                <Label> {
                                                    margin: {left: 6},
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "2 rooms"
                                                }

                                                <FillerX> {}

                                                workspace_delete_button = <RobrixIconButton> {
                                                    padding: 6,
                                                    draw_bg: {
                                                        color: #00000000,
                                                        color_hover: #00000000,
                                                        border_size: 0.0
                                                    },
                                                    draw_icon: {
                                                        svg_file: (ICON_TRASH),
                                                        color: (COLOR_FG_DANGER_RED)
                                                    }
                                                    icon_walk: {width: 12, height: 12}
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            workspace_rooms = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 4,

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#random"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4o-mini ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                                <RoundedView> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                    cursor: Hand,

                                                    show_bg: true,
                                                    draw_bg: {
                                                        instance hover: 0.0
                                                        color: #00000000
                                                        color_hover: #00000010
                                                        border_radius: 4.0
                                                        fn get_color(self) -> vec4 {
                                                            return mix(self.color, self.color_hover, self.hover);
                                                        }
                                                        fn pixel(self) -> vec4 {
                                                            let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                            sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                            sdf.fill(self.get_color());
                                                            return sdf.result;
                                                        }
                                                    }

                                                    <Label> {
                                                        width: Fill,
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 12},
                                                            color: #000000,
                                                            wrap: Word
                                                        },
                                                        text: "#private-notes"
                                                    }

                                                    model_select = <RobrixIconButton> {
                                                        width: 150,
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                        text: "gpt-4.1-mini ▾"
                                                    }

                                                    model_actions = <View> {
                                                        width: Fit,
                                                        height: Fit,
                                                        flow: Right,
                                                        spacing: 6,
                                                        visible: false

                                                        save_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                            draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Save"
                                                        }

                                                        cancel_button = <RobrixIconButton> {
                                                            padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                            draw_bg: { color: (COLOR_SECONDARY) }
                                                            draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                            text: "Cancel"
                                                        }
                                                    }

                                                    room_delete_button = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        },
                                                        draw_icon: {
                                                            svg_file: (ICON_TRASH),
                                                            color: (COLOR_FG_DANGER_RED)
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }
                                            }
                                        }
                                        room_card_personal = <RoundedView> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 8,
                                            padding: 12,

                                            show_bg: true,
                                            draw_bg: {
                                                color: #F6F6F6,
                                                border_radius: 8.0
                                            }

                                            room_header = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 8,
                                                align: {y: 0.5},

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "Standalone Room"
                                                }

                                                <Label> {
                                                    margin: {left: 6},
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "no workspace"
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            <RoundedView> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 10,
                                                padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                cursor: Hand,

                                                show_bg: true,
                                                draw_bg: {
                                                    instance hover: 0.0
                                                    color: #00000000
                                                    color_hover: #00000010
                                                    border_radius: 4.0
                                                    fn get_color(self) -> vec4 {
                                                        return mix(self.color, self.color_hover, self.hover);
                                                    }
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                        sdf.fill(self.get_color());
                                                        return sdf.result;
                                                    }
                                                }

                                                <Label> {
                                                    width: Fill,
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 12},
                                                        color: #000000,
                                                        wrap: Word
                                                    },
                                                    text: "#personal-journal"
                                                }

                                                model_select = <RobrixIconButton> {
                                                    width: 150,
                                                    padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                    draw_bg: {
                                                        color: #FFFFFF,
                                                        border_size: 1.0,
                                                        border_color: #DDDDDD,
                                                        border_radius: 4.0
                                                    }
                                                    draw_text: {
                                                        color: #000000
                                                        text_style: <REGULAR_TEXT> {font_size: 11}
                                                    }
                                                    text: "gpt-4.1-mini ▾"
                                                }

                                                model_actions = <View> {
                                                    width: Fit,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 6,
                                                    visible: false

                                                    save_button = <RobrixIconButton> {
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                        draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                        text: "Save"
                                                    }

                                                    cancel_button = <RobrixIconButton> {
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: { color: (COLOR_SECONDARY) }
                                                        draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                        text: "Cancel"
                                                    }
                                                }

                                                room_delete_button = <RobrixIconButton> {
                                                    padding: 6,
                                                    draw_bg: {
                                                        color: #00000000,
                                                        color_hover: #00000000,
                                                        border_size: 0.0
                                                    },
                                                    draw_icon: {
                                                        svg_file: (ICON_TRASH),
                                                        color: (COLOR_FG_DANGER_RED)
                                                    }
                                                    icon_walk: {width: 12, height: 12}
                                                }
                                            }
                                        }

                                        room_card_incident = <RoundedView> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 8,
                                            padding: 12,

                                            show_bg: true,
                                            draw_bg: {
                                                color: #F6F6F6,
                                                border_radius: 8.0
                                            }

                                            room_header = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 8,
                                                align: {y: 0.5},

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "Standalone Room"
                                                }

                                                <Label> {
                                                    margin: {left: 6},
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 11},
                                                        color: #666666
                                                    },
                                                    text: "no workspace"
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            <RoundedView> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Right,
                                                spacing: 10,
                                                padding: {top: 6, bottom: 6, left: 8, right: 8},
                                                cursor: Hand,

                                                show_bg: true,
                                                draw_bg: {
                                                    instance hover: 0.0
                                                    color: #00000000
                                                    color_hover: #00000010
                                                    border_radius: 4.0
                                                    fn get_color(self) -> vec4 {
                                                        return mix(self.color, self.color_hover, self.hover);
                                                    }
                                                    fn pixel(self) -> vec4 {
                                                        let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                                                        sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, max(1.0, self.border_radius));
                                                        sdf.fill(self.get_color());
                                                        return sdf.result;
                                                    }
                                                }

                                                <Label> {
                                                    width: Fill,
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 12},
                                                        color: #000000,
                                                        wrap: Word
                                                    },
                                                    text: "#incident-ops"
                                                }

                                                model_select = <RobrixIconButton> {
                                                    width: 150,
                                                    padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                    draw_bg: {
                                                        color: #FFFFFF,
                                                        border_size: 1.0,
                                                        border_color: #DDDDDD,
                                                        border_radius: 4.0
                                                    }
                                                    draw_text: {
                                                        color: #000000
                                                        text_style: <REGULAR_TEXT> {font_size: 11}
                                                    }
                                                    text: "gpt-4o-mini ▾"
                                                }

                                                model_actions = <View> {
                                                    width: Fit,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 6,
                                                    visible: false

                                                    save_button = <RobrixIconButton> {
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                        draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                        text: "Save"
                                                    }

                                                    cancel_button = <RobrixIconButton> {
                                                        padding: {top: 6, bottom: 6, left: 10, right: 10}
                                                        draw_bg: { color: (COLOR_SECONDARY) }
                                                        draw_text: { color: (MESSAGE_TEXT_COLOR) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                        text: "Cancel"
                                                    }
                                                }

                                                room_delete_button = <RobrixIconButton> {
                                                    padding: 6,
                                                    draw_bg: {
                                                        color: #00000000,
                                                        color_hover: #00000000,
                                                        border_size: 0.0
                                                    },
                                                    draw_icon: {
                                                        svg_file: (ICON_TRASH),
                                                        color: (COLOR_FG_DANGER_RED)
                                                    }
                                                    icon_walk: {width: 12, height: 12}
                                                }
                                            }
                                        }

                                    }
                                }
                            robit_content_agent = <View> {
                                visible: false
                                width: Fill,
                                height: Fill,
                                flow: Right,
                                spacing: 12,

                                provider_list = <RoundedView> {
                                    width: 170,
                                    height: Fill,
                                    flow: Down,
                                    spacing: 8,
                                    padding: 10,

                                    show_bg: true,
                                    draw_bg: {
                                        color: #F2F2F2,
                                        border_radius: 8.0
                                    }

                                    <Label> {
                                        draw_text: {
                                            text_style: <THEME_FONT_BOLD>{font_size: 12},
                                            color: #000000
                                        },
                                        text: "Providers"
                                    }



                                    provider_item_deepseek = <RoundedView> {
                                        width: Fill,
                                        height: Fit,
                                        flow: Overlay,
                                        show_bg: true,
                                        draw_bg: {
                                            color: #FFFFFF,
                                            border_radius: 6.0
                                        }

                                        content = <View> {
                                            width: Fill,
                                            height: Fit,
                                            flow: Right,
                                            spacing: 8,
                                            padding: {top: 8, bottom: 8, left: 8, right: 8},
                                            align: {y: 0.5},

                <Image> {
                    width: 18,
                    height: 18,
                    align: {y: 0.5},
                    source: (IMG_PROVIDER_DEEPSEEK)
                }

                <Label> {
                    align: {y: 0.5},
                    draw_text: {
                        text_style: <REGULAR_TEXT>{font_size: 11},
                        color: #000000
                                                },
                                                text: "DeepSeek"
                                            }
                                        }

                                        provider_item_deepseek_button = <RobrixIconButton> {
                                            width: Fill,
                                            height: Fill,
                                            padding: 0,
                                            cursor: Hand,
                                            draw_bg: {
                                                color: #00000000,
                                                color_hover: #00000000,
                                                border_size: 0.0
                                            }
                                            draw_text: { color: #00000000 }
                                            text: ""
                                        }
                                    }
                                }

                                provider_detail = <RoundedView> {
                                    width: Fill,
                                    height: Fill,
                                    flow: Down,
                                    padding: 12,

                                    show_bg: true,
                                    draw_bg: {
                                        color: #F7F7F7,
                                        border_radius: 8.0
                                    }

                                    detail_scroll = <ScrollXYView> {
                                        width: Fill,
                                        height: Fill,
                                        flow: Down,
                                        spacing: 12,



                                        provider_detail_deepseek = <View> {
                                            visible: true
                                            width: Fill,
                                            height: Fit,
                                            flow: Down,
                                            spacing: 12,

                                            header_row = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 6,

                                                header_title_row = <View> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 8,
                                                    align: {y: 0.5},

                                                    <Label> {
                                                        draw_text: {
                                                            text_style: <THEME_FONT_BOLD>{font_size: 13},
                                                            color: #000000
                                                        },
                                                        text: "DeepSeek"
                                                    }

                                                    <Label> {
                                                        margin: {left: 6},
                                                        draw_text: {
                                                            text_style: <REGULAR_TEXT>{font_size: 11},
                                                            color: #666666
                                                        },
                                                        text: "Type: DeepSeek"
                                                    }
                                                }

                                                header_actions_row = <View> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 10,
                                                    align: {y: 0.5},

                                                    <FillerX> {}

                                                    refresh_button = <View> {
                                                        visible: false
                                                        cursor: Hand
                                                        width: 22,
                                                        height: 22,
                                                        align: {y: 0.5},

                                                        refresh_icon = <Image> {
                                                            width: 16,
                                                            height: 16,
                                                            source: (IMG_SYNC_REFRESH)
                                                        }
                                                    }

                                                    provider_enabled_switch = <RobitSwitch> {
                                                        width: 34,
                                                        height: 18
                                                    }
                                                }
                                            }

                                            <LineH> { padding: 6 }

                                            api_host_group = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 6,

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "API Host"
                                                }

                                                api_host_input = <RobrixTextInput> {
                                                    width: Fill,
                                                    height: Fit,
                                                    text: "https://api.deepseek.com/v1",
                                                    draw_bg: {
                                                        color: #FFFFFF,
                                                        border_size: 1.0,
                                                        border_color: #DDDDDD,
                                                        border_radius: 4.0
                                                    }
                                                    draw_text: {
                                                        color: #000000
                                                        text_style: <REGULAR_TEXT> {font_size: 11}
                                                    }
                                                }
                                            }

                                            api_key_group = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 6,

                                                <Label> {
                                                    draw_text: {
                                                        text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                        color: #000000
                                                    },
                                                    text: "API Key"
                                                }

                                                api_key_row = <View> {
                                                    width: Fill,
                                                    height: Fit,
                                                    flow: Right,
                                                    spacing: 6,
                                                    align: {y: 0.5},

                                                    api_key_input = <RobrixTextInput> {
                                                        width: Fill,
                                                        height: Fit,
                                                        text: "",
                                                        empty_text: "sk-deepseek-********************************",
                                                        is_password: true,
                                                        draw_bg: {
                                                            color: #FFFFFF,
                                                            border_size: 1.0,
                                                            border_color: #DDDDDD,
                                                            border_radius: 4.0
                                                        }
                                                        draw_text: {
                                                            color: #000000
                                                            text_style: <REGULAR_TEXT> {font_size: 11}
                                                        }
                                                    }

                                                    toggle_key_visibility = <RobrixIconButton> {
                                                        padding: 6,
                                                        draw_bg: {
                                                            color: #00000000,
                                                            color_hover: #00000000,
                                                            border_size: 0.0
                                                        }
                                                        draw_icon: {
                                                            svg_file: (ICON_VIEW_SOURCE),
                                                            color: #666666
                                                        }
                                                        icon_walk: {width: 12, height: 12}
                                                    }
                                                }

                                            }

                                            save_provider_button = <RobrixIconButton> {
                                                width: Fit,
                                                padding: {top: 6, bottom: 6, left: 12, right: 12}
                                                draw_bg: { color: (COLOR_ACTIVE_PRIMARY) }
                                                draw_text: { color: (COLOR_PRIMARY) text_style: <REGULAR_TEXT>{font_size: 11} }
                                                text: "Save"
                                            }

                                            <LineH> { padding: 6 }

                                            <Label> {
                                                draw_text: {
                                                    text_style: <THEME_FONT_BOLD>{font_size: 12},
                                                    color: #000000
                                                },
                                                text: "Models"
                                            }

                                            models_status_label = <Label> {
                                                width: Fill,
                                                draw_text: {
                                                    text_style: <REGULAR_TEXT>{font_size: 10},
                                                    color: #666666
                                                },
                                                text: "Haven't synchronized models since app launch"
                                            }

                                            models_list = <View> {
                                                width: Fill,
                                                height: Fit,
                                                flow: Down,
                                                spacing: 6,

                                                model_row_0 = <RobitModelRow> {}
                                                model_row_1 = <RobitModelRow> {}
                                                model_row_2 = <RobitModelRow> {}
                                                model_row_3 = <RobitModelRow> {}
                                                model_row_4 = <RobitModelRow> {}
                                                model_row_5 = <RobitModelRow> {}
                                                model_row_6 = <RobitModelRow> {}
                                                model_row_7 = <RobitModelRow> {}
                                                model_row_8 = <RobitModelRow> {}
                                                model_row_9 = <RobitModelRow> {}

                                                models_overflow_label = <Label> {
                                                    visible: false
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 10},
                                                        color: #666666
                                                    },
                                                    text: ""
                                                }

                                                models_empty_label = <Label> {
                                                    visible: false
                                                    draw_text: {
                                                        text_style: <REGULAR_TEXT>{font_size: 10},
                                                        color: #666666
                                                    },
                                                    text: "No models synced yet"
                                                }
                                            }
                                        }
                                    }
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
    #[rust] robit_provider_show_key_deepseek: bool,
    #[rust] robit_provider_models: Vec<String>,
    #[rust] robit_provider_sync_status: RobitProviderSyncStatus,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
enum RobitModalTab {
    #[default]
    All,
    Agent,
    About,
}

#[derive(Clone, Debug, PartialEq)]
enum RobitProviderSyncStatus {
    Disconnected,
    Connecting,
    Synced,
    Error(String),
}

impl Default for RobitProviderSyncStatus {
    fn default() -> Self {
        RobitProviderSyncStatus::Disconnected
    }
}

#[derive(Clone, Debug, DefaultNone)]
enum RobitProviderSyncAction {
    None,
    Success(Vec<String>),
    Failure(String),
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
                self.robit_provider_show_key_deepseek = false;
                self.reset_robit_provider_state(cx);
                self.view
                    .text_input(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.api_key_row.api_key_input))
                    .apply_over(cx, live! { is_password: true });
                self.view
                    .check_box(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.header_row.header_actions_row.provider_enabled_switch))
                    .set_active(cx, false);
                self.view
                    .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.header_row.header_actions_row.refresh_button))
                    .set_visible(cx, false);
                self.view.modal(ids!(robit_modal)).open(cx);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_nav.robit_nav_all_button)).clicked(actions) {
                self.set_robit_modal_tab(cx, RobitModalTab::All);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_nav.robit_nav_agent_button)).clicked(actions) {
                self.set_robit_modal_tab(cx, RobitModalTab::Agent);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_nav.robit_nav_about_button)).clicked(actions) {
                self.set_robit_modal_tab(cx, RobitModalTab::About);
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.api_key_row.toggle_key_visibility)).clicked(actions) {
                self.robit_provider_show_key_deepseek = !self.robit_provider_show_key_deepseek;
                self.view
                    .text_input(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.api_key_row.api_key_input))
                    .apply_over(cx, live! { is_password: (!self.robit_provider_show_key_deepseek) });
            }

            let provider_enabled_switch = self
                .view
                .check_box(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.header_row.header_actions_row.provider_enabled_switch));
            if let Some(enabled) = provider_enabled_switch.changed(actions) {
                self.view
                    .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.header_row.header_actions_row.refresh_button))
                    .set_visible(cx, enabled);
            }

            if self
                .view
                .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.header_row.header_actions_row.refresh_button))
                .finger_up(actions)
                .is_some()
            {
                self.start_deepseek_model_sync(cx);
            }

            for action in actions {
                if let Some(sync_action) = action.downcast_ref::<RobitProviderSyncAction>() {
                    match sync_action {
                        RobitProviderSyncAction::Success(models) => {
                            self.robit_provider_models = models.clone();
                            self.set_robit_provider_sync_status(cx, RobitProviderSyncStatus::Synced);
                            self.apply_robit_provider_models(cx);
                        }
                        RobitProviderSyncAction::Failure(error) => {
                            self.set_robit_provider_sync_status(
                                cx,
                                RobitProviderSyncStatus::Error(error.clone()),
                            );
                        }
                        RobitProviderSyncAction::None => {}
                    }
                }
            }

            if self.view.button(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_nav.robit_nav_close_button)).clicked(actions)
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
            .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_all))
            .set_visible(cx, show_all);
        self.view
            .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent))
            .set_visible(cx, show_agent);
        self.view
            .view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_about))
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
            ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_nav.robit_nav_all_button),
            self.robit_modal_tab == RobitModalTab::All,
        );
        apply_style(
            ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_nav.robit_nav_agent_button),
            self.robit_modal_tab == RobitModalTab::Agent,
        );
        apply_style(
            ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_nav.robit_nav_about_button),
            self.robit_modal_tab == RobitModalTab::About,
        );
    }

    fn reset_robit_provider_state(&mut self, cx: &mut Cx) {
        self.robit_provider_models = vec![
            "deepseek-chat".to_string(),
            "deepseek-reasoner".to_string(),
            "deepseek-coder".to_string(),
        ];
        self.set_robit_provider_sync_status(cx, RobitProviderSyncStatus::Disconnected);
        self.apply_robit_provider_models(cx);
    }

    fn set_robit_provider_sync_status(&mut self, cx: &mut Cx, status: RobitProviderSyncStatus) {
        self.robit_provider_sync_status = status.clone();
        let status_label = self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_status_label));
        match status {
            RobitProviderSyncStatus::Disconnected => {
                status_label.set_text(cx, "Haven't synchronized models since app launch");
                status_label.apply_over(cx, live!{
                    draw_text: { color: (COLOR_FG_DISABLED) }
                });
            }
            RobitProviderSyncStatus::Connecting => {
                status_label.set_text(cx, "Connecting...");
                status_label.apply_over(cx, live!{
                    draw_text: { color: (COLOR_FG_DISABLED) }
                });
            }
            RobitProviderSyncStatus::Synced => {
                status_label.set_text(cx, "Models synchronized");
                status_label.apply_over(cx, live!{
                    draw_text: { color: (COLOR_FG_ACCEPT_GREEN) }
                });
            }
            RobitProviderSyncStatus::Error(message) => {
                status_label.set_text(cx, &message);
                status_label.apply_over(cx, live!{
                    draw_text: { color: (COLOR_FG_DANGER_RED) }
                });
            }
        }
    }

    fn apply_robit_provider_models(&mut self, cx: &mut Cx) {
        let models = self.robit_provider_models.clone();
        let mut rendered = 0;
        for (index, model) in models.iter().enumerate() {
            if index >= ROBIT_MODEL_ROW_COUNT {
                break;
            }
            if !self.set_robit_model_row(cx, index, Some(model)) {
                break;
            }
            rendered += 1;
        }

        for index in rendered..ROBIT_MODEL_ROW_COUNT {
            self.set_robit_model_row(cx, index, None);
        }

        let overflow = models.len().saturating_sub(ROBIT_MODEL_ROW_COUNT);
        let overflow_label = self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.models_overflow_label));
        if overflow > 0 {
            overflow_label.set_text(cx, &format!("and {overflow} more"));
            overflow_label.set_visible(cx, true);
        } else {
            overflow_label.set_visible(cx, false);
        }

        let empty_label = self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.models_empty_label));
        empty_label.set_visible(cx, models.is_empty());
    }

    fn set_robit_model_row(&mut self, cx: &mut Cx, index: usize, model_name: Option<&str>) -> bool {
        let visible = model_name.is_some();
        match index {
            0 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_0)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_0.model_name)).set_text(cx, name);
                }
            }
            1 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_1)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_1.model_name)).set_text(cx, name);
                }
            }
            2 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_2)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_2.model_name)).set_text(cx, name);
                }
            }
            3 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_3)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_3.model_name)).set_text(cx, name);
                }
            }
            4 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_4)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_4.model_name)).set_text(cx, name);
                }
            }
            5 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_5)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_5.model_name)).set_text(cx, name);
                }
            }
            6 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_6)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_6.model_name)).set_text(cx, name);
                }
            }
            7 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_7)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_7.model_name)).set_text(cx, name);
                }
            }
            8 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_8)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_8.model_name)).set_text(cx, name);
                }
            }
            9 => {
                self.view.view(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_9)).set_visible(cx, visible);
                if let Some(name) = model_name {
                    self.view.label(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.models_list.model_row_9.model_name)).set_text(cx, name);
                }
            }
            _ => return false,
        }
        true
    }

    fn start_deepseek_model_sync(&mut self, cx: &mut Cx) {
        let api_host = self
            .view
            .text_input(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.api_host_group.api_host_input))
            .text()
            .trim()
            .to_string();
        let api_key = self
            .view
            .text_input(ids!(robit_modal_inner.robit_modal_body.robit_modal_main.robit_modal_content.robit_content_agent.provider_detail.detail_scroll.provider_detail_deepseek.api_key_row.api_key_input))
            .text()
            .trim()
            .to_string();

        if api_host.is_empty() {
            self.set_robit_provider_sync_status(
                cx,
                RobitProviderSyncStatus::Error("Missing API host".to_string()),
            );
            return;
        }

        if api_key.is_empty() {
            self.set_robit_provider_sync_status(
                cx,
                RobitProviderSyncStatus::Error("Missing API key".to_string()),
            );
            return;
        }

        self.set_robit_provider_sync_status(cx, RobitProviderSyncStatus::Connecting);

        cx.spawn_thread(move || {
            let runtime = Builder::new_current_thread().enable_all().build();
            let result = match runtime {
                Ok(rt) => rt.block_on(fetch_deepseek_models(api_host, api_key)),
                Err(err) => Err(err.to_string()),
            };
            match result {
                Ok(models) => Cx::post_action(RobitProviderSyncAction::Success(models)),
                Err(error) => Cx::post_action(RobitProviderSyncAction::Failure(error)),
            }
            SignalToUI::set_ui_signal();
        });
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

#[derive(Deserialize)]
struct DeepSeekModelsResponse {
    data: Vec<DeepSeekModelItem>,
}

#[derive(Deserialize)]
struct DeepSeekModelItem {
    id: String,
}

async fn fetch_deepseek_models(api_host: String, api_key: String) -> Result<Vec<String>, String> {
    let base = api_host.trim_end_matches('/');
    let url = format!("{base}/models");
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?;
    let payload = response
        .json::<DeepSeekModelsResponse>()
        .await
        .map_err(|err| err.to_string())?;
    let mut models: Vec<String> = payload.data.into_iter().map(|model| model.id).collect();
    models.sort();
    Ok(models)
}

impl SettingsScreenRef {
    /// See [`SettingsScreen::populate()`].
    pub fn populate(&self, cx: &mut Cx, own_profile: Option<UserProfile>) {
        let Some(mut inner) = self.borrow_mut() else { return; };
        inner.populate(cx, own_profile);
    }
}
