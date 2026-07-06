use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

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
            Paragraph::new(label)
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(fg)
                        .bg(bg)
                        .add_modifier(if active {
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
    let width = width.max(1);
    let filled = (pct as usize * width) / 100;
    let role = match pct {
        0..=59 => Role::Success,
        60..=84 => Role::Warning,
        _ => Role::Danger,
    };
    let mut spans = Vec::with_capacity(width);
    for i in 0..width {
        let g = if i < filled {
            &theme.glyphs.bar_full
        } else {
            &theme.glyphs.bar_empty
        };
        let c = if i < filled {
            theme.color(role)
        } else {
            theme.color(Role::Border)
        };
        spans.push(Span::styled(g.clone(), Style::default().fg(c)));
    }
    Line::from(spans)
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
