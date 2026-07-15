use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::event::Message;
use crate::app::state::AppState;
use crate::ui::components::button;
use crate::ui::theme::{
    accent_style, muted_style, panel_block, shortcut_line, text_style, Role,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let title = format!(" {} ", i18n.t("git-panel-title"));
    let block = panel_block(&title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 6 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                i18n.t("git-title"),
                accent_style(theme).add_modifier(Modifier::BOLD),
            ),
        ])),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(i18n.t("git-subtitle")).style(muted_style(theme)),
        Rect {
            x: chunks[0].x,
            y: chunks[0].y + 1,
            width: chunks[0].width,
            height: 1,
        },
    );

    frame.render_widget(
        Paragraph::new(i18n.t("git-available")).style(muted_style(theme)),
        Rect {
            x: chunks[1].x,
            y: chunks[1].y,
            width: chunks[1].width,
            height: 1,
        },
    );
    let btn = Rect {
        x: chunks[1].x,
        y: chunks[1].y + 1,
        width: 22.min(chunks[1].width),
        height: 2.min(chunks[1].height.saturating_sub(1)),
    };
    button(
        frame,
        btn,
        &i18n.t("git-connect-github"),
        Role::Primary,
        Message::GitConnectStart,
        state.git_device.is_none(),
        state,
        theme,
    );

    render_accounts(frame, chunks[2], state);

    let shortcuts = if state.git_device.is_some() {
        vec![("Esc", i18n.t("git-shortcut-cancel-device"))]
    } else {
        vec![
            ("c", i18n.t("git-shortcut-connect")),
            ("x", i18n.t("git-shortcut-delete")),
            ("Esc", i18n.t("shortcut-back")),
        ]
    };
    let refs: Vec<(&str, &str)> = shortcuts.iter().map(|(k, v)| (*k, v.as_str())).collect();
    frame.render_widget(
        Paragraph::new(shortcut_line(theme, &refs)),
        chunks[3],
    );

    if state.git_device.is_some() {
        render_device_overlay(frame, area, state);
    }
}

fn render_accounts(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let mut y = area.y;

    frame.render_widget(
        Paragraph::new(i18n.t("git-connected")).style(muted_style(theme)),
        Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        },
    );
    y += 1;

    if state.git_accounts.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n.t("git-empty")).style(muted_style(theme)),
            Rect {
                x: area.x,
                y,
                width: area.width,
                height: 2,
            },
        );
        return;
    }

    for (i, acc) in state.git_accounts.iter().enumerate() {
        if y >= area.y + area.height {
            break;
        }
        let selected = i == state.selected_git_account;
        let prefix = if selected { "▸ " } else { "  " };
        let style = if selected {
            accent_style(theme)
        } else {
            text_style(theme)
        };
        let row = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 2.min(area.y + area.height - y),
        };
        frame.render_widget(
            Paragraph::new(format!("{prefix}● GitHub  {}  (@{})", acc.label, acc.login)).style(style),
            Rect {
                x: row.x,
                y: row.y,
                width: row.width,
                height: 1,
            },
        );
        if row.height > 1 {
            frame.render_widget(
                Paragraph::new(format!("    {}", acc.connected_at)).style(muted_style(theme)),
                Rect {
                    x: row.x,
                    y: row.y + 1,
                    width: row.width,
                    height: 1,
                },
            );
        }
        state.push_click(row, Message::WizardSelectAccount(i));
        y += 2;
    }
}

fn render_device_overlay(frame: &mut Frame, area: Rect, state: &AppState) {
    let Some(device) = &state.git_device else {
        return;
    };
    let theme = &state.theme;
    let i18n = &state.i18n;

    let modal = centered(60, 50, area);
    frame.render_widget(Clear, modal);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.style(Role::Primary))
        .title(Span::styled(
            format!(" {} ", i18n.t("git-device-title")),
            theme.style_bold(Role::Text),
        ))
        .style(Style::default().bg(theme.color(Role::Surface)))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    let lines = vec![
        Line::from(Span::styled(
            i18n.t("git-device-hint"),
            muted_style(theme),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                i18n.t("git-device-code"),
                muted_style(theme),
            ),
            Span::raw("  "),
            Span::styled(
                device.user_code.clone(),
                accent_style(theme).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            device.verification_uri.clone(),
            theme.style(Role::Primary),
        )),
        Line::from(""),
        Line::from(Span::styled(device.status.clone(), muted_style(theme))),
    ];
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        inner,
    );
}

fn centered(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(popup[1])[1]
}
