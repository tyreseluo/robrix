//! Dummy Robit-related widgets used when the `robit` feature is disabled.

use makepad_widgets::*;

live_design! {
    link robit_disabled

    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::helpers::*;
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
            margin: {top: 10, bottom: 10}
            draw_text: {
                wrap: Word,
                color: (MESSAGE_TEXT_COLOR),
                text_style: <MESSAGE_TEXT_STYLE>{ font_size: 11 },
            }
            text: "Robit features are not included in this build.\nTo use Robit, build Robrix with the 'robit' feature enabled."
        }
    }
}
