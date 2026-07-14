pub mod model;
pub mod registry;
pub mod resolve;
pub mod validate;

pub use model::{Role, Theme};
pub use registry::ThemeRegistry;

use ratatui::style::Style;
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use crate::i18n::I18n;
use crate::services::ssh::ConnectionState;

pub const BRAND: &str = "DokTUI";

pub fn title_style(theme: &Theme) -> Style {
    theme.style_bold(Role::Primary)
}

pub fn accent_style(theme: &Theme) -> Style {
    theme.style(Role::Accent)
}

pub fn muted_style(theme: &Theme) -> Style {
    theme.style(Role::TextMuted)
}

pub fn error_style(theme: &Theme) -> Style {
    theme.style(Role::Danger)
}

pub fn success_style(theme: &Theme) -> Style {
    theme.style(Role::Success)
}

pub fn warning_style(theme: &Theme) -> Style {
    theme.style(Role::Warning)
}

pub fn surface_style(theme: &Theme) -> Style {
    theme.style(Role::Surface)
}

pub fn text_style(theme: &Theme) -> Style {
    theme.style(Role::Text)
}

pub fn header_line(theme: &Theme, subtitle: &str) -> Line<'static> {
    let mascot = theme.mascot.idle.first().cloned().unwrap_or_else(|| "(◕‿◕)".into());
    Line::from(vec![
        Span::styled(mascot, theme.style(Role::Primary)),
        Span::raw(" "),
        Span::styled(BRAND.to_string(), title_style(theme)),
        Span::styled(format!(" · {subtitle}"), muted_style(theme)),
    ])
}

pub fn shortcut_line(theme: &Theme, items: &[(&str, &str)]) -> Line<'static> {
    let spans: Vec<Span> = items
        .iter()
        .flat_map(|(key, desc)| {
            [
                Span::styled(format!("[{key}]"), accent_style(theme)),
                Span::styled(format!(" {desc}  "), muted_style(theme)),
            ]
        })
        .collect();
    Line::from(spans)
}

pub fn border_style(theme: &Theme) -> Style {
    theme.style(Role::Border)
}

pub fn connection_badge(theme: &Theme, i18n: &I18n, state: ConnectionState) -> (String, Style) {
    match state {
        ConnectionState::Connected => (
            i18n.t_fmt("conn-online", &[("dot", &theme.glyphs.dot_on)]),
            success_style(theme),
        ),
        ConnectionState::Connecting => (
            i18n.t_fmt("conn-connecting", &[("dot", &theme.glyphs.dot_warn)]),
            warning_style(theme),
        ),
        ConnectionState::Reconnecting => (
            i18n.t("conn-reconnecting"),
            warning_style(theme),
        ),
        ConnectionState::Disconnected => (
            i18n.t_fmt("conn-offline", &[("dot", &theme.glyphs.dot_off)]),
            muted_style(theme),
        ),
    }
}

pub fn bordered_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    ratatui::widgets::Block::default()
        .borders(Borders::ALL)
        .border_set(border::PLAIN)
        .border_style(theme.style(Role::Border))
        .title(Span::styled(format!(" {title} "), theme.style(Role::TextMuted)))
        .style(surface_style(theme))
}

/// Dashed border for unfocused controls.
pub const BORDER_DASHED: border::Set = border::Set {
    top_left: "┌",
    top_right: "┐",
    bottom_left: "└",
    bottom_right: "┘",
    vertical_left: "┊",
    vertical_right: "┊",
    horizontal_top: "┄",
    horizontal_bottom: "┄",
};

pub fn welcome_card_block(theme: &Theme, i18n: &I18n) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(border::PLAIN)
        .border_style(theme.style(Role::Primary))
        .title(Span::styled(
            format!(" ▓▒░ {} ", i18n.t("card-getting-started")),
            theme.style(Role::TextMuted),
        ))
        .style(Style::default().bg(theme.color(Role::Surface)))
}

pub fn shortcut_hint_line(theme: &Theme, items: &[(&str, &str)]) -> Line<'static> {
    let spans: Vec<Span> = items
        .iter()
        .flat_map(|(key, desc)| {
            [
                Span::styled(key.to_string(), accent_style(theme)),
                Span::styled(format!(" {desc}  "), muted_style(theme)),
            ]
        })
        .collect();
    Line::from(spans)
}

pub fn panel_block<'a>(title: &'a str, theme: &Theme) -> ratatui::widgets::Block<'a> {
    ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .border_set(border::PLAIN)
        .border_style(theme.style(Role::Primary))
        .title(Span::styled(
            format!(" {title} "),
            theme.style_bold(Role::Text),
        ))
        .style(theme.style_bg(Role::Surface))
}
