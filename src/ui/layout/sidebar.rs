use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::widgets::{List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::event::Message;
use crate::app::state::{AppState, NavSection, UiMode};
use crate::services::ssh::ConnectionState;
use crate::ui::theme::{bordered_block, muted_style, text_style, Role};

use super::{nav_label, nav_shortcut};

pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let i18n = &state.i18n;
    let nav_title = i18n.t("block-navigation");
    let block = bordered_block(&nav_title, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(4),
            Constraint::Length(2),
        ])
        .split(inner);

    let nav_header = Rect {
        x: chunks[0].x,
        y: chunks[0].y,
        width: chunks[0].width,
        height: 1,
    };
    let nav_list = Rect {
        x: chunks[0].x,
        y: chunks[0].y + 1,
        width: chunks[0].width,
        height: chunks[0].height.saturating_sub(1),
    };

    let nav_items: Vec<ListItem> = NavSection::ALL
        .iter()
        .enumerate()
        .map(|(i, section)| {
            let selected = *section == state.nav_section;
            let dot = if selected {
                theme.glyphs.dot_on.clone()
            } else {
                theme.glyphs.dot_off.clone()
            };
            let label = format!(
                " {dot} [{}] {}",
                nav_shortcut(*section),
                nav_label(state, *section)
            );
            let row_rect = Rect {
                x: nav_list.x,
                y: nav_list.y + i as u16,
                width: nav_list.width,
                height: 1,
            };
            let hovered = state.is_hovered(row_rect);
            let style = if selected {
                theme.style_bold(Role::Primary)
            } else if hovered {
                theme.style(Role::Accent)
            } else if state.sidebar_focused {
                theme.style(Role::Text)
            } else {
                muted_style(theme)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(i18n.t("nav-navigation"))
            .style(theme.style(Role::TextMuted).add_modifier(Modifier::BOLD)),
        nav_header,
    );
    frame.render_widget(List::new(nav_items), nav_list);

    for (i, section) in NavSection::ALL.iter().enumerate() {
        if i as u16 >= nav_list.height {
            break;
        }
        state.push_click(
            Rect {
                x: nav_list.x,
                y: nav_list.y + i as u16,
                width: nav_list.width,
                height: 1,
            },
            Message::GoNav(*section),
        );
    }

    frame.render_widget(
        Paragraph::new(i18n.t("nav-servers"))
            .style(theme.style(Role::TextMuted).add_modifier(Modifier::BOLD)),
        Rect {
            x: chunks[1].x,
            y: chunks[1].y,
            width: chunks[1].width,
            height: 1,
        },
    );

    let server_items: Vec<ListItem> = if state.servers.is_empty() {
        vec![ListItem::new(format!("  {}", i18n.t("nav-none-yet"))).style(muted_style(theme))]
    } else {
        state
            .servers
            .iter()
            .enumerate()
            .map(|(i, srv)| {
                let conn = state.connection_state(srv.id);
                let dot = match conn {
                    ConnectionState::Connected => theme.glyphs.dot_on.as_str(),
                    ConnectionState::Connecting | ConnectionState::Reconnecting => {
                        theme.glyphs.dot_warn.as_str()
                    }
                    ConnectionState::Disconnected => theme.glyphs.dot_off.as_str(),
                };
                let row_rect = Rect {
                    x: chunks[1].x,
                    y: chunks[1].y + 1 + i as u16,
                    width: chunks[1].width,
                    height: 1,
                };
                let style = if Some(srv.id) == state.selected_server {
                    theme.style(Role::Accent)
                } else if state.is_hovered(row_rect) {
                    theme.style(Role::Primary)
                } else {
                    text_style(theme)
                };
                ListItem::new(format!(" {dot} {}", srv.name)).style(style)
            })
            .collect()
    };

    let server_list = Rect {
        x: chunks[1].x,
        y: chunks[1].y + 1,
        width: chunks[1].width,
        height: chunks[1].height.saturating_sub(1),
    };

    frame.render_widget(List::new(server_items), server_list);

    for (i, srv) in state.servers.iter().enumerate() {
        if i as u16 >= server_list.height {
            break;
        }
        state.push_click(
            Rect {
                x: server_list.x,
                y: server_list.y + i as u16,
                width: server_list.width,
                height: 1,
            },
            Message::SelectServer(srv.id),
        );
    }

    let hint = if state.ui_mode == UiMode::Overlay {
        if state.sidebar_focused {
            format!(
                "\"{}\" · {}",
                i18n.t("sidebar-tagline"),
                i18n.t("sidebar-resize-hint")
            )
        } else {
            format!("\"{}\"", i18n.t("sidebar-tagline"))
        }
    } else {
        i18n.t("sidebar-compact")
    };
    frame.render_widget(Paragraph::new(hint).style(muted_style(theme)), chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::{hit, NavSection};
    use crate::i18n::I18n;
    use ratatui::layout::Rect;

    #[test]
    fn nav_hit_rows_align_with_list_area() {
        let nav_list = Rect {
            x: 1,
            y: 5,
            width: 14,
            height: 5,
        };
        for (i, _section) in NavSection::ALL.iter().enumerate() {
            let row = Rect {
                x: nav_list.x,
                y: nav_list.y + i as u16,
                width: nav_list.width,
                height: 1,
            };
            assert!(hit(row, nav_list.x + 2, nav_list.y + i as u16));
        }
    }

    #[test]
    fn nav_labels_resolve() {
        let i18n = I18n::load("en").unwrap();
        let theme = crate::ui::theme::ThemeRegistry::active("pico8");
        let state = AppState::new(
            vec![],
            false,
            String::new(),
            String::new(),
            crate::config::EditorMode::Normal,
            crate::config::UiMode::Overlay,
            vec![],
            theme,
            i18n,
            22,
        );
        assert_eq!(nav_label(&state, NavSection::Home), "Home");
    }
}
