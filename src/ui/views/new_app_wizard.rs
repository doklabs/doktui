use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::symbols::border;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::event::Message;
use crate::app::state::{slugify_app_name, AppState, DeployMode, NewAppStep};
use crate::ui::anim;
use crate::ui::components::button;
use crate::ui::theme::{
    accent_style, muted_style, panel_block, shortcut_line, surface_style, text_style, Role,
    BORDER_DASHED, BRAND,
};

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;

    // Soft backdrop so the modal feels layered (Dokploy overlay).
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.color(Role::Bg))),
        area,
    );

    let modal = centered_rect(78, 82, area);
    let i18n = &state.i18n;
    let title = format!(" ✦ {} ", i18n.t("wizard-panel-title"));
    let block = panel_block(&title, theme).padding(Padding::new(2, 2, 1, 1));
    let inner = block.inner(modal);
    frame.render_widget(block, modal);

    if inner.height < 10 || inner.width < 30 {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(inner);

    render_hero(frame, chunks[0], state);
    render_progress(frame, chunks[1], state);

    match state.wizard_step {
        NewAppStep::Type => render_type_step(frame, chunks[2], state),
        NewAppStep::Identity => render_identity_step(frame, chunks[2], state),
        NewAppStep::Account => render_account_step(frame, chunks[2], state),
        NewAppStep::Repo => render_repo_step(frame, chunks[2], state),
    }

    render_footer(frame, chunks[3], state);
}

fn render_hero(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let mascot = theme
        .mascot
        .idle
        .first()
        .cloned()
        .unwrap_or_else(|| "(◕‿◕)".into());

    let (title, blurb) = match state.wizard_step {
        NewAppStep::Type => (
            i18n.t("wizard-type-title"),
            i18n.t("wizard-type-subtitle"),
        ),
        NewAppStep::Identity => (
            match state.deploy_form.mode {
                DeployMode::Compose => i18n.t("wizard-identity-title-compose"),
                DeployMode::GitHub => i18n.t("wizard-identity-title-app"),
            },
            i18n.t("wizard-identity-subtitle"),
        ),
        NewAppStep::Account => (
            i18n.t("wizard-account-title"),
            i18n.t("wizard-account-subtitle"),
        ),
        NewAppStep::Repo => (
            i18n.t("wizard-repo-title"),
            i18n.t("wizard-repo-subtitle"),
        ),
    };

    let pulse = anim::pulse(theme, state.anim_tick);
    let brand_mod = if pulse > 0.55 {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(mascot, theme.style(Role::Primary)),
            Span::raw(" "),
            Span::styled(
                BRAND.to_string(),
                theme.style_bold(Role::Primary).add_modifier(brand_mod),
            ),
            Span::styled("  ·  ", muted_style(theme)),
            Span::styled(title, accent_style(theme).add_modifier(Modifier::BOLD)),
        ])),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );
    frame.render_widget(
        Paragraph::new(blurb).style(muted_style(theme)),
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: 1,
        },
    );
}

fn render_progress(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let is_github =
        state.wizard_type_idx == 1 || state.deploy_form.mode == DeployMode::GitHub;
    let steps: Vec<String> = if is_github {
        vec![
            i18n.t("wizard-step-type"),
            i18n.t("wizard-step-identity"),
            i18n.t("wizard-step-account"),
            i18n.t("wizard-step-repo"),
        ]
    } else {
        vec![
            i18n.t("wizard-step-type"),
            i18n.t("wizard-step-identity"),
        ]
    };
    let cur = state.wizard_step.index().min(steps.len().saturating_sub(1));
    let bar_w = (area.width.saturating_sub(2) as usize).clamp(8, 24);
    let pct = (((cur + 1) * 100) / steps.len()) as u8;
    let bar = anim::progress_bar(theme, state.anim_tick, bar_w, pct);

    let mut spans = vec![
        Span::styled(bar, accent_style(theme)),
        Span::styled("  ", muted_style(theme)),
    ];
    for (i, label) in steps.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" › ", muted_style(theme)));
        }
        let (mark, style) = if i < cur {
            (theme.glyphs.dot_on.as_str(), theme.style(Role::Success))
        } else if i == cur {
            (
                theme.glyphs.dot_on.as_str(),
                accent_style(theme).add_modifier(Modifier::BOLD),
            )
        } else {
            (theme.glyphs.dot_off.as_str(), muted_style(theme))
        };
        spans.push(Span::styled(format!("{mark} {label}"), style));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_type_step(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;

    frame.render_widget(
        Paragraph::new(i18n.t("wizard-type-hint")).style(muted_style(theme)),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );

    let side_by_side = area.width >= 56;
    let cards = Layout::default()
        .direction(if side_by_side {
            Direction::Horizontal
        } else {
            Direction::Vertical
        })
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(Rect {
            x: area.x,
            y: area.y + 2,
            width: area.width,
            height: area.height.saturating_sub(2),
        });

    let options = [
        (
            0usize,
            "◫",
            i18n.t("wizard-type-compose"),
            i18n.t("wizard-type-compose-desc"),
            Role::Accent,
        ),
        (
            1,
            "⌘",
            i18n.t("wizard-type-application"),
            i18n.t("wizard-type-application-desc"),
            Role::Primary,
        ),
    ];

    for ((idx, icon, title, desc, role), card_area) in options.into_iter().zip(cards.iter()) {
        let pad = if side_by_side {
            Rect {
                x: card_area.x + if idx == 1 { 1 } else { 0 },
                y: card_area.y,
                width: card_area.width.saturating_sub(1),
                height: card_area.height,
            }
        } else {
            Rect {
                x: card_area.x,
                y: card_area.y + if idx == 1 { 1 } else { 0 },
                width: card_area.width,
                height: card_area.height.saturating_sub(1),
            }
        };
        render_type_card(frame, pad, state, idx, icon, &title, &desc, role);
    }
}

fn render_type_card(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    idx: usize,
    icon: &str,
    title: &str,
    desc: &str,
    role: Role,
) {
    let theme = &state.theme;
    let selected = state.wizard_type_idx == idx;
    let hovered = state.is_hovered(area);
    let active = selected || hovered;

    let border_role = if active { role } else { Role::Border };
    let border_set = if active {
        border::PLAIN
    } else {
        BORDER_DASHED
    };
    let bg = if selected {
        theme.color(Role::Surface)
    } else {
        theme.color(Role::Bg)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(border_set)
        .border_style(theme.style(border_role))
        .style(Style::default().bg(bg))
        .padding(Padding::new(1, 1, 0, 0));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 || inner.width == 0 {
        state.push_click(area, Message::WizardSelectType(idx));
        return;
    }

    let check = if selected {
        format!("{} ", theme.glyphs.dot_on)
    } else {
        "  ".into()
    };
    let title_style = if selected {
        theme.style_bold(role)
    } else {
        text_style(theme)
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(check, theme.style(Role::Success)),
            Span::styled(format!("{icon}  "), theme.style(role)),
            Span::styled(title.to_string(), title_style),
        ])),
        Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: 1,
        },
    );

    if inner.height > 2 {
        frame.render_widget(
            Paragraph::new(desc)
                .style(muted_style(theme))
                .wrap(Wrap { trim: true }),
            Rect {
                x: inner.x,
                y: inner.y + 2,
                width: inner.width,
                height: inner.height.saturating_sub(3),
            },
        );
    }

    if selected && inner.height > 1 {
        frame.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                format!("▸ {}", state.i18n.t("wizard-shortcut-next")),
                accent_style(theme),
            )]))
            .alignment(Alignment::Right),
            Rect {
                x: inner.x,
                y: inner.y + inner.height.saturating_sub(1),
                width: inner.width,
                height: 1,
            },
        );
    }

    state.push_click(area, Message::WizardSelectType(idx));
}

fn render_identity_step(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;
    let slug = slug_preview(&form.app_name);

    let mode_label = match form.mode {
        DeployMode::Compose => i18n.t("wizard-type-compose"),
        DeployMode::GitHub => i18n.t("wizard-type-application"),
    };
    let server = state
        .selected_server_config()
        .map(|s| format!("{} ({})", s.name, s.host))
        .unwrap_or_else(|| i18n.t("apps-target-none"));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(2),
        ])
        .split(area);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {mode_label} "),
                Style::default()
                    .fg(theme.color(Role::Bg))
                    .bg(theme.color(Role::Accent))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!(" {} {server} ", theme.glyphs.info),
                muted_style(theme),
            ),
        ])),
        chunks[0],
    );

    let name_label = format!(
        "{}  ·  {} {}",
        i18n.t("wizard-field-name"),
        i18n.t("wizard-field-app-name"),
        slug
    );
    render_input_box(
        frame,
        chunks[1],
        state,
        &name_label,
        &form.app_name,
        "",
        form.active_field == 0,
    );
    render_input_box(
        frame,
        chunks[2],
        state,
        &i18n.t("wizard-field-description"),
        &form.description,
        &i18n.t("wizard-field-description-placeholder"),
        form.active_field == 1,
    );
    render_input_box(
        frame,
        chunks[3],
        state,
        &i18n.t("deploy-field-remote-dir"),
        &form.remote_dir,
        "",
        form.active_field == 2,
    );

    if chunks[4].height > 0 {
        let mode = match form.mode {
            DeployMode::Compose => i18n.t("deploy-mode-compose"),
            DeployMode::GitHub => i18n.t("deploy-mode-github"),
        };
        let server_short = state
            .selected_server_config()
            .map(|s| s.name.as_str())
            .unwrap_or("?");
        frame.render_widget(
            Paragraph::new(i18n.t_fmt(
                "wizard-identity-summary",
                &[("mode", &mode), ("server", server_short)],
            ))
            .style(muted_style(theme))
            .wrap(Wrap { trim: true }),
            chunks[4],
        );
    }
}

fn render_account_step(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;

    if state.git_accounts.is_empty() {
        let lines = vec![
            Line::from(Span::styled(
                i18n.t("wizard-account-empty"),
                muted_style(theme),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("▸ {}", i18n.t("wizard-account-connect-cta")),
                accent_style(theme),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), area);
        state.push_click(area, Message::GoGitProviders);
        return;
    }

    frame.render_widget(
        Paragraph::new(i18n.t("wizard-account-hint")).style(muted_style(theme)),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );

    let mut y = area.y + 2;
    for (i, acc) in state.git_accounts.iter().enumerate() {
        if y >= area.y + area.height {
            break;
        }
        let selected = i == state.selected_git_account;
        let prefix = if selected { "▸ " } else { "  " };
        let style = if selected {
            accent_style(theme).add_modifier(Modifier::BOLD)
        } else {
            text_style(theme)
        };
        let row = Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(format!(
                "{prefix}GitHub  {}  (@{})",
                acc.label, acc.login
            ))
            .style(style),
            row,
        );
        state.push_click(row, Message::WizardSelectAccount(i));
        y += 1;
    }
}

fn render_repo_step(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let form = &state.deploy_form;

    let account_label = state
        .git_accounts
        .iter()
        .find(|a| Some(a.id) == form.git_account_id)
        .map(|a| format!("{} (@{})", a.label, a.login))
        .unwrap_or_else(|| i18n.t("wizard-account-none"));

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(i18n.t("wizard-repo-account"), muted_style(theme)),
            Span::raw("  "),
            Span::styled(account_label, accent_style(theme)),
        ])),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );

    if form.github_repos.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n.t("wizard-repo-empty")).style(muted_style(theme)),
            Rect {
                x: area.x,
                y: area.y + 2,
                width: area.width,
                height: 2,
            },
        );
        return;
    }

    frame.render_widget(
        Paragraph::new(i18n.t("wizard-repo-hint")).style(muted_style(theme)),
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: 1,
        },
    );

    let list_top = area.y + 3;
    let visible = area.height.saturating_sub(3) as usize;
    let sel = form.selected_repo;
    let start = sel.saturating_sub(visible.saturating_sub(1) / 2);
    let end = (start + visible).min(form.github_repos.len());

    for (offset, idx) in (start..end).enumerate() {
        let repo = &form.github_repos[idx];
        let selected = idx == sel;
        let prefix = if selected { "▸ " } else { "  " };
        let style = if selected {
            accent_style(theme).add_modifier(Modifier::BOLD)
        } else {
            text_style(theme)
        };
        let row = Rect {
            x: area.x,
            y: list_top + offset as u16,
            width: area.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(format!(
                "{prefix}{}  ·  {}",
                repo.full_name, repo.default_branch
            ))
            .style(style),
            row,
        );
        state.push_click(row, Message::WizardSelectRepo(idx));
    }
}

fn render_input_box(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
    label: &str,
    value: &str,
    placeholder: &str,
    active: bool,
) {
    let theme = &state.theme;
    if area.height < 2 {
        return;
    }

    frame.render_widget(
        Paragraph::new(label).style(if active {
            accent_style(theme).add_modifier(Modifier::BOLD)
        } else {
            muted_style(theme)
        }),
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );

    let box_area = Rect {
        x: area.x,
        y: area.y + 1,
        width: area.width,
        height: 2.min(area.height.saturating_sub(1)),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(if active {
            border::PLAIN
        } else {
            BORDER_DASHED
        })
        .border_style(if active {
            theme.style(Role::Accent)
        } else {
            theme.style(Role::Border)
        })
        .style(surface_style(theme));
    let inner = block.inner(box_area);
    frame.render_widget(block, box_area);

    let empty = value.is_empty();
    let text = if empty { placeholder } else { value };
    let mut spans = vec![Span::styled(
        text.to_string(),
        if empty {
            muted_style(theme)
        } else {
            text_style(theme)
        },
    )];
    if active {
        let cursor = if state.anim_tick % 20 < 12 { "▌" } else { " " };
        spans.push(Span::styled(cursor, accent_style(theme)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);
}

fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(10), Constraint::Length(18)])
        .split(area);

    let is_github =
        state.wizard_type_idx == 1 || state.deploy_form.mode == DeployMode::GitHub;
    let shortcuts = match state.wizard_step {
        NewAppStep::Type => vec![
            ("↑↓", i18n.t("wizard-shortcut-select")),
            ("Enter", i18n.t("wizard-shortcut-next")),
            ("Esc", i18n.t("wizard-shortcut-cancel")),
        ],
        NewAppStep::Identity => vec![
            ("Tab", i18n.t("wizard-shortcut-field")),
            (
                "Enter",
                if is_github {
                    i18n.t("wizard-shortcut-next")
                } else {
                    i18n.t("wizard-shortcut-create")
                },
            ),
            ("Esc", i18n.t("wizard-shortcut-back")),
        ],
        NewAppStep::Account => vec![
            ("↑↓", i18n.t("wizard-shortcut-select")),
            ("c", i18n.t("wizard-shortcut-connect")),
            ("Enter", i18n.t("wizard-shortcut-next")),
            ("Esc", i18n.t("wizard-shortcut-back")),
        ],
        NewAppStep::Repo => vec![
            ("↑↓", i18n.t("wizard-shortcut-select")),
            ("r", i18n.t("wizard-shortcut-refresh")),
            ("Enter", i18n.t("wizard-shortcut-create")),
            ("Esc", i18n.t("wizard-shortcut-back")),
        ],
    };
    let refs: Vec<(&str, &str)> = shortcuts.iter().map(|(k, v)| (*k, v.as_str())).collect();
    frame.render_widget(
        Paragraph::new(shortcut_line(theme, &refs)),
        Rect {
            x: chunks[0].x,
            y: chunks[0].y + chunks[0].height.saturating_sub(1),
            width: chunks[0].width,
            height: 1,
        },
    );

    let (btn_msg, btn_label) = match state.wizard_step {
        NewAppStep::Type => (Message::WizardNext, i18n.t("wizard-shortcut-next")),
        NewAppStep::Identity if is_github => {
            (Message::WizardNext, i18n.t("wizard-shortcut-next"))
        }
        NewAppStep::Identity => (Message::WizardFinish, i18n.t("wizard-create")),
        NewAppStep::Account => (Message::WizardNext, i18n.t("wizard-shortcut-next")),
        NewAppStep::Repo => (Message::WizardFinish, i18n.t("wizard-create")),
    };
    let btn_area = Rect {
        x: chunks[1].x,
        y: chunks[1].y,
        width: chunks[1].width,
        height: chunks[1].height.min(3),
    };
    button(
        frame,
        btn_area,
        &btn_label,
        Role::Primary,
        btn_msg,
        true,
        state,
        theme,
    );
}

fn slug_preview(name: &str) -> String {
    let slug = slugify_app_name(name);
    if slug.is_empty() {
        "my-app".into()
    } else {
        slug
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup[1])[1]
}
