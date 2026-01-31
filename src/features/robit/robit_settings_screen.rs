use makepad_widgets::*;

live_design! {
    link robit_enabled

    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::helpers::*;
    use crate::shared::icon_button::*;
    use crate::shared::styles::*;

    pub RobitSettingsScreen = <View> {
        width: Fill, height: Fit
        flow: Down
        align: {x: 0}

        <TitleLabel> {
            text: "Robit Settings"
        }

        <Label> {
            width: Fill, height: Fit
            flow: RightWrap,
            align: {x: 0}
            margin: {top: 10, bottom: 8}
            draw_text: {
                wrap: Word,
                color: (MESSAGE_TEXT_COLOR),
                text_style: <MESSAGE_TEXT_STYLE>{ font_size: 11 },
            }
            text: "Robit features are enabled in this build."
        }

        robit_open_button = <RobrixIconButton> {
            padding: {top: 10, bottom: 10, left: 12, right: 15}
            margin: {left: 5}
            draw_bg: {
                color: (COLOR_ACTIVE_PRIMARY)
            }
            draw_icon: {
                svg_file: (ICON_EXTERNAL_LINK)
                color: (COLOR_PRIMARY)
            }
            draw_text: {
                color: (COLOR_PRIMARY)
                text_style: <REGULAR_TEXT> {}
            }
            icon_walk: {width: 16, height: 16}
            text: "Robit Settings"
        }
    }
}
