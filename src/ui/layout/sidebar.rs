use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::widgets::{List, ListItem, Paragraph};

use crate::app::event::Message;
use crate::app::state::{AppState, NavSection, UiMode};
use crate::services::ssh::ConnectionState;
use crate::ui::theme::{Role, bordered_block, muted_style, text_style};

use super::{nav_label, nav_shortcut};

pub fn render_sidebar(frame: &mut Frame, area: Rect, state: &AppState) {
    let theme = &state.theme;
    let block = bordered_block(" navigation ", theme);
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

    let nav_items: Vec<ListItem> = NavSection::ALL
        .iter()
        .map(|section| {
            let selected = *section == state.nav_section;
            let dot = if selected {
                theme.glyphs.dot_on.clone()
            } else {
                theme.glyphs.dot_off.clone()
            };
            let label = format!(
                " {dot} [{}] {}",
                nav_shortcut(*section),
                nav_label(*section)
            );
            let style = if selected {
                theme.style_bold(Role::Primary)
            } else if state.sidebar_focused {
                theme.style(Role::Text)
            } else {
                muted_style(theme)
            };
            ListItem::new(label).style(style)
        })
        .collect();

    frame.render_widget(
        Paragraph::new("NAVIGATION")
            .style(theme.style(Role::TextMuted).add_modifier(Modifier::BOLD)),
        Rect {
            x: chunks[0].x,
            y: chunks[0].y,
            width: chunks[0].width,
            height: 1,
        },
    );
    frame.render_widget(List::new(nav_items), chunks[0]);

    let nav_list_y = chunks[0].y + 1;
    for (i, section) in NavSection::ALL.iter().enumerate() {
        state.push_click(
            Rect {
                x: chunks[0].x,
                y: nav_list_y + i as u16,
                width: chunks[0].width,
                height: 1,
            },
            Message::GoNav(*section),
        );
    }

    frame.render_widget(
        Paragraph::new("SERVERS")
            .style(theme.style(Role::TextMuted).add_modifier(Modifier::BOLD)),
        Rect {
            x: chunks[1].x,
            y: chunks[1].y,
            width: chunks[1].width,
            height: 1,
        },
    );

    let server_items: Vec<ListItem> = if state.servers.is_empty() {
        vec![ListItem::new("  (none yet)").style(muted_style(theme))]
    } else {
        state
            .servers
            .iter()
            .map(|srv| {
                let conn = state.connection_state(srv.id);
                let dot = match conn {
                    ConnectionState::Connected => theme.glyphs.dot_on.as_str(),
                    ConnectionState::Connecting | ConnectionState::Reconnecting => {
                        theme.glyphs.dot_warn.as_str()
                    }
                    ConnectionState::Disconnected => theme.glyphs.dot_off.as_str(),
                };
                let style = if Some(srv.id) == state.selected_server {
                    theme.style(Role::Accent)
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
        "\"deploy aman, bro\""
    } else {
        "compact"
    };
    frame.render_widget(
        Paragraph::new(hint).style(muted_style(theme)),
        chunks[2],
    );
}
