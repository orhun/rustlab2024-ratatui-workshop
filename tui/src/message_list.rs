use common::{RoomEvent, RoomName, ServerEvent, Username};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListDirection, ListItem, ListState, StatefulWidget, Widget},
};

#[derive(Debug, Clone, Default)]
pub struct MessageList {
    pub state: ListState,
    pub events: Vec<ServerEvent>,
    pub room_name: RoomName,
    pub username: Username,
}

impl Widget for &mut MessageList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items = self
            .events
            .iter()
            .rev()
            .filter_map(|event| self.server_event_line(&event))
            .map(ListItem::new)
            .collect::<Vec<_>>();

        let list = List::new(items)
            .block(Block::bordered().title("[ Messages ]"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol("> ")
            .repeat_highlight_symbol(true)
            .direction(ListDirection::BottomToTop);

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}

impl MessageList {
    pub fn selected_event(&self) -> Option<ServerEvent> {
        self.state
            .selected()
            .map(|v| self.events[self.events.len() - (v + 1)].clone())
    }

    fn server_event_line<'a>(&self, event: &'a ServerEvent) -> Option<Line<'a>> {
        match event {
            ServerEvent::Help(_, contents) => Some(Line::from(contents.as_str()).blue()),
            ServerEvent::RoomEvent(username, room_event) => {
                self.room_event_line(username.clone(), room_event)
            }
            ServerEvent::Error(error) => Some(Line::from(format!("Error: {error}")).red()),
            _ => None,
        }
    }

    fn room_event_line<'a>(&self, username: Username, event: &'a RoomEvent) -> Option<Line<'a>> {
        match event {
            RoomEvent::Message(message) => {
                let color = if username == self.username {
                    Color::Yellow
                } else {
                    Color::Cyan
                };
                Some(Line::from_iter([
                    Span::from(username).style(color),
                    ": ".white(),
                    message.into(),
                ]))
            }
            RoomEvent::Joined(_) => {
                Some(Line::from(format!("{username} joined the room")).italic())
            }
            RoomEvent::Left(_) => Some(Line::from(format!("{username} left the room")).italic()),
            RoomEvent::NameChange(name) => Some(Line::from(vec![
                Span::from(username).cyan().bold(),
                " is now known as ".into(),
                Span::from(name).green().italic(),
            ])),
            RoomEvent::File(file, _) => Some(Line::from(vec![
                Span::from(username).cyan().bold(),
                " sent a file: ".into(),
                Span::from(file).red().magenta(),
            ])),
        }
    }
}
