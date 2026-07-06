use std::collections::HashMap;

use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

/// Semantic color roles — views must use these, never raw `Color`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Bg,
    Surface,
    Border,
    Text,
    TextMuted,
    Primary,
    Accent,
    Success,
    Warning,
    Danger,
    Selection,
    Cursor,
}

impl Role {
    pub const ALL: [Role; 12] = [
        Role::Bg,
        Role::Surface,
        Role::Border,
        Role::Text,
        Role::TextMuted,
        Role::Primary,
        Role::Accent,
        Role::Success,
        Role::Warning,
        Role::Danger,
        Role::Selection,
        Role::Cursor,
    ];

    pub fn from_key(key: &str) -> Option<Role> {
        match key {
            "bg" => Some(Role::Bg),
            "surface" => Some(Role::Surface),
            "border" => Some(Role::Border),
            "text" => Some(Role::Text),
            "text_muted" => Some(Role::TextMuted),
            "primary" => Some(Role::Primary),
            "accent" => Some(Role::Accent),
            "success" => Some(Role::Success),
            "warning" => Some(Role::Warning),
            "danger" => Some(Role::Danger),
            "selection" => Some(Role::Selection),
            "cursor" => Some(Role::Cursor),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeMeta {
    pub name: String,
    pub display_name: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub extends: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawGlyphs {
    #[serde(default = "default_bar_full")]
    pub bar_full: String,
    #[serde(default = "default_bar_empty")]
    pub bar_empty: String,
    #[serde(default = "default_spinner")]
    pub spinner: Vec<String>,
    #[serde(default = "default_dot_on")]
    pub dot_on: String,
    #[serde(default = "default_dot_warn")]
    pub dot_warn: String,
    #[serde(default = "default_dot_off")]
    pub dot_off: String,
    #[serde(default = "default_sparkline")]
    pub sparkline: Vec<String>,
    #[serde(default = "default_check")]
    pub check: String,
    #[serde(default = "default_arrow")]
    pub arrow: String,
    #[serde(default = "default_star")]
    pub star: String,
    #[serde(default = "default_diamond")]
    pub diamond: String,
}

fn default_bar_full() -> String {
    "█".into()
}
fn default_bar_empty() -> String {
    "░".into()
}
fn default_spinner() -> Vec<String> {
    vec!["◜".into(), "◝".into(), "◞".into(), "◟".into()]
}
fn default_dot_on() -> String {
    "●".into()
}
fn default_dot_warn() -> String {
    "◐".into()
}
fn default_dot_off() -> String {
    "○".into()
}
fn default_sparkline() -> Vec<String> {
    vec![
        "▁".into(), "▂".into(), "▃".into(), "▄".into(), "▅".into(), "▆".into(), "▇".into(),
        "█".into(),
    ]
}
fn default_check() -> String {
    "✓".into()
}
fn default_arrow() -> String {
    "▶".into()
}
fn default_star() -> String {
    "★".into()
}
fn default_diamond() -> String {
    "◆".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawMotion {
    #[serde(default = "default_spinner_period")]
    pub spinner_period: u64,
    #[serde(default = "default_pulse_period")]
    pub pulse_period: u64,
    #[serde(default = "default_blink_every")]
    pub blink_every: u64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for RawMotion {
    fn default() -> Self {
        Self {
            spinner_period: default_spinner_period(),
            pulse_period: default_pulse_period(),
            blink_every: default_blink_every(),
            enabled: true,
        }
    }
}

fn default_spinner_period() -> u64 {
    1
}
fn default_pulse_period() -> u64 {
    6
}
fn default_blink_every() -> u64 {
    16
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawMascot {
    #[serde(default)]
    pub idle: Vec<String>,
    #[serde(default)]
    pub blink: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawTheme {
    pub meta: ThemeMeta,
    #[serde(default)]
    pub palette: HashMap<String, String>,
    #[serde(default)]
    pub roles: HashMap<String, String>,
    #[serde(default)]
    pub glyphs: RawGlyphs,
    #[serde(default)]
    pub motion: RawMotion,
    #[serde(default)]
    pub mascot: RawMascot,
}

#[derive(Debug, Clone)]
pub struct GlyphSet {
    pub bar_full: String,
    pub bar_empty: String,
    pub spinner: Vec<String>,
    pub dot_on: String,
    pub dot_warn: String,
    pub dot_off: String,
    pub sparkline: Vec<String>,
    pub check: String,
    pub arrow: String,
    pub star: String,
    pub diamond: String,
}

impl From<RawGlyphs> for GlyphSet {
    fn from(g: RawGlyphs) -> Self {
        Self {
            bar_full: g.bar_full,
            bar_empty: g.bar_empty,
            spinner: g.spinner,
            dot_on: g.dot_on,
            dot_warn: g.dot_warn,
            dot_off: g.dot_off,
            sparkline: g.sparkline,
            check: g.check,
            arrow: g.arrow,
            star: g.star,
            diamond: g.diamond,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Motion {
    pub spinner_period: u64,
    pub pulse_period: u64,
    pub blink_every: u64,
    pub enabled: bool,
}

impl From<RawMotion> for Motion {
    fn from(m: RawMotion) -> Self {
        Self {
            spinner_period: m.spinner_period,
            pulse_period: m.pulse_period,
            blink_every: m.blink_every,
            enabled: m.enabled,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Mascot {
    pub idle: Vec<String>,
    pub blink: Vec<String>,
}

impl From<RawMascot> for Mascot {
    fn from(m: RawMascot) -> Self {
        Self {
            idle: m.idle,
            blink: m.blink,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub meta: ThemeMeta,
    pub roles: HashMap<Role, Color>,
    pub glyphs: GlyphSet,
    pub motion: Motion,
    pub mascot: Mascot,
}

impl Theme {
    pub fn color(&self, role: Role) -> Color {
        self.roles.get(&role).copied().unwrap_or(Color::Reset)
    }

    pub fn style(&self, role: Role) -> Style {
        Style::default().fg(self.color(role))
    }

    pub fn style_bold(&self, role: Role) -> Style {
        self.style(role).add_modifier(Modifier::BOLD)
    }

    pub fn style_bg(&self, role: Role) -> Style {
        Style::default().bg(self.color(role))
    }
}

pub fn parse_hex(hex: &str) -> Option<Color> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}
