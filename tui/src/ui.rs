use ratatui::layout::{Constraint, Layout};
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::Frame;

use crate::app::App;

impl App {
    pub fn draw_ui(&mut self, frame: &mut Frame) {
        let [message_area, text_area] =
            Layout::vertical([Constraint::Percentage(100), Constraint::Min(3)]).areas(frame.area());

        self.text_area.set_block(
            Block::bordered()
                .title(format!("[ Send message ({}) ]", self.message_list.room))
                .title_bottom(
                    Line::from(format!("[ {} ]", self.message_list.username)).right_aligned(),
                ),
        );
        frame.render_widget(&self.text_area, text_area);

        let [message_area, room_area] =
            Layout::horizontal([Constraint::Percentage(80), Constraint::Percentage(20)])
                .areas(message_area);

        frame.render_widget(&mut self.message_list, message_area);
        frame.render_widget(&mut self.room_list, room_area);

        if let Some(popup) = &mut self.popup {
            frame.render_widget(popup, frame.area());
        }
    }
}
