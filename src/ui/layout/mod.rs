mod sidebar;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Clear, Paragraph, Wrap};

use crate::app::state::{AppState, NavSection, UiMode, clamp_sidebar_width, hit};

use super::theme::{border_style, error_style, muted_style, panel_block, Role};

pub use sidebar::render_sidebar;

pub fn split_shell(_frame: &mut Frame, area: Rect, state: &AppState) -> (Rect, Rect) {
    *state.shell_body.borrow_mut() = area;

    let w = clamp_sidebar_width(state.sidebar_width, area.width);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(w), Constraint::Min(crate::app::state::MIN_CONTENT_WIDTH)])
        .split(area);

    *state.sidebar_area.borrow_mut() = chunks[0];
    *state.gutter_rect.borrow_mut() = Rect {
        x: chunks[0].x.saturating_add(chunks[0].width),
        y: chunks[0].y,
        width: 1,
        height: chunks[0].height,
    };

    (chunks[0], chunks[1])
}

/// Draw the draggable gutter between sidebar and content.
pub fn render_gutter(frame: &mut Frame, state: &AppState) {
    if !state.screen.uses_app_shell() {
        return;
    }
    let gutter = *state.gutter_rect.borrow();
    if gutter.width == 0 {
        return;
    }
    let theme = &state.theme;
    let ch = if state.sidebar_resizing || state.is_hovered(gutter) {
        "▐"
    } else {
        "│"
    };
    let style = if state.sidebar_resizing {
        theme.style(Role::Accent)
    } else {
        border_style(theme)
    };
    for row in 0..gutter.height {
        frame.render_widget(
            Paragraph::new(ch).style(style),
            Rect {
                x: gutter.x,
                y: gutter.y + row,
                width: 1,
                height: 1,
            },
        );
    }
}

pub fn gutter_hit(state: &AppState, col: u16, row: u16) -> bool {
    let g = *state.gutter_rect.borrow();
    g.width > 0 && hit(g, col, row)
}

pub fn render_search_overlay(frame: &mut Frame, state: &AppState) {
    if !state.search_active {
        return;
    }
    let theme = &state.theme;
    let i18n = &state.i18n;

    let area = centered_rect_percent(60, 3, frame.area());
    frame.render_widget(Clear, area);
    let panel_title = format!(" {} ", i18n.t("search-title"));
    let block = panel_block(&panel_title, theme);
    let query = if state.search_query.is_empty() {
        "_".to_string()
    } else {
        state.search_query.clone()
    };
    frame.render_widget(Paragraph::new(query).block(block), area);
}

pub fn render_error_overlay(frame: &mut Frame, state: &AppState) {
    if !state.error_panel_open {
        return;
    }
    let theme = &state.theme;
    let i18n = &state.i18n;
    let Some(detail) = &state.error_detail else {
        return;
    };

    let height = frame.area().height.saturating_sub(4).min(18).max(8);
    let area = centered_rect_percent(80, height, frame.area());
    frame.render_widget(Clear, area);

    let lines: Vec<&str> = detail.lines().collect();
    let scroll = state.error_scroll as usize;
    let visible: String = lines
        .iter()
        .skip(scroll)
        .take(area.height.saturating_sub(2) as usize)
        .copied()
        .collect::<Vec<_>>()
        .join("\n");

    let panel_title = format!(" {} ", i18n.t("error-panel-title"));
    let block = panel_block(&panel_title, theme);
    frame.render_widget(
        Paragraph::new(visible)
            .wrap(Wrap { trim: false })
            .style(error_style(theme))
            .block(block),
        area,
    );
}

pub fn filter_match(haystack: &str, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    haystack
        .to_lowercase()
        .contains(&query.to_lowercase())
}

/// Return a `width`×`height` rect centered inside `area`.
/// Clamps to `area` when the terminal is smaller than requested.
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

fn centered_rect_percent(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + 2;
    Rect {
        x,
        y,
        width: popup_width.max(20),
        height,
    }
}

pub fn render_mode_hint(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let mode = match state.ui_mode {
        UiMode::Overlay => i18n.t("mode-overlay"),
        UiMode::Compact => i18n.t("mode-compact"),
    };
    let focus = if state.sidebar_focused {
        i18n.t("mode-sidebar")
    } else {
        i18n.t("mode-content")
    };
    let hint = i18n.t_fmt("mode-hint", &[("mode", &mode), ("focus", &focus)]);
    frame.render_widget(
        Paragraph::new(hint).style(muted_style(theme)),
        Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(1),
            width: area.width,
            height: 1,
        },
    );
}

pub fn nav_label(state: &AppState, section: NavSection) -> String {
    let i18n = &state.i18n;
    match section {
        NavSection::Home => i18n.t("nav-home"),
        NavSection::Projects => i18n.t("nav-projects"),
        NavSection::Deployments => i18n.t("nav-deployments"),
        NavSection::Monitoring => i18n.t("nav-monitoring"),
        NavSection::Schedules => i18n.t("nav-schedules"),
    }
}

pub fn nav_shortcut(section: NavSection) -> char {
    match section {
        NavSection::Home => '1',
        NavSection::Projects => '2',
        NavSection::Deployments => '3',
        NavSection::Monitoring => '4',
        NavSection::Schedules => '5',
    }
}

pub fn section_from_char(c: char) -> Option<NavSection> {
    match c {
        '1' => Some(NavSection::Home),
        '2' => Some(NavSection::Projects),
        '3' => Some(NavSection::Deployments),
        '4' => Some(NavSection::Monitoring),
        '5' => Some(NavSection::Schedules),
        _ => None,
    }
}
