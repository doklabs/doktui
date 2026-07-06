mod views;
pub mod anim;
pub mod components;
pub mod editor;
pub mod layout;
pub mod shell;
pub mod sprite;
pub mod theme;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::Paragraph;

use crate::app::state::AppState;
use crate::ui::theme::{Role, error_style, muted_style, success_style};

pub fn render(frame: &mut Frame, state: &AppState) {
    state.click_regions.borrow_mut().clear();
    let root = frame.area();
    let theme = &state.theme;

    frame.render_widget(
        Paragraph::new("").style(Style::default().bg(theme.color(Role::Bg))),
        root,
    );

    if state.screen.uses_app_shell() {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
                Constraint::Length(if state.update_notice.is_some() { 1 } else { 0 }),
                Constraint::Length(if !state.error_message.is_none() && !state.error_panel_open {
                    3
                } else {
                    0
                }),
            ])
            .split(root);

        shell::render_header(frame, outer[0], state);

        let body = outer[1];
        let (sidebar, content) = layout::split_shell(frame, body, state);
        layout::render_sidebar(frame, sidebar, state);
        render_content(frame, content, state);

        shell::render_footer(frame, outer[2], state);

        if let Some(notice) = &state.update_notice {
            let msg = format!(
                "{} {} available — run `doktui update`",
                theme.glyphs.star, notice.latest
            );
            frame.render_widget(
                Paragraph::new(msg)
                    .style(success_style(theme))
                    .style(Style::default().bg(theme.color(Role::Bg))),
                outer[3],
            );
        }

        if state.error_message.is_some() && !state.error_panel_open {
            render_status_bar(frame, outer[4], state);
        }
    } else {
        render_content(frame, root, state);
    }

    layout::render_search_overlay(frame, state);
    layout::render_error_overlay(frame, state);
}

fn render_content(frame: &mut Frame, area: Rect, state: &AppState) {
    use crate::app::state::Screen;
    match state.screen {
        Screen::Welcome => views::onboarding::render_welcome(frame, area, state),
        Screen::AddServer => views::onboarding::render_add_server(frame, area, state),
        Screen::HostKeyPrompt => views::onboarding::render_host_key(frame, area, state),
        Screen::Provisioning => views::provisioning::render(frame, area, state),
        Screen::Home => views::home::render(frame, area, state),
        Screen::DeploymentsHub => views::deployments_hub::render(frame, area, state),
        Screen::Monitoring => views::monitoring::render(frame, area, state),
        Screen::Schedules => views::schedules::render(frame, area, state),
        Screen::ServerList => views::servers::render(frame, area, state),
        Screen::Containers => views::containers::render(frame, area, state),
        Screen::Logs => views::logs::render(frame, area, state),
        Screen::Deploy => views::deploy::render(frame, area, state),
        Screen::Secrets => views::secrets::render(frame, area, state),
        Screen::Editor => editor::render_editor(frame, area, state),
        Screen::ConfirmDestructive => views::confirm::render(frame, area, state),
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let mut text = String::new();
    if state.loading {
        text.push_str(&format!("{} working…  ", anim::spinner(theme, state.anim_tick)));
    }
    if let Some(err) = &state.error_message {
        let suffix = if state.error_detail.is_some() {
            "  (E = full error)"
        } else {
            ""
        };
        frame.render_widget(
            Paragraph::new(format!("{text}{err}{suffix}"))
                .style(error_style(theme))
                .style(Style::default().bg(theme.color(Role::Surface))),
            area,
        );
        return;
    }
    if let Some(msg) = &state.status_message {
        text.push_str(msg);
    }
    if text.is_empty() {
        text = "Ctrl+C quit • Ctrl+F search • Tab focus".into();
    }
    frame.render_widget(
        Paragraph::new(text)
            .style(muted_style(theme))
            .style(Style::default().bg(theme.color(Role::Surface))),
        area,
    );
}
