use std::io;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, BorderType, Clear, StatefulWidget, Widget},
};
use ratatui_explorer::{FileExplorer, Theme};
use ratatui_image::{protocol::StatefulProtocol, StatefulImage};

pub enum Popup {
    FileExplorer(FileExplorer),
    ImagePreview(Box<dyn StatefulProtocol>),
}

impl Popup {
    pub fn file_explorer() -> io::Result<Self> {
        let theme = Theme::default()
            .add_default_title()
            .with_title_bottom(|fe| format!("[ {} files ]", fe.files().len()).into())
            .with_style(Color::Yellow)
            .with_highlight_item_style(Modifier::BOLD)
            .with_highlight_dir_style(Style::new().blue().bold())
            .with_highlight_symbol("> ")
            .with_block(Block::bordered().border_type(BorderType::Rounded));
        let file_explorer = FileExplorer::with_theme(theme)?;
        Ok(Self::FileExplorer(file_explorer))
    }
}

impl Widget for &mut Popup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self {
            Popup::FileExplorer(explorer) => render_explorer(area, buf, explorer),
            Popup::ImagePreview(ref mut protocol) => render_image_preview(area, buf, protocol),
        }
    }
}

fn render_explorer(area: Rect, buf: &mut Buffer, explorer: &mut FileExplorer) {
    let popup_area = popup_area(area, 50, 50);
    Clear.render(popup_area, buf);
    explorer.widget().render(popup_area, buf);
}

fn render_image_preview(area: Rect, buf: &mut Buffer, protocol: &mut Box<dyn StatefulProtocol>) {
    let popup_area = popup_area(area, 80, 80);
    Clear.render(popup_area, buf);
    let image = StatefulImage::new(None);
    image.render(popup_area, buf, protocol);
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
