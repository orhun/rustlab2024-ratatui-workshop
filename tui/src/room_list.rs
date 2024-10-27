use common::{RoomName, Username};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, StatefulWidget, Widget},
};
use tui_tree_widget::{Tree, TreeItem, TreeState};

#[derive(Debug, Default)]
pub struct RoomList {
    pub state: TreeState<String>,
    pub rooms: Vec<RoomName>,
    pub users: Vec<Username>,
    pub room_name: RoomName,
}

impl RoomList {
    pub fn push_room(&mut self, room: RoomName) {
        self.rooms.push(room);
    }

    pub fn remove_room(&mut self, room: &RoomName) {
        self.rooms.retain(|r| r != room);
    }
}

impl Widget for &mut RoomList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // TODO: render a Tree widget: <https://docs.rs/tui-tree-widget>
    }
}
