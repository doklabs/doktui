# DESIGN.md — Visual, Animation & Theme System

**Status:** Design / Proposal
**Date:** July 6, 2026
**Project:** Doklabs — DokTUI (open source)
**Reference:** [PRD.md](./PRD.md) §7.4 (Experience & Appearance), [TDD.md](./TDD.md) §5 (Event Loop)

---

## 1. Goals & Principles

DokTUI should feel **alive, retro, and fun** without sacrificing simplicity (PRD §7.4: gamification is limited to UI characters/visuals). This document designs three things:

1. A **pixel visual language** — a retro palette, block glyphs, a half-block mascot.
2. An **animation system** built on the existing tick loop.
3. A **modular theme system** — the heart of this document — so anyone can **override** or **add their own theme easily, without recompiling.**

### Binding design principles

| Principle | Consequence |
|-----------|-------------|
| **Themes are data, not code** | Themes are defined in TOML; adding a theme means dropping in one file, not changing source |
| **No hardcoded colors/glyphs in views** | Views reference *semantic roles* (`Role::Success`), not `Color::Rgb(90,197,79)` |
| **Partial override via inheritance** | A theme can `extends` another and override only a few keys |
| **Safe fallback** | A missing key falls back to the base theme, never a crash |
| **DX first** | Adding a theme is clear, validated, and live-reloadable during development |

> Golden rule: **if a view writes a literal color, that's a bug.** All colors & glyphs flow from `Theme`.

---

## 2. Pixel Visual Language

### 2.1 Palette

A pixel look is born from **palette discipline**: few colors, used consistently. Each theme defines a raw palette, then maps it to semantic roles (§3.2). Modern terminals support truecolor, so retro colors render exactly.

### 2.2 Block glyphs

Visual elements are built from block characters, not graphics:

- Fills & bars: `█ ▉ ▊ ▋ ▌ ▍ ▎ ▏ ░ ▒ ▓`
- Half-block (2× vertical resolution): `▀ ▄`
- Sparkline: `▁ ▂ ▃ ▄ ▅ ▆ ▇ █`
- Spinner: braille `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧` or corners `◜ ◝ ◞ ◟`
- Status: `● ◐ ○ ◆ ★`

All of these glyphs are **part of the theme** (§3.3), so other themes can use different glyph sets (e.g., ASCII-only for limited terminals).

### 2.3 Half-block mascot

The mascot is drawn as a pixel grid using `▀` (foreground = top pixel, background = bottom pixel) → one text row = two pixel rows. Mascot frames (idle, blink, happy) **belong to the theme**, so each theme can have its own mascot. See `src/ui/sprite.rs` for the current "Doko" Terminal Crate mascot.

---

## 3. Modular Theme Architecture (Core)

### 3.1 Data model

`Theme` is a pure data struct that can be deserialized from TOML. Three parts: **palette**, **roles** (semantic mapping), **glyphs**, and **motion**.

```rust
//! ui/theme/model.rs
use ratatui::style::Color;
use serde::Deserialize;
use std::collections::HashMap;

/// A complete theme, ready for rendering.
#[derive(Debug, Clone)]
pub struct Theme {
    pub meta: ThemeMeta,
    pub roles: RoleMap,     // semantic role → Color
    pub glyphs: GlyphSet,   // glyph role → String/Vec<String>
    pub motion: Motion,     // animation parameters
    pub mascot: Mascot,     // half-block mascot frames
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeMeta {
    pub name: String,             // unique id, e.g. "pico8"
    pub display_name: String,     // "PICO-8"
    pub author: Option<String>,
    /// Theme this one extends; unset keys are inherited from it.
    pub extends: Option<String>,
}
```

### 3.2 Semantic roles (the key to easy overrides)

Views **never** name a raw color. They name a *role*. This is what makes overriding trivial — change one palette line and the whole UI follows.

```rust
/// Semantic color role. Views use these, not Color directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Bg, Surface, Border,
    Text, TextMuted,
    Primary, Accent,
    Success, Warning, Danger,
    Selection, Cursor,
}

/// Role → Color mapping, the resolved palette.
#[derive(Debug, Clone, Default)]
pub struct RoleMap(pub HashMap<Role, Color>);

impl Theme {
    /// Color for a role. Always resolves (fallback guaranteed at load time).
    pub fn color(&self, role: Role) -> Color {
        self.roles.0.get(&role).copied().unwrap_or(Color::Reset)
    }
}
```

Usage in a view (clean, no literal colors):

```rust
let ok = Style::default().fg(theme.color(Role::Success));
frame.render_widget(Paragraph::new("✓ deployed").style(ok), area);
```

### 3.3 Glyphs & motion as part of the theme

```rust
/// Overridable per-theme glyph set.
#[derive(Debug, Clone, Deserialize)]
pub struct GlyphSet {
    pub bar_full: String,      // "█"
    pub bar_empty: String,     // "░"
    pub spinner: Vec<String>,  // ["◜","◝","◞","◟"]
    pub dot_on: String,        // "●"
    pub dot_warn: String,      // "◐"
    pub dot_off: String,       // "○"
    pub sparkline: Vec<String>,// ["▁",…,"█"]
}

/// Animation parameters (in tick units). A theme can speed up/slow down motion.
#[derive(Debug, Clone, Deserialize)]
pub struct Motion {
    pub spinner_period: u64,   // ticks per spinner frame
    pub pulse_period: u64,     // ticks per pulse cycle
    pub blink_every: u64,      // mascot blinks every N ticks
    pub enabled: bool,         // turn off all animation (accessibility/low-power)
}
```

### 3.4 TOML format

One theme = one `.toml` file. The schema is designed so **partial overrides** are pleasant: write only what changes and inherit the rest via `extends`.

```toml
# themes/pico8.toml
[meta]
name = "pico8"
display_name = "PICO-8"
author = "doklabs"

[palette]              # raw colors, freely named
void   = "#14121f"
purple = "#a06ee1"
cyan   = "#41e0d0"
green  = "#5ac54f"
yellow = "#ffcd75"
pink   = "#ff6e7f"
mist   = "#8f8bb3"

[roles]                # semantic role → palette name (or literal hex)
bg = "void"
surface = "#1a1728"
border = "#2a2740"
text = "#e9e6ff"
text_muted = "mist"
primary = "purple"
accent = "cyan"
success = "green"
warning = "yellow"
danger = "pink"
selection = "purple"
cursor = "cyan"

[glyphs]
bar_full = "█"
bar_empty = "░"
spinner = ["◜","◝","◞","◟"]
dot_on = "●"
dot_warn = "◐"
dot_off = "○"

[motion]
spinner_period = 2
pulse_period = 12
blink_every = 45
enabled = true
```

**Partial override** — a derived theme that only changes the accent:

```toml
# themes/pico8-mono.toml
[meta]
name = "pico8-mono"
display_name = "PICO-8 Mono"
extends = "pico8"      # inherit everything from pico8

[roles]
accent = "#ffffff"     # override a single role
primary = "#ffffff"
```

### 3.5 Resolution & inheritance

Order for building the final `Theme`:

```
1. Load the raw TOML file (RawTheme).
2. If `extends` is set, recursively load the base and MERGE:
   base.roles ⊕ override.roles  (override wins, other keys inherited)
   same for glyphs / motion / mascot.
3. Resolve `roles`: palette name → hex → Color; literal hex → Color.
4. Validate: every Role must have a value (after merge). If any is missing,
   fill from DEFAULT_THEME and log a warning — never crash.
```

```rust
//! ui/theme/resolve.rs
/// Merge override on top of base (override wins per key).
fn merge(base: RawTheme, over: RawTheme) -> RawTheme { /* per-field HashMap extend */ }

/// Build a ready-to-use Theme; guaranteed complete (fallback to default).
pub fn resolve(file: RawTheme, registry: &RawRegistry) -> Result<Theme, ThemeError> {
    let merged = match &file.meta.extends {
        Some(base_id) => merge(registry.load_raw(base_id)?, file),
        None => file,
    };
    let theme = merged.into_theme_with_fallback(&DEFAULT_THEME);
    theme.validate()?; // descriptive error on problems
    Ok(theme)
}
```

### 3.6 Sources & load order

Themes are discovered from several locations, **later wins** (making overrides easy without touching source):

```
1. Built-in (embedded via include_str!)   → always present, the fallback
2. User theme dir (~/.config/doktui/themes/*.toml, %APPDATA%\doktui\themes)
3. Project theme dir (./.doktui/themes/*.toml)         → per-repo/branding
```

```rust
//! ui/theme/registry.rs
pub struct ThemeRegistry { themes: HashMap<String, Theme> }

impl ThemeRegistry {
    /// Load all themes from built-in → user → project (layered overrides).
    pub fn load_all() -> Self { /* scan then resolve each */ }
    pub fn get(&self, name: &str) -> Option<&Theme> { self.themes.get(name) }
    pub fn names(&self) -> impl Iterator<Item = &str> { self.themes.keys().map(String::as_str) }
}
```

Built-in themes stay embedded so the single binary works with no external files (consistent with the PRD: direct install, no dependencies).

### 3.7 Programmatic themes (advanced option)

For dynamic cases (e.g., a theme that follows OS colors), provide a trait — but this is **optional**; the primary path stays TOML.

```rust
/// Non-file theme source (rarely used; TOML is the primary path).
pub trait ThemeProvider {
    fn theme(&self) -> Theme;
}
```

---

## 4. Animation System

All animation is a **pure function of `anim_tick`** (see [INTERACTION-AND-POLISH.md](./INTERACTION-AND-POLISH.md) for the fast frame clock). There are no per-widget timers; one global clock.

```rust
//! ui/anim.rs
use crate::ui::theme::Theme;

/// Current spinner frame based on the tick & the theme's speed.
pub fn spinner(theme: &Theme, tick: u64) -> &str {
    if !theme.motion.enabled { return &theme.glyphs.spinner[0]; }
    let n = theme.glyphs.spinner.len() as u64;
    &theme.glyphs.spinner[((tick / theme.motion.spinner_period) % n) as usize]
}

/// "Marching" progress: the lit segment offset shifts each tick.
pub fn marching(tick: u64, total: usize, lit: usize) -> Vec<bool> {
    (0..total).map(|i| ((i as u64 + tick) % total as u64) < lit as u64).collect()
}

/// Pulse intensity 0.0–1.0 (to pick dot_on/dot_warn glyphs).
pub fn pulse(theme: &Theme, tick: u64) -> f32 {
    let p = theme.motion.pulse_period.max(1);
    let phase = (tick % p) as f32 / p as f32;
    (phase * std::f32::consts::TAU).sin() * 0.5 + 0.5
}
```

Because animation speed lives in `Motion`, **themes can set the rhythm** — and `motion.enabled=false` turns off all animation at once (accessibility, slow terminals, or a low-power preference).

---

## 5. Mascot

The mascot is rendered by a helper that reads the frames from the theme, picks a frame based on the tick, then turns each `▀`-grid row into a colored `Line`/`Span`. Because frames live in the theme, changing the mascot means editing data, not code. The current mascot is "Doko" — a Terminal Crate with a `>` prompt screen face — see `src/ui/sprite.rs`.

---

## 6. Developer & Contributor Experience

This section is the main reason for the architecture above.

### 6.1 Add a new theme — 3 steps, no recompile

```
1. Copy themes/pico8.toml → ~/.config/doktui/themes/my-theme.toml
2. Change [meta].name and the colors under [palette]/[roles]
3. `doktui --theme my-theme`  (or set it in config.toml)
```

No Rust, no rebuild. To bundle a theme, drop it in the repo's `themes/` and add it to the embed list.

### 6.2 Override only part of a theme

Via `extends`, contributors override only what they need (e.g., just `danger` and `accent`). The rest is inherited — no need to copy the whole palette.

### 6.3 The "no hardcoded color" rule is enforced

- A custom lint / test: search for `Color::Rgb`/`Color::` literals in `ui/views/**` → fail CI if present (except in `ui/theme/`).
- Code-review checklist: new views must use `theme.color(Role::…)`.

### 6.4 Validation with helpful messages

```
error: theme "sunset" is invalid
  → role `success` references palette name `grean`, which does not exist
  → did you mean `green`? (available palette: void, green, cyan, …)
```

Validation runs at load; a broken theme never kills the app — it falls back to the default and shows a notice.

### 6.5 Live reload during development

A `--dev` mode watches the theme folder (the `notify` crate) and reloads the `Theme` on file change, without restarting. Design iteration becomes fast.

### 6.6 Theme testing

- **Unit**: every built-in theme loads, resolves, and passes `validate()`.
- **Snapshot**: render one reference view with each theme → compare the buffer (catch color/glyph regressions).
- **Contract**: a test ensures every `Role` variant is mapped in the default theme.

### 6.7 Documentation & discoverability

- `doktui themes list` shows installed themes and their source (built-in/user/project).
- A future `docs/THEMES.md` documents the TOML schema + a table of every `Role`/`Glyph` as a contributor reference.

---

## 7. File Structure

```
src/ui/
├── theme/
│   ├── mod.rs          # public API: Theme, Role, ThemeRegistry
│   ├── model.rs        # Theme/RoleMap/GlyphSet/Motion/Mascot structs
│   ├── resolve.rs      # merge + inheritance + fallback
│   ├── registry.rs     # layered discovery & loading
│   └── validate.rs     # validation + helpful error messages
├── anim.rs             # tick-based animation primitives
├── sprite.rs           # mascot sprites
└── views/…             # ONLY use theme.color()/glyph()/anim()

themes/                 # bundled themes (embedded)
├── pico8.toml
├── gruvbox-material.toml
└── …
```

---

## 8. Bundled Themes

| Theme | Vibe | Core palette |
|-------|------|--------------|
| **gruvbox-material** (default) | Warm, earthy, easy on the eyes | brown bg, pink/aqua accents |
| **pico8** | Colorful retro, cheerful | purple/cyan/green/yellow/pink |

Both share the same structure; the only difference is TOML data — proof the system is truly modular. More variants (Gameboy, Synthwave, light/hard) can be added later.

---

## 9. Phased Implementation

1. **Phase 1 — Theme foundation.** `model.rs` + `resolve.rs` + `registry.rs`, embed a default, refactor the old `theme.rs` → semantic roles. Replace all literal colors in views with `theme.color(Role::…)`.
2. **Phase 2 — Animation.** `anim.rs` (spinner/marching/pulse), wired to `anim_tick`, parameters from `Motion`.
3. **Phase 3 — Mascot & visual gamification.** Half-block mascot from the theme + a cosmetic achievement toast.
4. **Phase 4 — DX polish.** `doktui themes list`, `--dev` live reload, a no-hardcoded-color CI lint, `docs/THEMES.md`.

---

## 10. Open Questions

- Role names (`Role`) — which keys are frozen as the public contract? (Adding a role later is safe; renaming one is breaking.)
- May a theme define layout (sidebar width, etc.) or only colors/glyphs/motion in v1? (Recommendation: v1 is cosmetic only, for simplicity.)
- A theme version scheme (`schema_version`) for forward compatibility?
- Cap the mascot to a fixed size (e.g., max 8×8 pixels) for layout consistency?
