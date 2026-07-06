# Layout Revision — Welcome / Onboarding

**Status:** Implemented
**Date:** July 6, 2026
**Project:** Doklabs — DokTUI
**Reference:** [DESIGN.md](./DESIGN.md) (theme & anim), `src/ui/views/onboarding.rs`

---

## 1. Problem with the earlier layout

On the earlier Welcome screen:

1. **Large dead space.** Content clustered top-left (header, intro, SSH key) and bottom-left (steps + keybinds), leaving the center and the entire right side empty. A wide terminal went unused.
2. **Unbalanced.** A "heavy at the top + pinned at the bottom" vertical distribution made the screen feel unfinished.
3. **All left-aligned.** No focal point; the eye had nowhere to go.
4. **Raw SSH key.** A long string shown bare, noisy, unlabeled/unboxed, with no way to copy.
5. **The "Welcome" box wrapped only the title**, not the content — it looked odd.

---

## 2. Revision principles

- **Center the onboarding as a single card** (horizontal + vertical). Balanced margins on all sides remove the dead space and look intentional.
- **One vertical focal flow:** mascot → title → SSH key → steps → actions.
- **SSH key in a labeled sub-box** + a `[c] copy` hint.
- **A horizontal three-step stepper** with accented numbers (using theme color roles).
- **Explicit action row:** a primary `[⏎] Add server` button + a ghost `[q] Quit`.
- Welcome is only the **"no server yet"** state. Once a server is connected, Home switches to a **sidebar + dashboard** layout.

---

## 3. Implementation (ratatui)

### 3.1 Helper: a fixed-size centered rect

The key to this layout. In `ui/layout/mod.rs`.

```rust
use ratatui::layout::Rect;

/// Return a `width`×`height` Rect centered inside `area`.
/// If the area is smaller than requested, use the area as-is (safe on small terminals).
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect { x, y, width: w, height: h }
}
```

### 3.2 Render Welcome as a card

`src/ui/views/onboarding.rs` — using theme roles (not literal colors, per DESIGN.md).

```rust
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::Frame;

use crate::ui::layout::centered_rect;
use crate::ui::theme::{Role, Theme};

pub fn render_welcome(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    // Responsive: full layout with mascot when there's room; compact otherwise.
    let compact = area.height < 24 || area.width < 46;
    if compact { render_welcome_compact(frame, area, state, theme); }
    else       { render_welcome_full(frame, area, state, theme); }
}
```

The full layout centers a 64×24 card and splits it into balanced vertical zones (mascot, title, tagline, divider, SSH-key box, stepper, divider, actions, footer hint). See §4 for responsive behavior.

### 3.3 Horizontal accented stepper

```rust
fn render_stepper(frame: &mut Frame, area: Rect, theme: &Theme) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3); 3])
        .split(area);

    let step = |n: &str, label: &str, role: Role| {
        Line::from(vec![
            Span::styled(format!("[{n}]"), theme.style_bold(role)),
            Span::styled(format!(" {label}"), theme.style(Role::Text)),
        ])
    };

    frame.render_widget(Paragraph::new(step("1", "Register", Role::Primary)).alignment(Alignment::Center), cols[0]);
    frame.render_widget(Paragraph::new(step("2", "Check Docker", Role::Accent)).alignment(Alignment::Center), cols[1]);
    frame.render_widget(Paragraph::new(step("3", "Deploy", Role::Success)).alignment(Alignment::Center), cols[2]);
}
```

### 3.4 SSH key box

A bordered box with a `◆ dedicated ssh key` title, the key middle-truncated to fit, and a `[c] copy` hint. The whole box registers a click region for copy (see [INTERACTION-AND-POLISH.md](./INTERACTION-AND-POLISH.md)).

---

## 4. Responsive / Compact Mode

The card was originally a fixed `centered_rect(64, 24)`, so on a short terminal (e.g., the VS Code bottom panel) the bottom got clipped. The fix branches on `area`:

- **Full mode** (`height ≥ 24` and `width ≥ 46`): the card with mascot and all sections.
- **Compact mode**: the card fills the available height, the mascot is dropped, spacing is tighter, and the two actions become a single combined clickable line. Sections are ordered so essentials (SSH key, steps, actions) survive first if space runs out; cosmetic rows (footer/status) collapse first.

This is auto-fit based on size, complementary to the user-toggled `UiMode` (Compact/Overlay).

---

## 5. Transition to Home (once a server exists)

Welcome is first-run only. When `state.servers` is non-empty, `Screen::Home` renders **sidebar + content**:

```rust
let cols = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Length(22), Constraint::Min(0)]) // fixed sidebar, flexible content
    .split(area);
render_sidebar(frame, cols[0], state, theme);
render_home_dashboard(frame, cols[1], state, theme); // stat cards + deploy panel
```

This structurally removes the "dead space" problem for the main screen: the sidebar fills the left, the dashboard fills the rest of the width.

---

## 6. Notes

All sizes (64×24) are a starting point; tune after seeing them in a real terminal. The principle held: **one centered card, balanced margins, one vertical focal flow** — not content scattered at the edges.
