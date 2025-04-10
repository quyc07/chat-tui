use crate::action::{Action, ConfirmEvent};
use crate::app::{Mode, ModeHolderLock};
use crate::components::recent_chat::SELECTED_STYLE;
use crate::components::user_input::{InputData, UserInput};
use crate::components::{area_util, Component};
use crate::proxy::friend::{Friend, FriendReq, FriendRequestStatus};
use crate::proxy::{friend, user};
use crate::token::CURRENT_USER;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Text;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::{symbols, Frame};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::task::id;
use tracing::error;

pub(crate) struct Contact {
    mode_holder: ModeHolderLock,
    friends_holder: FriendsHolder,
    search_result: Arc<Mutex<Vec<FriendSearchRes>>>,
    friend_req_holder: FriendReqHolder,
    friend_list_state: ListState,
    friend_req_list_state: ListState,
    search_list_state: ListState,
    user_input: UserInput,
    state: State,
}

struct FriendsHolder {
    need_fetch: bool,
    friends: Arc<Mutex<Vec<Friend>>>,
}

struct FriendReqHolder {
    need_fetch: bool,
    friend_reqs: Arc<Mutex<Vec<FriendReq>>>,
}

impl FriendReqHolder {
    fn has_new_friend_reqs(&self) -> bool {
        self.friend_reqs
            .lock()
            .unwrap()
            .iter()
            .any(|req| req.status == FriendRequestStatus::WAIT)
    }
}

#[derive(Default, Eq, PartialEq)]
enum State {
    #[default]
    Friends,
    Search,
    AddFriend,
    FriendReq,
}

impl Contact {
    pub(crate) fn new(mode_holder: ModeHolderLock) -> Self {
        Self {
            mode_holder,
            friends_holder: FriendsHolder {
                need_fetch: true,
                friends: Arc::new(Mutex::new(Vec::new())),
            },
            search_result: Arc::new(Mutex::new(Vec::new())),
            friend_req_holder: FriendReqHolder {
                need_fetch: true,
                friend_reqs: Arc::new(Mutex::new(vec![])),
            },
            friend_list_state: Default::default(),
            friend_req_list_state: Default::default(),
            search_list_state: Default::default(),
            user_input: UserInput::new(InputData::Search {
                label: Some("Press e To Search New Friend Here.".to_string()),
                data: None,
            }),
            state: Default::default(),
        }
    }

    fn change_state(&mut self, state: State) {
        match state {
            State::Friends => {
                self.state = State::Friends;
                self.user_input.is_editing = false;
            }
            State::Search => {
                self.state = State::Search;
                self.user_input.is_editing = true;
            }
            State::AddFriend => {
                self.state = State::AddFriend;
                self.user_input.is_editing = false;
            }
            State::FriendReq => {
                self.state = State::FriendReq;
                self.user_input.is_editing = false;
            }
        }
    }

    fn clean_search(&mut self) {
        self.search_result.lock().unwrap().clear();
        self.user_input.reset();
    }

    fn search(&mut self, name: String) {
        match user::search(name) {
            Ok(users) => {
                self.search_result = Arc::new(Mutex::new(
                    users
                        .into_iter()
                        .map(|u| FriendSearchRes {
                            id: u.id,
                            name: u.name,
                            is_friend: u.is_friend,
                        })
                        .collect(),
                ));
            }
            Err(err) => {
                error!("Failed to search user: {}", err);
            }
        }
    }

    fn render_friends(&mut self, frame: &mut Frame, friend_area: Rect, block: Block) {
        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self
            .friends_holder
            .friends
            .lock()
            .unwrap()
            .iter()
            .map(|friend| ListItem::new(Text::from(friend)))
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(list, friend_area, &mut self.friend_list_state);
    }
    fn render_friend_reqs(&mut self, frame: &mut Frame, friend_area: Rect, block: Block) {
        // Iterate through all elements in the `items` and stylize them.
        if self.friend_req_holder.has_new_friend_reqs() {
            let items: Vec<ListItem> = self
                .friend_req_holder
                .friend_reqs
                .lock()
                .unwrap()
                .iter()
                .map(|friend_req| ListItem::new(Text::from(friend_req)))
                .collect();

            // Create a List from all list items and highlight the currently selected one
            let list = List::new(items)
                .block(block)
                .highlight_style(SELECTED_STYLE)
                .highlight_spacing(HighlightSpacing::Always);
            frame.render_stateful_widget(list, friend_area, &mut self.friend_req_list_state);
        }
    }
    fn render_friend_search_res(&mut self, frame: &mut Frame, friend_area: Rect, block: Block) {
        let items: Vec<ListItem> = self
            .search_result
            .lock()
            .unwrap()
            .iter()
            .map(|friend| ListItem::new(Text::from(friend)))
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(list, friend_area, &mut self.search_list_state);
    }
}

#[derive(Clone)]
struct FriendSearchRes {
    id: i32,
    name: String,
    is_friend: bool,
}

impl From<&FriendSearchRes> for Text<'_> {
    fn from(f: &FriendSearchRes) -> Self {
        let spans = if f.is_friend {
            vec![
                Span::styled(
                    format!("名称: {}", f.name),
                    Style::default().fg(Color::White),
                ),
                Span::styled("，", Style::default().fg(Color::White)),
                Span::styled("已是好友", Style::default().fg(Color::Green)),
            ]
        } else {
            vec![Span::styled(
                format!("名称: {}", f.name),
                Style::default().fg(Color::White),
            )]
        };
        Line::from(spans).into()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ToChat {
    User(i32, String),
    Group(i32, String),
}

impl Component for Contact {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Contact {
            match self.state {
                State::Friends => match key.code {
                    KeyCode::Char('e') => {
                        self.change_state(State::Search);
                        self.friend_list_state.select(None);
                    }
                    KeyCode::Up => self.friend_list_state.select_previous(),
                    KeyCode::Down => self.friend_list_state.select_next(),
                    KeyCode::Right => {
                        self.friend_list_state.select(None);
                        self.change_state(State::FriendReq)
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.friend_list_state.selected() {
                            if let Some(friend) =
                                self.friends_holder.friends.lock().unwrap().get(idx)
                            {
                                return Ok(Some(Action::ToChat(ToChat::User(
                                    friend.id,
                                    friend.name.clone(),
                                ))));
                            }
                        }
                    }
                    _ => {}
                },
                State::Search => match key.code {
                    KeyCode::Enter => {
                        self.user_input.submit_message();
                        self.search(self.user_input.data().unwrap());
                        self.change_state(State::AddFriend);
                    }
                    KeyCode::Char(to_insert) => self.user_input.enter_char(to_insert),
                    KeyCode::Backspace => self.user_input.delete_char(),
                    KeyCode::Left => self.user_input.move_cursor_left(),
                    KeyCode::Right => self.user_input.move_cursor_right(),
                    KeyCode::Esc => {
                        self.clean_search();
                        self.change_state(State::Friends)
                    }
                    _ => {}
                },
                State::AddFriend => match key.code {
                    KeyCode::Up => self.search_list_state.select_previous(),
                    KeyCode::Down => self.search_list_state.select_next(),
                    KeyCode::Enter => {
                        if let Some(idx) = self.search_list_state.selected() {
                            let friend =
                                self.search_result.lock().unwrap().get(idx).unwrap().clone();
                            let uid = friend.id;
                            let name = friend.name;
                            return Ok(Some(Action::Alert(
                                format!("要添加{name}为好友么？"),
                                Some(ConfirmEvent::AddFriend(uid)),
                            )));
                        }
                    }
                    KeyCode::Esc => {
                        self.clean_search();
                        self.change_state(State::Friends)
                    }
                    _ => {}
                },
                State::FriendReq => match key.code {
                    KeyCode::Char('e') => {
                        self.change_state(State::Search);
                        self.friend_req_list_state.select(None);
                    }
                    KeyCode::Up => self.friend_req_list_state.select_previous(),
                    KeyCode::Down => self.friend_req_list_state.select_next(),
                    KeyCode::Left => {
                        self.friend_req_list_state.select(None);
                        self.change_state(State::Friends)
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.friend_req_list_state.selected() {
                            if let Some(friend_req) =
                                self.friend_req_holder.friend_reqs.lock().unwrap().get(idx)
                            {
                                return Ok(Some(Action::Alert(
                                    format!("接受{}的好友请求么？", friend_req.request_name),
                                    Some(ConfirmEvent::ConfirmFriendReq(None)),
                                )));
                            }
                        }
                    }
                    _ => {}
                },
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Contact && self.friends_holder.need_fetch {
            self.friends_holder.need_fetch = false;
            match friend::friends() {
                Ok(friends) => {
                    self.friends_holder.friends = Arc::new(Mutex::new(friends));
                }
                Err(err) => {
                    error!("Failed to get friends: {}", err);
                }
            };
        }
        if self.mode_holder.get_mode() == Mode::Contact && self.friend_req_holder.need_fetch {
            self.friend_req_holder.need_fetch = false;
            match friend::friend_reqs() {
                Ok(mut friend_reqs) => {
                    friend_reqs.sort_by_key(|f| f.create_time);
                    self.friend_req_holder.friend_reqs = Arc::new(Mutex::new(friend_reqs));
                }
                Err(err) => {
                    error!("Failed to get friend reqs: {}", err);
                }
            };
        }
        match action {
            Action::Confirm(ConfirmEvent::AddFriend(friend_uid)) => {
                let uid = CURRENT_USER.get_user().user.unwrap().id;
                if let Err(e) = friend::add_friend(uid, friend_uid) {
                    return Ok(Some(Action::Alert(e.to_string(), None)));
                }
                self.clean_search();
                self.change_state(State::Friends);
                self.search_list_state.select(None);
            }
            Action::Confirm(ConfirmEvent::ConfirmFriendReq(opt)) => {
                if let Some(b) = opt {
                    if let Some(idx) = self.friend_req_list_state.selected() {
                        if let Some(friend_req) =
                            self.friend_req_holder.friend_reqs.lock().unwrap().get(idx)
                        {
                            match friend::review_friend_req(
                                friend_req.id,
                                if b {
                                    FriendRequestStatus::APPROVE
                                } else {
                                    FriendRequestStatus::REJECT
                                },
                            ) {
                                Ok(_) => {}
                                Err(e) => error!("Failed to review friend req: {}", e),
                            }
                        }
                    }
                }
                self.friend_req_list_state.select(None);
                self.friend_req_holder.need_fetch = true;
                self.friends_holder.need_fetch = true;
                self.change_state(State::Friends)
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() == Mode::Contact {
            let area = area_util::contact_area(area);
            let [search_area, remain_area] =
                Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
            let [friend_area, friend_req_area] = if self.friend_req_holder.has_new_friend_reqs() {
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(remain_area)
            } else {
                Layout::horizontal([Constraint::Percentage(100), Constraint::Percentage(0)])
                    .areas(remain_area)
            };

            let list_block = Block::new()
                .title("↑↓ To Switch, Enter to select friend.")
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
            match self.state {
                State::Friends => {
                    self.render_friends(frame, friend_area, list_block.clone());
                    self.render_friend_reqs(frame, friend_req_area, list_block);
                }
                State::Search | State::AddFriend => {
                    self.user_input.set_cursor_position(search_area);
                    self.render_friend_search_res(frame, remain_area, list_block);
                }
                State::FriendReq => {
                    self.render_friends(frame, friend_area, list_block.clone());
                    self.render_friend_reqs(frame, friend_req_area, list_block);
                }
            }
        }
        Ok(())
    }
}

impl From<&Friend> for Text<'_> {
    fn from(friend: &Friend) -> Self {
        Line::from(Span::styled(
            format!("好友: {}", friend.name),
            Style::default().fg(Color::White),
        ))
        .into()
    }
}
impl From<&FriendReq> for Text<'_> {
    fn from(friend_req: &FriendReq) -> Self {
        let line = Line::from(Span::styled(
            format!("{}请求添加好友", friend_req.request_name),
            Style::default().fg(Color::White),
        ));
        let line1 = Line::from(Span::styled(
            format!("{}", friend_req.status),
            match friend_req.status {
                FriendRequestStatus::WAIT => Style::default().fg(Color::Yellow),
                FriendRequestStatus::APPROVE => Style::default().fg(Color::Green),
                FriendRequestStatus::REJECT => Style::default().fg(Color::Red),
            },
        ));
        let line2 = Line::from(Span::styled(
            format!("时间：{}", friend_req.create_time),
            Style::default().fg(Color::White),
        ));
        Text::from(vec![line, line1, line2])
    }
}
