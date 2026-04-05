spec: task
name: "Scrollable @mention User List"
inherits: project
tags: [feature, mention, ui, scrolling]
estimate: 1d
---

## Intent

Make the @mention autocomplete popup's user list scrollable so that users can browse all matching members instead of being limited to a hardcoded maximum (10 on desktop, 5 on mobile). In rooms with many members, the current truncation silently hides relevant results. A scrollable list with a fixed maximum height improves discoverability while keeping the popup compact.

## Context

The current `List` widget (in `command_text_input.rs`) is a plain `View` with `flow: Down` — it has no scroll capability. The popup uses `height: Fit` which means it grows unbounded. To prevent this, `DESKTOP_MAX_VISIBLE_ITEMS = 10` and `MOBILE_MAX_VISIBLE_ITEMS = 5` artificially cap the displayed items. The search buffer fetches `max_visible_items * 2` results but only the first half are shown.

Keyboard navigation (ArrowUp/Down) is managed via `keyboard_focus_index` in `CommandTextInput`. The highlight uses Animator states (`highlight.on/off`) on dynamically-created widgets.

## Acceptance Criteria

### Scenario: Popup shows scrollable list when results exceed visible area
- **Given** a room with 50+ members
- **When** the user types `@` (no filter text)
- **Then** the popup shows up to ~10 visible items with a scrollbar
- **And** the user can scroll down to see more results

### Scenario: Keyboard navigation scrolls the list
- **Given** the mention popup is open with 20+ results
- **When** the user presses ArrowDown past the visible area
- **Then** the list scrolls to keep the focused item visible
- **When** the user presses ArrowUp past the top of the visible area
- **Then** the list scrolls back up to show the focused item

### Scenario: Mouse wheel scrolling works
- **Given** the mention popup is open with results exceeding the visible area
- **When** the user scrolls with the mouse wheel/trackpad over the list
- **Then** the list scrolls smoothly

### Scenario: Search results are no longer artificially truncated
- **Given** a room where 30 members match the search query
- **When** the popup displays results
- **Then** all 30 results are accessible via scrolling (not capped at 10)

### Scenario: Popup height is bounded
- **Given** a room with 500+ members
- **When** the popup shows unfiltered results
- **Then** the popup height does not exceed a reasonable maximum (~400px desktop, ~250px mobile)
- **And** the popup does not overflow the screen

## Decisions

- Wrap the `List` widget in a `ScrollYView` named `list_scroll` in the CommandTextInput DSL — do NOT rewrite List's draw logic
- Use `height: Fit{max: Abs(360)}` on `list_scroll` in MentionableTextInput DSL for auto-sizing with a cap (native Makepad bounded Fit)
- Add separate `DESKTOP_MAX_DISPLAY_ITEMS = 50` / `MOBILE_MAX_DISPLAY_ITEMS = 25` constants for the scrollable list item limit
- Also increase backend search limit (`max_results`) to use display limits — otherwise CPU search caps at 20/10 results
- Keep existing `MAX_VISIBLE_ITEMS` constants for height reference only
- Keyboard navigation (`on_keyboard_move`) auto-scrolls via `set_scroll_pos()` with manual position calculation — `scroll_bars_obj` is private
- Add `reset_list_scroll()` called from `clear_popup()` and new-search-start — NOT from `clear_items()` (which runs on every streaming refresh and would cause scroll jumping)
- `clip_y: true` on `list_scroll` to prevent content leaking past rounded corners

## Boundaries

### Allowed Changes
- `src/shared/command_text_input.rs` — modify List widget to support scrolling, adjust DSL
- `src/shared/mentionable_text_input.rs` — adjust constants, popup DSL overrides, scroll-on-keyboard logic
- No new cargo dependencies

### Forbidden
- Do NOT change the trigger mechanism or search logic
- Do NOT change the mention insertion or tracking behavior
- Do NOT change the highlight/Animator system
- Do NOT use `PortalList` (virtual scrolling) — overkill for 50 items, and would require rewriting item instantiation
- Do NOT run `cargo fmt`

## Completion Criteria

- [ ] User list scrolls vertically when items exceed visible area
- [ ] Keyboard ArrowUp/Down auto-scrolls to keep focused item visible
- [ ] Mouse wheel/trackpad scrolling works on the list
- [ ] Popup height is bounded (does not grow unbounded)
- [ ] More results are accessible than before (not capped at 10)
- [ ] No visual regression: rounded corners, shadow, header still look correct
- [ ] `cargo build` passes
