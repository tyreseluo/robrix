use makepad_widgets::*;

use crate::{
    botfather::{self, BotfatherAction},
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

    BotSelectorDropDown = <DropDown> {
        width: Fill
        height: Fit
        popup_menu_position: BelowInput
        labels: ["No bots configured"]
        draw_bg: {
            border_radius: 6.0
            color: (COLOR_PRIMARY)
            color_hover: (COLOR_BG_PREVIEW)
            color_focus: (COLOR_BG_PREVIEW)
            color_down: (COLOR_BG_PREVIEW)
            border_color: (COLOR_SECONDARY)
            border_color_hover: (COLOR_SECONDARY)
            border_color_focus: (COLOR_ACTIVE_PRIMARY)
            border_color_down: (COLOR_ACTIVE_PRIMARY)
            border_color_2: (COLOR_SECONDARY)
            border_color_2_hover: (COLOR_SECONDARY)
            border_color_2_focus: (COLOR_ACTIVE_PRIMARY)
            border_color_2_down: (COLOR_ACTIVE_PRIMARY)
        }
        draw_text: {
            color: (COLOR_TEXT)
        }
    }

    pub BotfatherRoomPanel = {{BotfatherRoomPanel}} {
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
                text: "Bot Room Panel"
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
            text: "This panel manages the room's active bot binding. Pick the room bot from the dropdown below. If both runtimes are available and you have not overridden the room, Crew still wins by default."
        }

        binding_summary_label = <Label> {
            width: Fill, height: Fit
            flow: RightWrap
            draw_text: {
                wrap: Word
                text_style: <REGULAR_TEXT>{font_size: 10.5}
                color: (COLOR_TEXT)
            }
            text: "No bot binding resolved yet."
        }

        <View> {
            width: Fill, height: Fit
            flow: Down
            spacing: 8

            <SubsectionLabel> {
                text: "Room Bot"
            }

            bot_selector_dropdown = <BotSelectorDropDown> {}
        }

        <View> {
            width: Fill, height: Fit
            flow: RightWrap
            spacing: 10

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
pub struct BotfatherRoomPanel {
    #[deref]
    view: View,
    #[rust]
    bot_choice_ids: Vec<String>,
}

#[derive(Clone, Debug, DefaultNone)]
pub enum BotfatherRoomPanelAction {
    CloseRequested,
    None,
}

impl Widget for BotfatherRoomPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if matches!(event, Event::Signal) {
            let _ = botfather::ensure_loaded_for_current_user();
            let _ = botfather::refresh_inventory_from_rooms_list(cx);
            self.refresh_room_state(cx, current_room_id(scope));
        }

        if let Event::Actions(actions) = event {
            let current_room_id = current_room_id(scope);
            let bot_selector_dropdown = self.drop_down(ids!(bot_selector_dropdown));
            let unbind_room_button = self.view.button(ids!(unbind_room_button));
            let healthcheck_button = self.view.button(ids!(healthcheck_button));
            let close_button = self.view.button(ids!(close_button));

            if let Some(selected_index) = bot_selector_dropdown.selected(actions) {
                if let Some(bot_id) = self.bot_choice_ids.get(selected_index).cloned() {
                    match current_room_id.as_deref() {
                        Some(room_id) => match botfather::bind_room_to_bot(room_id, &bot_id) {
                            Ok(message) => {
                                self.refresh_room_state(cx, current_room_id.clone());
                                self.set_status(cx, &message);
                            }
                            Err(error) => self.set_status(cx, &error),
                        },
                        None => self.set_status(cx, "This room is not ready for BotFather yet."),
                    }
                } else if self.bot_choice_ids.is_empty() {
                    self.set_status(
                        cx,
                        "Configure at least one bot in BotFather Settings first.",
                    );
                }
            }

            if unbind_room_button.clicked(actions) {
                match current_room_id.as_deref() {
                    Some(room_id) => match botfather::unbind_room(room_id) {
                        Ok(()) => {
                            self.refresh_room_state(cx, current_room_id.clone());
                            self.set_status(cx, "Removed the room-level bot override.");
                        }
                        Err(error) => self.set_status(cx, &error),
                    },
                    None => self.set_status(cx, "This room is not ready for BotFather yet."),
                }
            }

            if healthcheck_button.clicked(actions) {
                match current_room_id.clone() {
                    Some(room_id) => match botfather::run_room_healthcheck(room_id) {
                        Ok(()) => self.set_status(cx, "Running bot runtime healthcheck..."),
                        Err(error) => self.set_status(cx, &error),
                    },
                    None => self.set_status(cx, "This room is not ready for BotFather yet."),
                }
            }

            if close_button.clicked(actions) {
                cx.widget_action(
                    self.widget_uid(),
                    &scope.path,
                    BotfatherRoomPanelAction::CloseRequested,
                );
            }

            for action in actions {
                if let Some(LoginAction::LoginSuccess) = action.downcast_ref() {
                    let _ = botfather::ensure_loaded_for_current_user();
                    self.refresh_room_state(cx, current_room_id.clone());
                    continue;
                }

                if let Some(LogoutAction::ClearAppState { .. }) = action.downcast_ref() {
                    self.clear(cx);
                    continue;
                }

                if let Some(BotfatherAction::StateChanged) = action.downcast_ref() {
                    self.refresh_room_state(cx, current_room_id.clone());
                    continue;
                }

                if let Some(BotfatherAction::Status(status)) = action.downcast_ref() {
                    self.set_status(cx, status);
                    continue;
                }

                if let Some(BotfatherAction::HealthcheckFinished { room_id, result }) =
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

impl BotfatherRoomPanel {
    fn refresh_room_state(&mut self, cx: &mut Cx, room_id: Option<String>) {
        match room_id {
            Some(room_id) => {
                self.view
                    .label(ids!(binding_summary_label))
                    .set_text(cx, &botfather::describe_room_binding(&room_id));
                self.update_bot_selector(cx, Some(room_id.as_str()));
            }
            None => {
                self.view.label(ids!(binding_summary_label)).set_text(
                    cx,
                    "This room is not ready yet. Open it again after Robrix finishes loading.",
                );
                self.update_bot_selector(cx, None);
            }
        }
    }

    fn clear(&mut self, cx: &mut Cx) {
        self.view
            .label(ids!(binding_summary_label))
            .set_text(cx, "No bot binding resolved yet.");
        self.view.label(ids!(status_label)).set_text(cx, "");
        self.update_bot_selector(cx, None);
    }

    fn set_status(&mut self, cx: &mut Cx, status: &str) {
        self.view.label(ids!(status_label)).set_text(cx, status);
    }

    fn update_bot_selector(&mut self, cx: &mut Cx, room_id: Option<&str>) {
        let dropdown = self.drop_down(ids!(bot_selector_dropdown));
        let options = match room_id {
            Some(room_id) => botfather::room_bot_options(Some(room_id)),
            None => Vec::new(),
        };

        if options.is_empty() {
            self.bot_choice_ids.clear();
            let placeholder = if room_id.is_some() {
                "No bots configured"
            } else {
                "Room unavailable"
            };
            dropdown.set_labels(cx, vec![placeholder.to_string()]);
            dropdown.set_selected_item(cx, 0);
            return;
        }

        self.bot_choice_ids = options.iter().map(|option| option.bot_id.clone()).collect();
        dropdown.set_labels(
            cx,
            options
                .iter()
                .map(|option| option.label.clone())
                .collect::<Vec<_>>(),
        );

        let selected_index = room_id
            .and_then(botfather::room_primary_bot_id)
            .and_then(|bot_id| {
                self.bot_choice_ids
                    .iter()
                    .position(|candidate| candidate == &bot_id)
            })
            .unwrap_or(0);
        dropdown.set_selected_item(cx, selected_index);
    }
}

fn current_room_id(scope: &mut Scope) -> Option<String> {
    scope
        .props
        .get::<RoomScreenProps>()
        .map(|props| props.room_name_id.room_id().to_string())
}
