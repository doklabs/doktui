use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::state::AppState;
use crate::ui::anim;
use crate::ui::sprite::{MascotContext, mascot_header_glyph};
use crate::ui::theme::{Role, connection_badge, shortcut_line};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn render_header(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(22)])
        .split(area);

    let mascot = mascot_header_glyph(
        theme,
        state.anim_tick,
        MascotContext {
            loading: state.loading,
            error: state.error_message.is_some(),
            success: state.achievement.is_some() || state.status_message.is_some(),
        },
    );

    let conn = state
        .selected_server
        .map(|id| state.connection_state(id))
        .unwrap_or(crate::services::ssh::ConnectionState::Disconnected);
    let (conn_label, conn_style) = connection_badge(theme, i18n, conn);

    let left = Line::from(vec![
        Span::styled(mascot, theme.style_bold(Role::Accent)),
        Span::raw("  "),
        Span::styled(
            format!(
                "{} · {}",
                i18n.t("brand-name"),
                i18n.t("brand-subtitle")
            ),
            theme.style_bold(Role::Text),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(left).style(Style::default().bg(theme.color(Role::Bg))),
        chunks[0],
    );

    let right = Line::from(vec![
        Span::styled(format!("v{VERSION}  "), theme.style(Role::TextMuted)),
        Span::styled(theme.glyphs.diamond.clone(), theme.style(Role::Accent)),
        Span::raw(" "),
        Span::styled(conn_label, conn_style),
    ]);
    frame.render_widget(
        Paragraph::new(right)
            .alignment(ratatui::layout::Alignment::Right)
            .style(Style::default().bg(theme.color(Role::Bg))),
        chunks[1],
    );
}

pub fn render_footer(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(14),
            Constraint::Length(16),
        ])
        .split(area);

    let keys = if state.screen.uses_app_shell() {
        shortcut_line(
            theme,
            &[
                ("↑↓", &i18n.t("shortcut-nav")),
                ("↵", &i18n.t("shortcut-open")),
                ("d", &i18n.t("shortcut-deploy")),
                ("e", &i18n.t("shortcut-editor")),
                ("/", &i18n.t("shortcut-search")),
                ("q", &i18n.t("shortcut-quit")),
            ],
        )
    } else {
        shortcut_line(
            theme,
            &[
                ("↵", &i18n.t("shortcut-continue")),
                ("Esc", &i18n.t("shortcut-back")),
                ("q", &i18n.t("shortcut-quit")),
            ],
        )
    };
    frame.render_widget(
        Paragraph::new(keys).style(Style::default().bg(theme.color(Role::Bg))),
        chunks[0],
    );

    let spin = anim::spinner(theme, state.anim_tick);
    let fps = i18n.t_fmt("shortcut-fps", &[("spin", &spin)]);
    frame.render_widget(
        Paragraph::new(fps).style(theme.style(Role::TextMuted)),
        chunks[1],
    );

    let conn = state
        .selected_server
        .map(|id| state.connection_state(id))
        .unwrap_or(crate::services::ssh::ConnectionState::Disconnected);
    let (label, style) = connection_badge(theme, i18n, conn);
    frame.render_widget(
        Paragraph::new(label)
            .style(style.add_modifier(Modifier::BOLD))
            .alignment(ratatui::layout::Alignment::Right),
        chunks[2],
    );
}
