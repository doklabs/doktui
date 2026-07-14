use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;

use crate::app::event::Message;
use crate::app::state::AppState;
use crate::ui::theme::{Role, Theme, BORDER_DASHED};

/// Clickable button — dashed when idle, solid accent when focused or hovered.
pub fn button(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    role: Role,
    msg: Message,
    focused: bool,
    state: &AppState,
    theme: &Theme,
) {
    let hovered = state.is_hovered(area);
    let active = focused || hovered;

    let label_role = if role == Role::Border {
        Role::Text
    } else {
        role
    };
    let accent_role = if role == Role::Border {
        Role::Accent
    } else {
        role
    };

    let (fg, bg, border_color, border_set) = if active {
        (
            theme.color(Role::Bg),
            theme.color(accent_role),
            theme.color(accent_role),
            border::PLAIN,
        )
    } else {
        (
            theme.color(label_role),
            theme.color(Role::Surface),
            theme.color(Role::TextMuted),
            BORDER_DASHED,
        )
    };

    if area.height < 3 {
        frame.render_widget(
            Paragraph::new(label).alignment(Alignment::Center).style(
                Style::default().fg(fg).bg(bg).add_modifier(if active {
                    ratatui::style::Modifier::BOLD
                } else {
                    ratatui::style::Modifier::empty()
                }),
            ),
            area,
        );
        state.push_click(area, msg);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border_set)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(bg));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(fg).bg(bg)),
        inner,
    );
    state.push_click(area, msg);
}

pub fn health_bar(pct: u8, width: usize, theme: &Theme) -> Line<'static> {
    metric_bar(theme, width, pct, false)
}

pub fn sparkline(values: &[u8], width: usize, theme: &Theme) -> Line<'static> {
    let glyphs = &theme.glyphs.sparkline;
    if glyphs.is_empty() || values.is_empty() || width == 0 {
        return Line::from("");
    }
    let max = values.iter().copied().max().unwrap_or(1).max(1);
    let n = glyphs.len();
    let spans: Vec<Span> = values
        .iter()
        .take(width)
        .map(|v| {
            let idx = (*v as usize * (n - 1)) / max as usize;
            Span::styled(
                glyphs[idx].clone(),
                Style::default().fg(theme.color(Role::Accent)),
            )
        })
        .collect();
    Line::from(spans)
}

/// Status kinds for `badge`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    Warning,
    Danger,
    Info,
    Muted,
}

/// Map a Docker-style container status string to a status kind.
pub fn container_status(status: &str) -> Status {
    let s = status.to_lowercase();
    if s.starts_with("up") {
        Status::Success
    } else if s.contains("restarting") {
        Status::Warning
    } else if s.starts_with("exited") {
        Status::Danger
    } else {
        Status::Info
    }
}

/// A short colored label with an icon glyph.
pub fn badge(theme: &Theme, label: &str, status: Status) -> Line<'static> {
    let (glyph, role) = match status {
        Status::Success => (theme.glyphs.dot_on.clone(), Role::Success),
        Status::Warning => (theme.glyphs.warning.clone(), Role::Warning),
        Status::Danger => (theme.glyphs.cross.clone(), Role::Danger),
        Status::Info => (theme.glyphs.info.clone(), Role::Accent),
        Status::Muted => (theme.glyphs.dot_off.clone(), Role::TextMuted),
    };
    Line::from(vec![
        Span::styled(format!("{glyph} "), theme.style(role)),
        Span::styled(label.to_string(), theme.style(role)),
    ])
}

/// A block with internal padding, used as a card/container in views.
pub fn card<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    card_with_role(title, theme, Role::Primary)
}

/// A card with a specific border/title role.
pub fn card_with_role<'a>(title: &'a str, theme: &Theme, role: Role) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_set(border::PLAIN)
        .border_style(theme.style(role))
        .title(Span::styled(
            format!(" {title} "),
            theme.style_bold(Role::Text),
        ))
        .style(theme.style_bg(Role::Surface))
        .padding(Padding::uniform(1))
}

/// A stat card with a title and a value line.
pub fn stat(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    value: Line<'static>,
    role: Role,
    theme: &Theme,
) {
    let block = card(title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(
        Paragraph::new(value)
            .alignment(Alignment::Center)
            .style(theme.style(role)),
        inner,
    );
}

/// A bar whose color thresholds can be inverted so that a high value is good.
pub fn metric_bar(theme: &Theme, width: usize, percent: u8, invert: bool) -> Line<'static> {
    let role = if invert {
        match percent {
            0..=59 => Role::Danger,
            60..=84 => Role::Warning,
            _ => Role::Success,
        }
    } else {
        match percent {
            0..=59 => Role::Success,
            60..=84 => Role::Warning,
            _ => Role::Danger,
        }
    };
    let mut spans = Vec::with_capacity(width);
    let width = width.max(1);
    let filled = (percent as usize * width) / 100;
    for i in 0..width {
        let (g, c) = if i < filled {
            (&theme.glyphs.bar_full, theme.color(role))
        } else {
            (&theme.glyphs.bar_empty, theme.color(Role::Border))
        };
        spans.push(Span::styled(g.clone(), Style::default().fg(c)));
    }
    Line::from(spans)
}
