use crate::action::{Action, ConfirmEvent};
use crate::app::{Mode, ModeHolderLock};
use crate::components::recent_chat::SELECTED_STYLE;
use crate::components::user_input::{InputData, UserInput};
use crate::components::{area_util, Component};
use crate::proxy::friend::Friend;
use crate::proxy::group::{DetailRes, GroupUser};
use crate::proxy::{friend, group};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Color, Line, Span, Style, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::{symbols, Frame};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use tracing::error;

pub(crate) struct GroupManager {
    mode_holder: ModeHolderLock,
    user_input: UserInput,
    gid: Option<i32>,
    detail: Arc<Mutex<DetailRes>>,
    friends: Arc<Mutex<Vec<Friend>>>,
    state: State,
    group_members_list_state: ListState,
    friends_list_state: ListState,
}

#[derive(Eq, PartialEq)]
enum State {
    GroupDetail,
    InviteFriend,
}

#[derive(
    Eq, PartialEq, Clone, Copy, Debug, Display, FromRepr, EnumIter, Serialize, Deserialize,
)]
pub(crate) enum ManageAction {
    #[strum(to_string = "移出群聊")]
    Evict,
    #[strum(to_string = "禁言🤐")]
    Forbid,
    #[strum(to_string = "设为管理员")]
    SetManager,
}

impl From<ManageAction> for Text<'_> {
    fn from(action: ManageAction) -> Self {
        let span = Span::styled(format!(">: {action}"), Style::default().fg(Color::White));
        Line::from(span).into()
    }
}

#[cfg(test)]
mod test {
    use crate::components::group_manager::ManageAction;
    use strum::IntoEnumIterator;

    #[test]
    fn test_enum() {
        for e in ManageAction::iter() {
            println!("{}", e.to_string());
        }
    }
}

impl Component for GroupManager {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::GroupManager {
            match self.state {
                State::GroupDetail => match key.code {
                    KeyCode::Esc => {
                        self.mode_holder.set_mode(Mode::Chat);
                        self.group_members_list_state.select(None);
                    }
                    KeyCode::Char('e') => {
                        self.next_state();
                        self.fetch_friends();
                        self.group_members_list_state.select(None);
                    }
                    KeyCode::Up => self.group_members_list_state.select_previous(),
                    KeyCode::Down => self.group_members_list_state.select_next(),
                    KeyCode::Enter => {
                        //TODO 移出群，禁言，设为管理员
                        if let Some(idx) = self.group_members_list_state.selected() {
                            let name = self
                                .detail
                                .lock()
                                .unwrap()
                                .users
                                .get(idx)
                                .unwrap()
                                .name
                                .clone();
                            return Ok(Some(Action::Alert(
                                format!("你希望将{name}:"),
                                Some(ConfirmEvent::GroupManage(None)),
                            )));
                        }
                    }
                    _ => {}
                },
                State::InviteFriend => match key.code {
                    KeyCode::Esc => {
                        self.friends_list_state.select(None);
                        self.next_state();
                        self.user_input.reset();
                    }
                    KeyCode::Char(to_insert) => self.user_input.enter_char(to_insert),
                    KeyCode::Backspace => self.user_input.delete_char(),
                    KeyCode::Left => self.user_input.move_cursor_left(),
                    KeyCode::Right => self.user_input.move_cursor_right(),
                    KeyCode::Up => self.friends_list_state.select_previous(),
                    KeyCode::Down => self.friends_list_state.select_next(),
                    KeyCode::Enter => {
                        if let Some(idx) = self.friends_list_state.selected() {
                            let name = self.friends.lock().unwrap().get(idx).unwrap().name.clone();
                            return Ok(Some(Action::Alert(
                                format!("确定邀请{name}入群么？"),
                                Some(ConfirmEvent::InviteFriend),
                            )));
                        }
                    }
                    _ => {}
                },
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Confirm(ConfirmEvent::InviteFriend) => {
                self.mode_holder.set_mode(Mode::GroupManager);
                self.invite_group_member();
                self.next_state();
                self.group_detail(self.gid.unwrap());
            }
            Action::Group(gid) => {
                self.gid = Some(gid);
                self.mode_holder.set_mode(Mode::GroupManager);
                self.group_detail(gid);
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        match self.mode_holder.get_mode() {
            Mode::GroupManager | Mode::Alert => {
                let area = area_util::group_manager_area(area);
                let [search_area, group_member_area] =
                    Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
                let [group_member_area, friend_area] = match self.state {
                    State::GroupDetail => {
                        Layout::horizontal([Constraint::Percentage(100), Constraint::Percentage(0)])
                            .areas(group_member_area)
                    }
                    State::InviteFriend => {
                        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                            .areas(group_member_area)
                    }
                };
                let list_block = Block::new()
                    .title("Group Members(↑↓ Or Enter)")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED);

                let search_block = Block::new()
                    .title(self.user_input.input_data.label())
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED);
                let user_input =
                    Paragraph::new(self.user_input.input.clone().unwrap_or("".to_string()))
                        .style(self.user_input.select_style())
                        .block(search_block);
                frame.render_widget(user_input, search_area);
                self.render_group_members(frame, group_member_area, list_block);
                match self.state {
                    State::GroupDetail => {}
                    State::InviteFriend => {
                        let list_block = Block::new()
                            .title("Friends(↑↓ Or Enter)")
                            .title_alignment(Alignment::Center)
                            .borders(Borders::ALL)
                            .border_set(symbols::border::ROUNDED);
                        self.user_input.set_cursor_position(search_area);
                        self.render_friends(frame, friend_area, list_block);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl GroupManager {
    pub(crate) fn new(mode_holder: ModeHolderLock) -> Self {
        Self {
            mode_holder,
            user_input: UserInput::new(InputData::Search {
                label: Some("Press e To Invite Friend To Join Group".to_string()),
                data: None,
            }),
            gid: None,
            detail: Arc::new(Mutex::new(DetailRes {
                group_id: 0,
                name: "".to_string(),
                users: vec![],
            })),
            friends: Arc::new(Mutex::new(vec![])),
            state: State::GroupDetail,
            group_members_list_state: Default::default(),
            friends_list_state: Default::default(),
        }
    }

    fn render_group_members(&mut self, frame: &mut Frame, friend_area: Rect, block: Block) {
        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self
            .detail
            .lock()
            .unwrap()
            .users
            .iter()
            .map(|gu| ListItem::new(Text::from(gu)))
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(list, friend_area, &mut self.group_members_list_state);
    }
    fn render_friends(&mut self, frame: &mut Frame, friend_area: Rect, block: Block) {
        let items: Vec<ListItem> = self
            .friends
            .lock()
            .unwrap()
            .iter()
            .filter_map(|f| match self.user_input.input.clone() {
                None => Some(ListItem::new(Text::from(f))),
                Some(x) => {
                    if f.name.contains(&x) {
                        Some(ListItem::new(Text::from(f)))
                    } else {
                        None
                    }
                }
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(list, friend_area, &mut self.friends_list_state);
    }

    fn next_state(&mut self) {
        match self.state {
            State::GroupDetail => {
                self.state = State::InviteFriend;
                self.user_input.is_editing = true;
            }
            State::InviteFriend => {
                self.state = State::GroupDetail;
                self.user_input.is_editing = false;
            }
        }
    }

    fn fetch_friends(&mut self) {
        match friend::friends() {
            Ok(friends) => {
                self.friends = Arc::new(Mutex::new(friends));
            }
            Err(err) => {
                error!("Failed to get friends: {}", err);
            }
        };
    }

    fn invite_group_member(&mut self) {
        if let Some(idx) = self.friends_list_state.selected() {
            if let Some(friend) = self.friends.lock().unwrap().get(idx) {
                let uid = friend.id;
                let gid = self.gid.unwrap();
                if let Err(e) = group::invite(uid, gid) {
                    error!("Failed to invite group :{e}");
                };
            }
        }
    }

    fn group_detail(&mut self, gid: i32) {
        match group::detail(gid) {
            Ok(detail) => {
                self.detail = Arc::new(Mutex::new(detail));
            }
            Err(err) => error!("fail to fetch group detail: {}", err),
        }
    }
}

impl From<&GroupUser> for Text<'_> {
    fn from(gu: &GroupUser) -> Self {
        let mut spans = vec![Span::styled(
            format!("好友: {}", gu.name),
            Style::default().fg(Color::White),
        )];
        if gu.admin {
            spans.push(Span::styled(", ", Style::default().fg(Color::White)));
            spans.push(Span::styled("管理员", Style::default().fg(Color::Blue)));
        };
        if gu.forbid {
            spans.push(Span::styled(", ", Style::default().fg(Color::White)));
            spans.push(Span::styled("已禁言🤐", Style::default().fg(Color::Red)));
        };
        Line::from(spans).into()
    }
}
