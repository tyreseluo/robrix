spec: task
name: "Migrate @mention User Feature from Makepad 1.0 to 2.0"
tags: [feature, migration, mention, makepad-2.0]
estimate: 3d
---

## Intent

Migrate the complete @mention user feature from the robrix (Makepad 1.0) project to robrix2 (Makepad 2.0). The 1.0 project at `/Users/zhangalex/Work/Projects/FW/robius/robrix` has a fully working mention system (~4600 lines) covering autocomplete popup, background member search, mention insertion as markdown links, mention metadata in sent messages, and pill-style rendering in the timeline. The robrix2 project currently has only a placeholder `MentionableTextInput` with no actual mention functionality. The existing `CommandTextInput` popup infrastructure in robrix2 provides the autocomplete foundation to build upon.

## Decisions

- Reuse `CommandTextInput` as the popup base widget for mention autocomplete — do not recreate popup from scratch
- Port `member_search.rs` algorithm as-is (pure Rust, no UI framework changes needed)
- Port `cpu_worker.rs` for background thread search execution via `cx.spawn_thread()`
- Mention insertion format: `[{username}](matrixUri)` markdown link with trailing space
- Track mentions via `possible_mentions: BTreeMap<OwnedUserId, String>` and `possible_room_mention: bool`
- Extract real mentions from message text using `matrix_sdk::ruma::events::Mentions` before sending
- Enable existing `MatrixLinkPill` rendering in `html_or_plaintext.rs` by uncommenting Matrix URI parsing
- `@room` mention availability controlled by user's room power levels via `MentionableTextInputAction::PowerLevelsUpdated`
- Trigger character: `@` preceded by whitespace or start of text
- State machine: `Idle` → `WaitingForMembers` → `Searching` → `Idle` (also `JustCancelled` on ESC)
- Search results streamed in batches of 10 via MPSC channel for progressive UI updates
- Cancellation via `Arc<AtomicBool>` token for graceful background search abort
- Makepad 2.0 syntax: `script_mod!` DSL, `#[derive(Script, ScriptHook, Widget)]`, `script_apply_eval!` for runtime property updates

## Boundaries

### Allowed Changes
- src/shared/mentionable_text_input.rs
- src/shared/html_or_plaintext.rs
- src/room/member_search.rs (new)
- src/cpu_worker.rs (new)
- src/room/mod.rs
- src/lib.rs
- src/home/room_screen.rs
- src/home/editing_pane.rs

### Forbidden
- Do not modify `CommandTextInput` internals — extend via composition only
- Do not add new cargo dependencies — use existing `matrix_sdk`, `unicode_segmentation`, `ruma` crates
- Do not change the message sending pipeline in `sliding_sync.rs`
- Do not modify the `RoomInputBar` DSL layout — the `mentionable_text_input` widget slot already exists

## Out of Scope

- Vertical alignment of inline MatrixLinkPill with surrounding text (known Makepad limitation)
- Mention extraction during message editing (editing_pane.rs has a TODO for this)
- Custom pill colors per user
- Mention notification sound/vibration
- Desktop vs mobile adaptive layout for popup items (use single layout initially)

## Completion Criteria

Scenario: Typing @ triggers mention popup
  Test: manual_test_at_trigger_popup
  Given a room with at least 3 members
  When the user types "@" in the message input
  Then a popup appears with header "Users in this Room"
  And the popup lists room members with avatars and display names

Scenario: Search filters members by typed text
  Test: manual_test_mention_search_filter
  Given a room with members "Alice", "Bob", "Alex"
  When the user types "@al"
  Then the popup shows "Alice" and "Alex"
  And the popup does not show "Bob"

Scenario: Selecting a mention inserts markdown link
  Test: manual_test_mention_insert_markdown
  Given the mention popup is showing results
  When the user selects "Alice" from the popup
  Then the input text contains "[Alice](matrix:u/alice:example.com) "
  And the popup closes
  And the cursor is positioned after the trailing space

Scenario: ESC dismisses mention popup
  Test: manual_test_esc_dismisses_popup
  Given the mention popup is open
  When the user presses ESC
  Then the popup closes
  And typing another "@" immediately does not re-open the popup (JustCancelled state)

Scenario: Sent message includes Mentions metadata
  Test: manual_test_mentions_metadata_in_sent_message
  Given the user inserted a mention for "@alice:example.com" in the input
  When the user sends the message
  Then the `RoomMessageEventContent` includes `Mentions` with `user_ids` containing "alice:example.com"

Scenario: @room mention requires power level
  Test: manual_test_at_room_power_level
  Given the user has room notification power level
  When the user types "@room" and selects the @room item
  Then the message includes `Mentions` with `room: true`

Scenario: @room hidden when user lacks power level
  Test: manual_test_at_room_hidden_without_power
  Given the user does not have room notification power level
  When the user types "@"
  Then the popup does not show the @room option

Scenario: Member search runs in background without UI freeze
  Test: manual_test_background_search_responsive
  Given a room with 1000+ members
  When the user types "@a"
  Then the UI remains responsive during search
  And results appear progressively as they are found

Scenario: MatrixLinkPill renders for received mentions
  Test: manual_test_pill_renders_in_timeline
  Given a message containing an HTML mention link `<a href="matrix:u/alice:example.com">Alice</a>`
  When the message is displayed in the timeline
  Then a pill-style widget renders with avatar and display name "Alice"

Scenario: Current user mention renders with red background
  Test: manual_test_current_user_pill_red
  Given a message mentions the current logged-in user
  When the message is displayed in the timeline
  Then the mention pill renders with red background color `#d91b38`

Scenario: Empty search shows no-matches indicator
  Test: manual_test_no_matches_indicator
  Given a room with members "Alice", "Bob"
  When the user types "@zzzzzzz"
  Then the popup shows "No matching users found" indicator

Scenario: Member search handles Unicode names
  Test: manual_test_unicode_member_search
  Given a room with a member named "张三" (Zhang San)
  When the user types "@张"
  Then the popup shows "张三" in the results

Scenario: Mention popup handles room with no loaded members gracefully
  Test: manual_test_no_members_loaded_error
  Given a room where members have not been fetched yet
  When the user types "@"
  Then the popup shows a loading indicator
  And the popup does not crash or show empty results prematurely

Scenario: Stale search results are discarded after cancellation
  Test: manual_test_stale_search_discarded
  Given a background member search is in progress for "@al"
  When the user presses ESC to cancel and then types "@bo"
  Then results from the first search "@al" are discarded
  And only results matching "@bo" are displayed

Scenario: Member search compiles and runs correctly in Makepad 2.0
  Test: manual_test_member_search_ported
  Given the `member_search.rs` module is ported from robrix 1.0
  When `cargo build` is executed
  Then the project compiles without errors
  And `search_room_members_streaming_with_sort()` produces correct results
