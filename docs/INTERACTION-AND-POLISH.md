# Interaction & Polish — Mouse, Faster Animation, Proper Pixel Design

**Status:** Implemented / reference
**Date:** July 6, 2026
**Project:** Doklabs — DokTUI
**Reference:** [DESIGN.md](./DESIGN.md), [LAYOUT-REVISION.md](./LAYOUT-REVISION.md)

This document covers three improvements:

1. **Faster, smoother animation** (tick rate + a separate animation clock).
2. **Mouse support** — click, hover, scroll.
3. **Proper pixel design** — a half-block sprite mascot, button components, a spacing system.

> All colors/glyphs still flow through `theme.color(Role::…)` / `theme.glyphs` (DESIGN.md). No literals in views.

---

## 1. Faster, Smoother Animation

### Diagnosis

The tick loop was 250 ms → animation ran at **4 fps** (choppy/slow). For pleasant pixel art, target ~12–15 fps.

### 1.1 Lower the tick + separate the animation clock (`src/app/mod.rs`)

```rust
// BEFORE
let tick_rate = Duration::from_millis(250);

// AFTER — input & housekeeping stay light, animation gets a fast frame
const FRAME_RATE:   Duration = Duration::from_millis(66);   // ~15 fps for animation
const HOUSEKEEPING:  Duration = Duration::from_millis(1000); // heavy refresh: 1s
```

Main loop: redraw every `FRAME_RATE`, but do heavy work (docker polling, etc.) only every `HOUSEKEEPING`.

```rust
let mut last_frame = Instant::now();
let mut last_housekeeping = Instant::now();

loop {
    // Draw every frame (cheap — ratatui is double-buffered, no flicker).
    terminal.draw(|f| ui::render(f, &state, &theme))?;

    // Wait for input at most until the next frame.
    let timeout = FRAME_RATE.saturating_sub(last_frame.elapsed());
    if poll(timeout)? {
        match read()? {
            Event::Key(key)    => { /* … */ }
            Event::Mouse(me)   => handle_mouse(&mut state, me, &bus), // §2
            Event::Resize(w,h) => update(&mut state, Message::Resize(w,h), &config, &bus).await,
            _ => {}
        }
    }

    // Frame tick → advance animation (fast).
    if last_frame.elapsed() >= FRAME_RATE {
        state.anim_tick = state.anim_tick.wrapping_add(1);
        last_frame = Instant::now();
    }

    // Housekeeping tick → heavy work (rare).
    if last_housekeeping.elapsed() >= HOUSEKEEPING {
        update(&mut state, Message::Tick, &config, &bus).await;
        last_housekeeping = Instant::now();
    }

    while let Ok(msg) = rx.try_recv() { update(&mut state, msg.clone(), &config, &bus).await; bus.dispatch(msg); }
    if state.should_quit { break; }
}
```

Add a field to `AppState` (`src/app/state.rs`):

```rust
/// Animation frame counter (advances ~15×/sec). Separate from housekeeping.
pub anim_tick: u64,
```

### 1.2 Tune theme parameters (`themes/*.toml`)

Since a frame is now ~66 ms, set `[motion]` to feel alive:

```toml
[motion]
spinner_period = 2   # swap frames every 2×66ms ≈ 130ms (flowing spinner)
pulse_period   = 12  # one cycle ≈ 0.8s
blink_every    = 45  # mascot blinks ~every 3s
enabled        = true
```

Animation functions (`ui/anim.rs`) read `state.anim_tick`, not the housekeeping tick — so animation is smooth without loading the network.

---

## 2. Mouse Support (click · hover · scroll)

### 2.1 Enable mouse capture (`src/main.rs` / terminal setup)

```rust
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};

// setup
execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
// teardown
execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
```

### 2.2 Clickable region registry (`src/app/state.rs`)

Rendering "registers" interactive areas; the event loop matches the click position. Interior mutability keeps render signatures mostly unchanged.

```rust
use std::cell::RefCell;
use ratatui::layout::Rect;
use crate::app::event::Message;

/// A clickable area + the message sent when clicked.
#[derive(Clone)]
pub struct ClickRegion { pub rect: Rect, pub msg: Message }

// in AppState:
pub click_regions: RefCell<Vec<ClickRegion>>,  // refilled every render
pub mouse_pos: Option<(u16, u16)>,             // for hover

impl AppState {
    /// Called by views to register an interactive area.
    pub fn push_click(&self, rect: Rect, msg: Message) {
        self.click_regions.borrow_mut().push(ClickRegion { rect, msg });
    }
    /// True if the mouse cursor is over the rect (for hover styling).
    pub fn is_hovered(&self, rect: Rect) -> bool {
        matches!(self.mouse_pos, Some((c, r)) if hit(rect, c, r))
    }
}

/// Hit-test: is the point (col,row) inside the rect?
pub fn hit(r: Rect, col: u16, row: u16) -> bool {
    col >= r.x && col < r.x + r.width && row >= r.y && row < r.y + r.height
}
```

Clear the registry at the start of `ui::render`:

```rust
pub fn render(frame: &mut Frame, state: &AppState, theme: &Theme) {
    state.click_regions.borrow_mut().clear();
    // … render as usual; views call state.push_click(rect, msg) …
}
```

### 2.3 Mouse handler (`src/app/mod.rs`)

```rust
use crossterm::event::{MouseEvent, MouseEventKind, MouseButton};

fn handle_mouse(state: &mut AppState, me: MouseEvent, bus: &CommandBus) {
    match me.kind {
        // HOVER — store position for styling
        MouseEventKind::Moved => { state.mouse_pos = Some((me.column, me.row)); }

        // LEFT CLICK — find the hit region, send its message
        MouseEventKind::Down(MouseButton::Left) => {
            let hit_msg = state.click_regions.borrow().iter()
                .find(|c| hit(c.rect, me.column, me.row))
                .map(|c| c.msg.clone());
            if let Some(msg) = hit_msg {
                update_sync(state, msg.clone());   // mutate state directly
                bus.dispatch(msg);                 // async side effects
            }
        }

        // SCROLL — navigate the list in the focused view
        MouseEventKind::ScrollDown => { let _ = bus.tx.send(Message::ScrollDown); }
        MouseEventKind::ScrollUp   => { let _ = bus.tx.send(Message::ScrollUp); }
        _ => {}
    }
}
```

Add variants to `Message` (`src/app/event.rs`): `ScrollUp`, `ScrollDown` (mapped to list navigation in `update`).

### 2.4 Example: a clickable button with hover

A reusable `button()` component fills on hover and registers a click region:

```rust
/// Button component: fills on hover, registers a click.
fn button(frame: &mut Frame, area: Rect, label: &str, role: Role,
          msg: Message, state: &AppState, theme: &Theme) {
    let hovered = state.is_hovered(area);
    let (fg, bg) = if hovered {
        (theme.color(Role::Bg), theme.color(role))          // filled on hover
    } else {
        (theme.color(role), theme.color(Role::Surface))     // outline when idle
    };
    let block = Block::default().borders(Borders::ALL)
        .border_style(Style::default().fg(theme.color(role)))
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(label).alignment(Alignment::Center).style(Style::default().fg(fg).bg(bg)),
        inner,
    );
    state.push_click(area, msg);   // ← this is what makes it clickable
}
```

Sidebar nav, server cards, stepper items — all use the same `push_click` pattern, so the whole UI is consistently clickable.

---

## 3. Proper Pixel Design

Goal: a deliberate "sprite" feel, not forced text characters.

### 3.1 Mascot as a half-block sprite (double density)

Instead of a few characters, draw a sprite from a grid of color indices. Each text row uses `▀`: **foreground = top pixel, background = bottom pixel** → 2× vertical resolution.

```rust
//! ui/sprite.rs
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

/// Sprite = grid of palette indices; 0 = transparent.
pub struct Sprite { pub w: usize, pub h: usize, pub px: &'static [u8] }

/// Render a sprite to Vec<Line> using '▀' (2 pixels per vertical cell).
pub fn render_sprite(s: &Sprite, pal: &[Color]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut y = 0;
    while y < s.h {
        let mut spans = Vec::new();
        for x in 0..s.w {
            let top = s.px[y * s.w + x] as usize;
            let bot = if y + 1 < s.h { s.px[(y + 1) * s.w + x] as usize } else { 0 };
            let fg = pal.get(top).copied().unwrap_or(Color::Reset);
            let bg = pal.get(bot).copied().unwrap_or(Color::Reset);
            spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
        }
        lines.push(Line::from(spans));
        y += 2;
    }
    lines
}
```

The current mascot is "Doko" — a **Terminal Crate**: a shipping crate with a dark screen face showing a `>` prompt and a blinking cursor. Its palette is bound to theme roles (crate = `Primary`, prompt/cursor = `Accent`, screen = `Bg`, typed text/success = `Success`, signal = `Warning`), so it adapts to any theme. Animation frames: idle, blinking cursor, typing, success (green screen), broadcasting signal, and a CRT glitch.

### 3.2 Consistent spacing & components

- **A fixed spacing scale:** 1-cell padding inside a card, a 2-cell gutter between columns. Always via `Layout::margin`/`spacing`, never manual spaces.
- **Reusable components:** `button()` (§2.4), `card(title)` (a titled Block), `stat(label, value, role)`, `health_bar()`. Views compose components, not loose characters.
- **Icons from the theme GlyphSet** (not random emoji): `dot_on/dot_warn/dot_off`, nav arrows, checkmarks. Consistent & themeable.
- **Clean borders:** one border style (`Borders::ALL` + `BorderType::Rounded`); don't mix manual box characters.

### 3.3 Real health bar & sparkline

Replace ad-hoc bars with full/empty cells from the GlyphSet, colored by threshold:

```rust
fn health_bar(pct: u8, width: usize, theme: &Theme) -> Line<'static> {
    let filled = (pct as usize * width) / 100;
    let role = match pct { 0..=59 => Role::Success, 60..=84 => Role::Warning, _ => Role::Danger };
    let mut spans = Vec::new();
    for i in 0..width {
        let g = if i < filled { &theme.glyphs.bar_full } else { &theme.glyphs.bar_empty };
        let c = if i < filled { theme.color(role) } else { theme.color(Role::Border) };
        spans.push(Span::styled(g.clone(), Style::default().fg(c)));
    }
    Line::from(spans)
}
```

Monitoring: braille sparkline `▁▂▃▄▅▆▇█` from `theme.glyphs.sparkline` for smooth CPU/mem graphs.

---

## 4. Notes on Portability

- Mouse capture is supported by crossterm on macOS/Linux/Windows. Some terminals need a specific mode for drag; click & scroll are safe on most modern terminals.
- Provide a config option `mouse = false` for users who rely on terminal text selection (mouse capture intercepts native selection). When off, don't call `EnableMouseCapture`.
