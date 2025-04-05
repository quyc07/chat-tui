use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::recent_chat::SELECTED_STYLE;
use crate::components::user_input::{InputData, UserInput};
use crate::components::{Component, area_util};
use crate::proxy::friend::Friend;
use crate::proxy::{friend, user};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Color, Line, Span, Style, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, symbols};
use std::sync::{Arc, Mutex};
use tracing::error;

pub(crate) struct Contact {
    mode_holder: ModeHolderLock,
    friends_holder: FriendsHolder,
    search_result: Arc<Mutex<Vec<FriendSearchRes>>>,
    list_state: ListState,
    user_input: UserInput,
    state: State,
}

struct FriendsHolder {
    need_fetch: bool,
    friends: Arc<Mutex<Vec<Friend>>>,
}

#[derive(Default, Eq, PartialEq)]
enum State {
    #[default]
    Friends,
    Search,
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
            list_state: Default::default(),
            user_input: UserInput::new(InputData::Search {
                label: Some("Press e To Search New Friend Here.".to_string()),
                data: None,
            }),
            state: Default::default(),
        }
    }

    fn next_state(&mut self) {
        match self.state {
            State::Friends => {
                self.state = State::Search;
                self.user_input.is_editing = true;
            }
            State::Search => {
                self.state = State::Friends;
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
        frame.render_stateful_widget(list, friend_area, &mut self.list_state);
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
        frame.render_stateful_widget(list, friend_area, &mut self.list_state);
    }
}

struct FriendSearchRes {
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

impl Component for Contact {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Contact {
            match self.state {
                State::Friends => match key.code {
                    KeyCode::Char('e') => {
                        self.next_state();
                    }
                    KeyCode::Up => self.list_state.select_previous(),
                    KeyCode::Down => self.list_state.select_next(),
                    _ => {}
                },
                State::Search => match key.code {
                    KeyCode::Enter => {
                        self.user_input.submit_message();
                        self.search(self.user_input.data().unwrap());
                    }
                    KeyCode::Char(to_insert) => self.user_input.enter_char(to_insert),
                    KeyCode::Backspace => self.user_input.delete_char(),
                    KeyCode::Left => self.user_input.move_cursor_left(),
                    KeyCode::Right => self.user_input.move_cursor_right(),
                    KeyCode::Up => self.list_state.select_previous(),
                    KeyCode::Down => self.list_state.select_next(),
                    KeyCode::Esc => {
                        self.clean_search();
                        self.next_state()
                    }
                    _ => {}
                },
            }
        }
        Ok(None)
    }

    fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Contact
            && self.state == State::Friends
            && self.friends_holder.need_fetch
        {
            match friend::friends() {
                Ok(friends) => {
                    self.friends_holder.need_fetch = false;
                    self.friends_holder.friends = Arc::new(Mutex::new(friends));
                }
                Err(err) => {
                    error!("Failed to get friends: {}", err);
                }
            };
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() == Mode::Contact {
            let area = area_util::contact_area(area);
            let [search_area, friend_area] =
                Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
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
                    self.render_friends(frame, friend_area, list_block);
                }
                State::Search => {
                    self.user_input.set_cursor_position(search_area);
                    self.render_friend_search_res(frame, friend_area, list_block);
                }
            }
        }
        Ok(())
    }
}
