use ratatui::{
    layout::{Constraint, Layout},
    text::Line,
    widgets::Block,
    Frame,
};

use crate::app::App;

impl App {
    pub fn draw_ui(&mut self, frame: &mut Frame) {
        let [message_area, text_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Max(3)]).areas(frame.area());

        self.text_area.set_block(
            Block::bordered()
                .title(format!(
                    "[ Send message ({}) ]",
                    self.message_list.room_name
                ))
                .title_bottom(
                    Line::from(format!("[ {} ]", self.message_list.username)).right_aligned(),
                ),
        );
        frame.render_widget(&self.text_area, text_area);
        frame.render_widget(&mut self.message_list, message_area);
    }
}
