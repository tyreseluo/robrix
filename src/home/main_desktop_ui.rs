use makepad_widgets::*;
use matrix_sdk::{RoomState, ruma::OwnedRoomId};
use ruma::OwnedRoomAliasId;
use tokio::sync::Notify;
use std::{collections::HashMap, sync::Arc};

use crate::{app::{AppState, AppStateAction, SelectedRoom}, room::{RoomAliasAction, RoomPreviewAction, loading_screen::{LoadingStatus, LoadingTabState, LoadingType, RoomLoadingScreenAction}, preview_screen::PreviewScreenWidgetRefExt}, shared::popup_list::{PopupItem, PopupKind, enqueue_popup_notification}, sliding_sync::{MatrixRequest, get_client, submit_async_request}, utils::room_name_or_id};
use super::{invite_screen::InviteScreenWidgetRefExt, room_screen::RoomScreenWidgetRefExt, rooms_list::RoomsListAction};

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::styles::*;
    use crate::home::light_themed_dock::*;
    use crate::home::rooms_sidebar::RoomsSideBar;
    use crate::home::welcome_screen::WelcomeScreen;
    use crate::home::room_screen::RoomScreen;
    use crate::home::invite_screen::InviteScreen;
    use crate::room::loading_screen::LoadingScreen;
    use crate::room::preview_screen::PreviewScreen;

    pub MainDesktopUI = {{MainDesktopUI}} {
        dock = <Dock> {
            width: Fill,
            height: Fill,
            padding: 0,
            spacing: 0,
            // Align the dock with the RoomFilterInputBar. Not sure why we need this...
            margin: {left: 1.75}


            root = Splitter {
                axis: Horizontal,
                align: FromA(300.0),
                a: rooms_sidebar_tab,
                b: main
            }

            // This is a "fixed" tab with no header that cannot be closed.
            rooms_sidebar_tab = Tab {
                name: "" // show no tab header
                kind: rooms_sidebar // this template is defined below.
            }

            main = Tabs{tabs:[home_tab], selected:0}

            home_tab = Tab {
                name: "Home"
                kind: welcome_screen
                template: PermanentTab
            }

            // Below are the templates of widgets that can be created within dock tabs.
            rooms_sidebar = <RoomsSideBar> {}
            welcome_screen = <WelcomeScreen> {}
            room_screen = <RoomScreen> {}
            invite_screen = <InviteScreen> {}
            loading_screen = <LoadingScreen> {}
            preview_screen = <PreviewScreen> {}
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct MainDesktopUI {
    #[deref]
    view: View,

    /// The rooms that are currently open, keyed by the LiveId of their tab.
    #[rust]
    open_rooms: HashMap<LiveId, SelectedRoom>,

    /// The tab that should be closed in the next draw event
    #[rust]
    tab_to_close: Option<LiveId>,

    /// The order in which the rooms were opened, in chronological order
    /// from first opened (at the beginning) to last opened (at the end).
    #[rust]
    room_order: Vec<SelectedRoom>,

    /// The most recently selected room, used to prevent re-selecting the same room in Dock
    /// which would trigger redraw of whole Widget.
    #[rust]
    most_recently_selected_room: Option<SelectedRoom>,

    /// Boolean to indicate if we've drawn the MainDesktopUi previously in the desktop view.
    ///
    /// When switching mobile view to desktop, we need to restore the app state.
    /// If false, this widget emits an action to load the dock from the saved dock state.
    /// If true, this widget proceeds to draw the desktop UI as normal.
    #[rust]
    drawn_previously: bool,

    #[rust]
    loading_tab_counter: usize,

    #[rust]
    loading_tabs: HashMap<LiveId, LoadingTabState>,

    #[rust]
    resolved_room_aliases: HashMap<OwnedRoomAliasId, OwnedRoomId>,
}

impl Widget for MainDesktopUI {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.widget_match_event(cx, event, scope); // invokes `WidgetMatchEvent` impl
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // When changing from mobile to Desktop, we need to restore the app state.
        if !self.drawn_previously {
            let app_state = scope.data.get_mut::<AppState>().unwrap();
            if !app_state.saved_dock_state.open_rooms.is_empty() {
                cx.action(MainDesktopUiAction::LoadDockFromAppState);
            }
            self.drawn_previously = true;
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl MainDesktopUI {
    /// Focuses on a room if it is already open, otherwise creates a new tab for the room.
    fn focus_or_create_tab(&mut self, cx: &mut Cx, room: SelectedRoom) {
        let dock = self.view.dock(ids!(dock));

        // Do nothing if the room to select is already created and focused.
        if self.most_recently_selected_room.as_ref().is_some_and(|r| r == &room) {
            return;
        }

        // If the room is already open, select (jump to) its existing tab
        let room_id_as_live_id = LiveId::from_str(room.room_id().as_str());
        if self.open_rooms.contains_key(&room_id_as_live_id) {
            dock.select_tab(cx, room_id_as_live_id);
            self.most_recently_selected_room = Some(room);
            return;
        }

        // Create a new tab for the room
        let (tab_bar, _pos) = dock.find_tab_bar_of_tab(id!(home_tab)).unwrap();
        let (kind, name) = match &room {
            SelectedRoom::JoinedRoom { room_id, room_name }  => (
                id!(room_screen),
                room_name_or_id(room_name.as_ref(), room_id),
            ),
            SelectedRoom::InvitedRoom { room_id, room_name } => (
                id!(invite_screen),
                room_name_or_id(room_name.as_ref(), room_id),
            ),
            _ => (LiveId::empty(), "".into()), // should not happen
        };
        let new_tab_widget = dock.create_and_select_tab(
            cx,
            tab_bar,
            room_id_as_live_id,
            kind,
            name,
            id!(CloseableTab),
            None, // insert the tab at the end
            // TODO: insert the tab after the most-recently-selected room
        );

        // if the tab was created, set the room screen and add the room to the room order
        if let Some(new_widget) = new_tab_widget {
            self.room_order.push(room.clone());
            match &room {
                SelectedRoom::JoinedRoom { room_id, .. }  => {
                    new_widget.as_room_screen().set_displayed_room(
                        cx,
                        room_id.clone().into(),
                        room.room_name().cloned(),
                    );
                }
                SelectedRoom::InvitedRoom { room_id, room_name: _ } => {
                    new_widget.as_invite_screen().set_displayed_invite(
                        cx,
                        room_id.clone().into(),
                        room.room_name().cloned()
                    );
                }
                _ => {}
            }
            cx.action(MainDesktopUiAction::SaveDockIntoAppState);
        } else {
            error!("BUG: failed to create tab for {room:?}");
        }

        self.open_rooms.insert(room_id_as_live_id, room.clone());
        self.most_recently_selected_room = Some(room);
    }

    /// Closes a tab in the dock and focuses on the latest open room.
    fn close_tab(&mut self, cx: &mut Cx, tab_id: LiveId) {
        let dock = self.view.dock(ids!(dock));
        if let Some(room_being_closed) = self.open_rooms.get(&tab_id) {
            self.room_order.retain(|sr| sr != room_being_closed);

            if self.open_rooms.len() > 1 {
                // If the closing tab is the active one, then focus the next room
                let active_room = self.most_recently_selected_room.as_ref();
                if let Some(active_room) = active_room {
                    if active_room == room_being_closed {
                        if let Some(new_focused_room) = self.room_order.last() {
                            // notify the app state about the new focused room
                            cx.action(AppStateAction::RoomFocused(new_focused_room.clone()));

                            // Set the new selected room to be used in the current draw
                            self.most_recently_selected_room = Some(new_focused_room.clone());
                        }
                    }
                }
            } else {
                // If there is no room to focus, notify app to reset the selected room in the app state
                cx.action(AppStateAction::FocusNone);
                dock.select_tab(cx, id!(home_tab));
                self.most_recently_selected_room = None;
            }
        }

        dock.close_tab(cx, tab_id);

        if self.loading_tabs.remove(&tab_id).is_some() {
            log!("Removed loading tab state for closed tab {}", tab_id);
        }

        self.tab_to_close = None;
        self.open_rooms.remove(&tab_id);
    }

    /// Closes all tabs
    pub fn close_all_tabs(&mut self, cx: &mut Cx) {
        let dock = self.view.dock(ids!(dock));
        for tab_id in self.open_rooms.keys() {        
            dock.close_tab(cx, *tab_id);
        }

        dock.select_tab(cx, id!(home_tab));
        cx.action(AppStateAction::FocusNone);

        // Clear tab-related dock UI state.
        self.open_rooms.clear();
        self.tab_to_close = None;
        self.room_order.clear();
        self.most_recently_selected_room = None;
    }

    /// Replaces an invite with a joined room in the dock.
    fn replace_invite_with_joined_room(
        &mut self,
        cx: &mut Cx,
        _scope: &mut Scope,
        room_id: OwnedRoomId,
        room_name: Option<String>,
    ) {
        let dock = self.view.dock(ids!(dock));
        let Some((new_widget, true)) = dock.replace_tab(
            cx,
            LiveId::from_str(room_id.as_str()),
            id!(room_screen),
            Some(room_name_or_id(room_name.as_ref(), &room_id)),
            false,
        ) else {
            // Nothing we can really do here except log an error.
            error!("BUG: failed to replace InviteScreen tab with RoomScreen for {room_id}");
            return;
        };

        // Set the info to be displayed in the newly-replaced RoomScreen..
        new_widget.as_room_screen().set_displayed_room(
            cx,
            room_id.clone(),
            room_name.clone(),
        );

        // Go through all existing `SelectedRoom` instances and replace the
        // `SelectedRoom::InvitedRoom`s with `SelectedRoom::JoinedRoom`s.
        for selected_room in self.most_recently_selected_room.iter_mut()
            .chain(self.room_order.iter_mut())
            .chain(self.open_rooms.values_mut())
        {
            selected_room.upgrade_invite_to_joined(&room_id);
        }

        // Finally, emit an action to update the AppState with the new room.
        cx.action(AppStateAction::UpgradedInviteToJoinedRoom(room_id));
    }

    fn show_loading_tab(&mut self, cx: &mut Cx) -> Option<LiveId> {
        let dock = self.view.dock(ids!(dock));

        self.loading_tab_counter += 1;
        let loading_tab_id = LiveId::from_str(&format!("loading_tab_{}", self.loading_tab_counter));

        let (tab_bar, _pos) = dock.find_tab_bar_of_tab(id!(home_tab)).unwrap();
        let new_tab_widget = dock.create_and_select_tab(
            cx,
            tab_bar,
            loading_tab_id,
            id!(loading_screen),
            "Loading...".into(),
            id!(CloseableTab),
            None,
        );

        if new_tab_widget.is_none() {
            error!("BUG: failed to create loading tab");
            return None;
        }

        Some(loading_tab_id)
    }

    fn try_jump_to_known_room(&mut self, cx: &mut Cx, room_id: OwnedRoomId) -> bool {
        let uid = self.widget_uid();
        let dock = self.view.dock(ids!(dock));

        let room_tab_id = LiveId::from_str(room_id.as_str());
        if let Some(selected_room) = self.open_rooms.get(&room_tab_id) {
            // The room is already open, so just select its tab.
            dock.select_tab(cx, room_tab_id);
            self.most_recently_selected_room = Some(selected_room.clone());
            return true;
        }

        let Some(known_room) = get_client().and_then(|c| c.get_room(&room_id)) else {
            return false;
        };

        if known_room.is_space() {
            enqueue_popup_notification(PopupItem {
                message: "Not support space main page yet.".into(),
                kind: PopupKind::Info,
                auto_dismissal_duration: Some(3.0),
            });
            return true;
        }

        if known_room.is_tombstoned() {
            enqueue_popup_notification(PopupItem {
                message: format!(
                    "The room {} has been replaced and cannot be opened. \
                     Robrix does not support tombstoned rooms yet.",
                    room_id
                ),
                kind: PopupKind::Info,
                auto_dismissal_duration: Some(3.0),
            });
            return true;
        }

        match known_room.state() {
            RoomState::Joined => {
                cx.widget_action(
                    uid,
                    &Scope::empty().path,
                    RoomsListAction::Selected(SelectedRoom::JoinedRoom {
                        room_id: known_room.room_id().to_owned().into(),
                        room_name: known_room.name().clone(),
                    }),
                );
                return true;
            }
            RoomState::Invited => {
                cx.widget_action(
                    uid,
                    &Scope::empty().path,
                    RoomsListAction::Selected(SelectedRoom::InvitedRoom {
                        room_id: known_room.room_id().to_owned().into(),
                        room_name: known_room.name().clone(),
                    }),
                );
                return true;
            }
            _ => {
                log!("TODO: handle other room states when replacing loading tab for room ID: {}", room_id);
                return false
            }
        }
    }
}

impl WidgetMatchEvent for MainDesktopUI {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, scope: &mut Scope) {
        let mut should_save_dock_action: bool = false;
        for action in actions {
            let widget_action = action.as_widget_action();

            if let Some(MainDesktopUiAction::CloseAllTabs { on_close_all }) = action.downcast_ref() {
                self.close_all_tabs(cx);
                on_close_all.notify_one();
                continue;
            }

            if let Some(RoomAliasAction::ResolvedToRoomId { room_alias, result }) = action.downcast_ref() {
                let dock = self.view.dock(ids!(dock));
                // Find the loading tab that is waiting for this room
                let Some((loading_tab_id, state)) = self.loading_tabs.iter_mut()
                    .find_map(|(tab_id, state)| {
                        if state.room_alias_id.as_ref() == Some(room_alias) {
                            Some((tab_id.clone(), state))
                        } else {
                            None
                        }
                    })
                else {
                    error!("BUG: received room alias resolution for {room_alias} but no loading tab is waiting for it");
                    continue;
                };

                match result {
                    Ok(response) => {
                        let room_id = response.room_id.clone();
                        let via = response.servers.clone();
                        state.room_id = Some(room_id.clone());
                        state.via = via.clone();
                        self.resolved_room_aliases.insert(room_alias.clone(), room_id.clone());
                        if self.try_jump_to_known_room(cx, room_id.clone()) {
                            dock.close_tab(cx, loading_tab_id);
                        } else {
                            submit_async_request(MatrixRequest::GetRoomPreview {
                                room_or_alias_id: room_id.clone().into(),
                                via: via.clone(),
                            });
                        }
                    }
                    Err(err) => {
                        error!("Failed to resolve room alias {room_alias}: {err}");
                        state.loading_status = LoadingStatus::Failed {
                            title: Some("Failed to resolve room alias".into()),
                            details: Some(format!("Could not resolve room alias {}: {}", room_alias, err)),
                        };
                    }
                }
            }

            if let Some(RoomPreviewAction::Fetched(res)) = action.downcast_ref() {
                match res {
                    Ok(frp) => {
                        let dock = self.view.dock(ids!(dock));
                        let room_id = frp.room_id.clone();
                        let room_name = frp.name.clone();
                        let is_world_readable = frp.is_world_readable.unwrap_or(false); // default to false if unknown

                        let selected_room = SelectedRoom::PreviewRoom {
                            room_id: room_id.clone().into(),
                            room_name: room_name.clone(),
                        };

                        if self.most_recently_selected_room.as_ref().is_some_and(|r| r == &selected_room) {
                            // The room is already focused, so no need to create a new tab.
                            return;
                        }

                        let loading_tab_id = self.loading_tabs.iter()
                            .find_map(|(tab_id, state)| {
                                if state.room_id.as_ref() == Some(&room_id) {
                                    Some(tab_id.clone())
                                } else {
                                    None
                                }
                            })
                            .expect("BUG: no loading tab found for fetched room preview");

                        let _state = self.loading_tabs
                            .remove(&loading_tab_id)
                            .expect("BUG: expected loading tab state to exist");

                        dock.close_tab(cx, loading_tab_id);

                        let room_id_as_live_id = LiveId::from_str(room_id.as_str());
                        let (tab_bar, _pos) = dock.find_tab_bar_of_tab(id!(home_tab)).unwrap();
                        let new_tab_widget = dock.create_and_select_tab(
                            cx,
                            tab_bar,
                            room_id_as_live_id,
                            id!(preview_screen),
                            room_name_or_id(room_name.as_ref(), &room_id),
                            id!(CloseableTab),
                            None,
                        );

                        if let Some(new_widget) = new_tab_widget {
                            self.room_order.push(selected_room.clone());
                            if is_world_readable {
                                new_widget.as_preview_screen().show_room_can_preview(
                                    cx,
                                    &frp,
                                );
                            } else {
                                new_widget.as_preview_screen().show_room_can_not_preview(
                                    cx,
                                    &frp
                                );
                            }
                            cx.action(MainDesktopUiAction::SaveDockIntoAppState);
                        } else {
                            error!("BUG: failed to create tab for room preview of {}", room_id);
                        }

                        self.open_rooms.insert(room_id_as_live_id, selected_room.clone());
                        self.most_recently_selected_room = Some(selected_room);
                    }
                    Err(err) => {
                        error!("Failed to fetch room preview: {}", err);
                    }
                }
                continue;
            }

            match widget_action.cast() {
                RoomLoadingScreenAction::Loading(loading_type) => {
                    match loading_type.clone() {
                        LoadingType::Room{room_or_alias_id, via} => {
                            match OwnedRoomId::try_from(room_or_alias_id) {
                                Ok(room_id) => {
                                    if self.try_jump_to_known_room(cx, room_id.clone()) {
                                        return;
                                    }

                                    let Some(loading_tab_id) = self.show_loading_tab(cx) else {
                                        error!("BUG: failed to create and show loading tab");
                                        return;
                                    };

                                    submit_async_request(MatrixRequest::GetRoomPreview {
                                        room_or_alias_id: room_id.clone().into(),
                                        via: via.clone(),
                                    });

                                    self.loading_tabs.insert(
                                        loading_tab_id,
                                        LoadingTabState {
                                            room_id: Some(room_id.clone()),
                                            room_alias_id: None,
                                            loading_type,
                                            loading_status: LoadingStatus::Requested,
                                            via: via.clone(),
                                        },
                                    );
                                }
                                Err(room_alias_id) => {
                                    if let Some(room_id) = self.resolved_room_aliases.get(&room_alias_id).cloned() {
                                        if self.try_jump_to_known_room(cx, room_id.clone()) {
                                            return;
                                        }

                                        let Some(loading_tab_id) = self.show_loading_tab(cx) else {
                                            error!("BUG: failed to create and show loading tab");
                                            return;
                                        };

                                        submit_async_request(MatrixRequest::GetRoomPreview {
                                            room_or_alias_id: room_id.clone().into(),
                                            via: via.clone(),
                                        });

                                        self.loading_tabs.insert(
                                            loading_tab_id,
                                            LoadingTabState {
                                                room_id: Some(room_id.clone()),
                                                room_alias_id: None,
                                                loading_type,
                                                loading_status: LoadingStatus::Requested,
                                                via: via.clone(),
                                            },
                                        );

                                        return;
                                    }

                                    let Some(loading_tab_id) = self.show_loading_tab(cx) else {
                                        error!("BUG: failed to create and show loading tab");
                                        return;
                                    };

                                    submit_async_request(MatrixRequest::ResolveRoomAlias(room_alias_id.clone()));

                                    self.loading_tabs.insert(
                                        loading_tab_id,
                                        LoadingTabState {
                                            room_id: None,
                                            room_alias_id: Some(room_alias_id),
                                            loading_type,
                                            loading_status: LoadingStatus::Requested,
                                            via: via.clone(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            // Handle actions emitted by the dock within the MainDesktopUI
            match widget_action.cast() { // TODO: don't we need to call `widget_uid_eq(dock.widget_uid())` here?
                // Whenever a tab (except for the home_tab) is pressed, notify the app state.
                DockAction::TabWasPressed(tab_id) => {
                    if tab_id == id!(home_tab) {
                        cx.action(AppStateAction::FocusNone);
                        self.most_recently_selected_room = None;
                    }
                    else if let Some(selected_room) = self.open_rooms.get(&tab_id) {
                        cx.action(AppStateAction::RoomFocused(selected_room.clone()));
                        self.most_recently_selected_room = Some(selected_room.clone());
                    }
                    should_save_dock_action = true;
                }
                DockAction::TabCloseWasPressed(tab_id) => {
                    self.tab_to_close = Some(tab_id);
                    self.close_tab(cx, tab_id);
                    self.redraw(cx);
                    should_save_dock_action = true;
                }
                // When dragging a tab, allow it to be dragged
                DockAction::ShouldTabStartDrag(tab_id) => {
                    self.view.dock(ids!(dock)).tab_start_drag(
                        cx,
                        tab_id,
                        DragItem::FilePath {
                            path: "".to_string(),
                            internal_id: Some(tab_id),
                        },
                    );
                }
                // When dragging a tab, allow it to be dragged
                DockAction::Drag(drag_event) => {
                    if drag_event.items.len() == 1 {
                        self.view.dock(ids!(dock)).accept_drag(cx, drag_event, DragResponse::Move);
                    }
                }
                // When dropping a tab, move it to the new position
                DockAction::Drop(drop_event) => {
                    // from inside the dock, otherwise it's an external file
                    if let DragItem::FilePath {
                        internal_id: Some(internal_id),
                        ..
                    } = &drop_event.items[0] {
                        self.view.dock(ids!(dock)).drop_move(cx, drop_event.abs, *internal_id);
                    }
                    should_save_dock_action = true;
                }
                _ => (),
            }

            // Handle RoomsList actions, which are updates from the rooms list.
            match widget_action.cast() {
                RoomsListAction::Selected(selected_room) => {
                    // Note that this cannot be performed within draw_walk() as the draw flow prevents from
                    // performing actions that would trigger a redraw, and the Dock internally performs (and expects)
                    // a redraw to be happening in order to draw the tab content.
                    self.focus_or_create_tab(cx, selected_room);
                }
                RoomsListAction::InviteAccepted { room_id, room_name } => {
                    self.replace_invite_with_joined_room(cx, scope, room_id, room_name);
                }
                RoomsListAction::None => { }
            }

            // Handle our own actions related to dock updates that we have previously emitted.
            match action.downcast_ref() {
                Some(MainDesktopUiAction::LoadDockFromAppState) => {
                    let app_state = scope.data.get_mut::<AppState>().unwrap();
                    let dock = self.view.dock(ids!(dock));
                    self.room_order = app_state.saved_dock_state.room_order.clone();
                    self.open_rooms = app_state.saved_dock_state.open_rooms.clone();
                    if app_state.saved_dock_state.dock_items.is_empty() {
                        return;
                    }

                    if let Some(mut dock) = dock.borrow_mut() {
                        dock.load_state(cx, app_state.saved_dock_state.dock_items.clone());
                        for (head_live_id, (_, widget)) in dock.items().iter() {
                            match app_state.saved_dock_state.open_rooms.get(head_live_id) {
                                Some(SelectedRoom::JoinedRoom { room_id, room_name }) => {
                                    widget.as_room_screen().set_displayed_room(
                                        cx,
                                        room_id.clone().into(),
                                        room_name.clone(),
                                    );
                                }
                                Some(SelectedRoom::InvitedRoom { room_id, room_name }) => {
                                    widget.as_invite_screen().set_displayed_invite(
                                        cx,
                                        room_id.clone().into(),
                                        room_name.clone(),
                                    );
                                }
                                _ => { }
                            }
                        }
                    } else {
                        error!("BUG: failed to borrow dock widget to restore state upon LoadDockFromAppState action.");
                        continue;
                    }
                    // Note: the borrow of `dock` must end here *before* we call `self.focus_or_create_tab()`.

                    if let Some(selected_room) = &app_state.selected_room {
                        self.focus_or_create_tab(cx, selected_room.clone());
                    }
                    self.view.redraw(cx);
                }
                Some(MainDesktopUiAction::SaveDockIntoAppState) => {
                    let app_state = scope.data.get_mut::<AppState>().unwrap();
                    let dock = self.view.dock(ids!(dock));
                    if let Some(dock_items) = dock.clone_state() {
                        app_state.saved_dock_state.dock_items = dock_items;
                    }
                    app_state.saved_dock_state.open_rooms = self.open_rooms.clone();
                    app_state.saved_dock_state.room_order = self.room_order.clone();
                }
                _ => {}
            }
        }

        if should_save_dock_action {
            cx.action(MainDesktopUiAction::SaveDockIntoAppState);
        }
    }
}

/// Actions sent to the MainDesktopUI widget for saving/restoring its dock state.
#[derive(Debug)]
pub enum MainDesktopUiAction {
    /// Save the state of the dock into the AppState.
    SaveDockIntoAppState,
    /// Load the room panel state from the AppState to the dock.
    LoadDockFromAppState,
    /// Close all tabs; see [`MainDesktopUI::close_all_tabs()`]
    CloseAllTabs {
        on_close_all: Arc<Notify>,
    },
}
