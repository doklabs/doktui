use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::event::Message;
use crate::app::state::AppState;
use crate::security::hostkey;
use crate::ui::components::button;
use crate::ui::layout::centered_rect;
use crate::ui::sprite::{mascot_anim, mascot_bob, mascot_palette, mascot_sprite_for, render_sprite};
use crate::ui::theme::{
    accent_style, muted_style, panel_block, shortcut_hint_line, success_style, title_style,
    welcome_card_block, Role, Theme,
};

pub fn render_welcome(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;

    // Responsive density: when the terminal is short or narrow (e.g. the VS Code
    // bottom panel), drop cosmetic rows so the card never gets clipped. Essential
    // elements — SSH key, steps, actions — always stay visible.
    let compact = area.height < 24 || area.width < 46;
    if compact {
        render_welcome_compact(frame, area, state, theme);
    } else {
        render_welcome_full(frame, area, state, theme);
    }
}

/// Full layout with mascot — used when there is comfortable vertical space.
fn render_welcome_full(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let card = centered_rect(64, 24, area);

    let block = welcome_card_block(theme, &state.i18n);
    let inner = block.inner(card);
    frame.render_widget(block, card);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(inner);

    render_mascot(frame, rows[0], state, theme);
    centered_line(frame, rows[1], &state.i18n.t("brand-name"), Role::Text, theme);
    centered_line(
        frame,
        rows[2],
        &state.i18n.t("brand-tagline"),
        Role::TextMuted,
        theme,
    );
    hrule(frame, rows[3], theme);
    render_ssh_key_box(frame, rows[4], state, theme);
    render_stepper(frame, rows[6], state, theme);
    hrule(frame, rows[7], theme);
    render_actions(frame, rows[8], state, theme);

    if let Some(msg) = &state.status_message {
        centered_line(frame, rows[9], msg, Role::TextMuted, theme);
    } else {
        frame.render_widget(
            Paragraph::new(welcome_footer_hint(state)).alignment(Alignment::Center),
            rows[9],
        );
    }
}

/// Compact layout for short/narrow terminals: no mascot, tighter spacing, and a
/// single combined actions line. Ordered so essentials survive even if clipped.
fn render_welcome_compact(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    // Fill the available height instead of a fixed-size card.
    let card = centered_rect(area.width.min(64), area.height, area);

    let block = welcome_card_block(theme, &state.i18n);
    let inner = block.inner(card);
    frame.render_widget(block, card);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Length(1), // title (mascot dropped)
            Constraint::Length(4), // ssh key — the point of onboarding
            Constraint::Length(1), // stepper
            Constraint::Length(1), // actions (single combined line)
            Constraint::Min(0),    // footer/status — collapses first
        ])
        .split(inner);

    centered_line(frame, rows[0], &state.i18n.t("brand-tagline-short"), Role::Text, theme);
    render_ssh_key_box(frame, rows[1], state, theme);
    render_stepper(frame, rows[2], state, theme);
    render_actions_compact(frame, rows[3], state, theme);

    if rows[4].height > 0 {
        if let Some(msg) = &state.status_message {
            centered_line(frame, rows[4], msg, Role::TextMuted, theme);
        } else {
            frame.render_widget(
                Paragraph::new(welcome_footer_hint(state)).alignment(Alignment::Center),
                rows[4],
            );
        }
    }
}

/// Both actions on one line (saves 4 rows vs bordered buttons), still clickable.
fn render_actions_compact(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(14),
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Min(0),
        ])
        .split(area);

    state.push_click(cols[1], Message::GoAddServer);
    state.push_click(cols[3], Message::Quit);

    let i18n = &state.i18n;
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t("welcome-btn-add-server"),
            theme.style_bold(Role::Primary),
        )]))
        .alignment(Alignment::Center),
        cols[1],
    );
    frame.render_widget(
        Paragraph::new("·")
            .alignment(Alignment::Center)
            .style(theme.style(Role::Border)),
        cols[2],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t("welcome-btn-quit"),
            theme.style(Role::TextMuted),
        )]))
        .alignment(Alignment::Center),
        cols[3],
    );
}

fn welcome_footer_hint(state: &AppState) -> Line<'static> {
    let i18n = &state.i18n;
    shortcut_hint_line(
        &state.theme,
        &[
            ("⏎", &i18n.t("welcome-hint-add")),
            ("c", &i18n.t("welcome-hint-copy")),
            ("q", &i18n.t("welcome-hint-quit")),
        ],
    )
}

fn centered_line(frame: &mut Frame, area: Rect, text: &str, role: Role, theme: &Theme) {
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .style(theme.style(role)),
        area,
    );
}

fn hrule(frame: &mut Frame, area: Rect, theme: &Theme) {
    let width = area.width as usize;
    if width == 0 {
        return;
    }
    let line = "─".repeat(width);
    frame.render_widget(
        Paragraph::new(line)
            .alignment(Alignment::Center)
            .style(theme.style(Role::Border)),
        area,
    );
}

fn truncate_middle(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    let char_count = s.chars().count();
    if char_count <= max_len {
        return s.to_string();
    }
    if max_len <= 1 {
        return "…".chars().take(max_len).collect();
    }
    if max_len <= 3 {
        return ".".repeat(max_len);
    }
    let keep = max_len - 1;
    let left = keep / 2;
    let right = keep - left;
    let chars: Vec<char> = s.chars().collect();
    let start: String = chars.iter().take(left).collect();
    let end: String = chars.iter().skip(char_count - right).collect();
    format!("{start}…{end}")
}

fn render_mascot(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let bob = mascot_bob(state.anim_tick);
    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(bob),
            Constraint::Min(1),
        ])
        .split(area);

    let anim = mascot_anim(state.anim_tick, theme);
    let sprite = mascot_sprite_for(anim);
    let pal = mascot_palette(theme);
    let lines = render_sprite(sprite, &pal);
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        slots[1],
    );
}

fn render_stepper(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let i18n = &state.i18n;
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

    frame.render_widget(
        Paragraph::new(step("1", &i18n.t("welcome-step-register"), Role::Primary))
            .alignment(Alignment::Center),
        cols[0],
    );
    frame.render_widget(
        Paragraph::new(step("2", &i18n.t("welcome-step-docker"), Role::Accent)).alignment(Alignment::Center),
        cols[1],
    );
    frame.render_widget(
        Paragraph::new(step("3", &i18n.t("welcome-step-deploy"), Role::Success)).alignment(Alignment::Center),
        cols[2],
    );
}

fn render_ssh_key_box(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let i18n = &state.i18n;
    let b = Block::default()
        .borders(Borders::ALL)
        .border_set(border::PLAIN)
        .border_style(theme.style(Role::Border))
        .title(Span::styled(
            format!(" {} ", i18n.t("welcome-ssh-box-title")),
            theme.style(Role::TextMuted),
        ))
        .style(Style::default().bg(theme.color(Role::Bg)));
    let inner = b.inner(area);
    frame.render_widget(b, area);
    state.push_click(area, Message::CopyPublicKey);

    let content = Rect {
        x: inner.x.saturating_add(1),
        y: inner.y,
        width: inner.width.saturating_sub(2),
        height: inner.height,
    };
    let hint_len = 10usize;
    let key_width = content.width.saturating_sub(hint_len as u16 + 1) as usize;
    let key = truncate_middle(&state.public_key, key_width);
    let fp = truncate_middle(&state.public_key_fingerprint, key_width);
    let lines = vec![
        Line::from(vec![
            Span::styled(key, success_style(theme)),
            Span::raw(" "),
            Span::styled(i18n.t("welcome-ssh-copy"), accent_style(theme)),
        ]),
        Line::from(vec![
            Span::styled(i18n.t("welcome-ssh-fingerprint"), muted_style(theme)),
            Span::raw(" "),
            Span::styled(fp, muted_style(theme)),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        content,
    );
}

fn render_actions(frame: &mut Frame, area: Rect, state: &AppState, theme: &Theme) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(18),
            Constraint::Length(2),
            Constraint::Length(12),
            Constraint::Min(0),
        ])
        .split(area);

    let quit_hovered = state.is_hovered(cols[3]);
    let add_focused = state.is_hovered(cols[1]) || !quit_hovered;

    let i18n = &state.i18n;
    button(
        frame,
        cols[1],
        &i18n.t("welcome-btn-add-server"),
        Role::Primary,
        Message::GoAddServer,
        add_focused,
        state,
        theme,
    );
    button(
        frame,
        cols[3],
        &i18n.t("welcome-btn-quit"),
        Role::Border,
        Message::Quit,
        quit_hovered,
        state,
        theme,
    );
}

pub fn render_add_server(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let panel_title = format!(" {} ", i18n.t("form-add-server-title"));
    let block = panel_block(&panel_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let form = &state.server_form;
    let fields = [
        (i18n.t("form-name"), form.name.as_str(), form.active_field == 0),
        (i18n.t("form-host"), form.host.as_str(), form.active_field == 1),
        (i18n.t("form-port"), form.port.as_str(), form.active_field == 2),
        (i18n.t("form-user"), form.user.as_str(), form.active_field == 3),
        (i18n.t("form-acme-email"), form.acme_email.as_str(), form.active_field == 4),
    ];

    let lines: Vec<ratatui::text::Line> = fields
        .iter()
        .map(|(label, value, active)| {
            let style = if *active {
                accent_style(theme)
            } else {
                muted_style(theme)
            };
            ratatui::text::Line::from(format!("{label}: {value}")).style(style)
        })
        .collect();

    let hint_title = format!(" {} ", i18n.t("form-add-server-hint"));
    frame.render_widget(
        Paragraph::new(lines).block(panel_block(&hint_title, theme)),
        inner,
    );
}

pub fn render_host_key(frame: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let prompt = state.host_key_prompt.as_ref();
    let host_label = prompt
        .map(|p| hostkey::host_label(&p.host, p.port))
        .unwrap_or_default();
    let panel_title = if host_label.is_empty() {
        format!(" {} ", i18n.t("hostkey-title"))
    } else {
        format!(" {} — {} ", i18n.t("hostkey-title"), host_label)
    };
    let block = panel_block(&panel_title, theme).style(title_style(theme));
    let text = if let Some(p) = prompt {
        let label = hostkey::host_label(&p.host, p.port);
        i18n.t_fmt(
            "hostkey-trust",
            &[("host", &label), ("fingerprint", &p.fingerprint)],
        )
    } else {
        i18n.t("hostkey-none")
    };
    frame.render_widget(
        Paragraph::new(text).wrap(Wrap { trim: true }).block(block),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::truncate_middle;

    #[test]
    fn truncate_middle_short_string_unchanged() {
        assert_eq!(truncate_middle("abc", 10), "abc");
    }

    #[test]
    fn truncate_middle_long_string() {
        let s = "ssh-ed25519 AAAAverylongkeypartRMBER doktui@doklabs";
        let out = truncate_middle(s, 20);
        assert!(out.chars().count() <= 20);
        assert!(out.contains('…'));
    }

    #[test]
    fn health_bar_respects_width() {
        use crate::ui::components::health_bar;
        let theme = crate::ui::theme::ThemeRegistry::active("pico8");
        let line = health_bar(50, 8, &theme);
        assert_eq!(line.spans.len(), 8);
    }
}
