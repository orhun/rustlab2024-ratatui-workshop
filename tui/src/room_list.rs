use common::Username;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, StatefulWidget, Widget},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

#[derive(Debug, Default)]
pub struct RoomList {
    pub state: TreeState<String>,
    pub rooms: Vec<String>,
    pub users: Vec<Username>,
    pub room: String,
}

impl Widget for &mut RoomList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let leaves: Vec<TreeItem<String>> = self
            .rooms
            .iter()
            .flat_map(|room| {
                if room == &self.room {
                    TreeItem::new(
                        room.as_str().to_string(),
                        room.as_str().to_string(),
                        self.users
                            .iter()
                            .map(|user| {
                                TreeItem::new_leaf(user.as_str().to_string(), user.as_str())
                            })
                            .collect(),
                    )
                } else {
                    TreeItem::new(room.as_str().to_string(), room.as_str(), vec![])
                }
            })
            .collect();

        if let Ok(tree) = Tree::new(&leaves) {
            let tree = tree
                .block(Block::bordered().title("[ Rooms ]"))
                .style(Style::default().fg(Color::White));
            self.state.open(vec![self.room.as_str().to_string()]);
            StatefulWidget::render(tree, area, buf, &mut self.state);
        }
    }
}
