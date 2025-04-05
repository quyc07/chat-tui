use crate::action::Action;
use crate::app::{Mode, ModeHolder, ModeHolderLock};
use crate::components::recent_chat::SELECTED_STYLE;
use crate::components::user_input::{InputData, UserInput};
use crate::components::{Component, area_util};
use crate::proxy;
use crate::proxy::friend::Friend;
use crate::proxy::group::{DetailRes, GroupUser};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Color, Line, Span, Style, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, symbols};
use std::sync::{Arc, Mutex};
use tracing::error;

pub(crate) struct GroupManager {
    mode_holder: ModeHolderLock,
    user_input: UserInput,
    gid: Option<i32>,
    detail: Arc<Mutex<DetailRes>>,
    state: State,
    list_state: ListState,
}

enum State {
    GroupDetail,
    InviteFriend,
}

impl Component for GroupManager {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::GroupManager {
            match self.state {
                State::GroupDetail => match key.code {
                    KeyCode::Esc => {
                        self.mode_holder.set_mode(Mode::Chat);
                    }
                    KeyCode::Char('e') => {
                        self.next_state();
                    }
                    KeyCode::Up => self.list_state.select_previous(),
                    KeyCode::Down => self.list_state.select_next(),
                    _ => {}
                },
                State::InviteFriend => match key.code {
                    KeyCode::Esc => {
                        self.mode_holder.set_mode(Mode::Chat);
                    }
                    KeyCode::Enter => {
                        self.user_input.submit_message();
                        // self.search(self.user_input.data().unwrap()); TODO
                    }
                    KeyCode::Char(to_insert) => self.user_input.enter_char(to_insert),
                    KeyCode::Backspace => self.user_input.delete_char(),
                    KeyCode::Left => self.user_input.move_cursor_left(),
                    KeyCode::Right => self.user_input.move_cursor_right(),
                    KeyCode::Up => self.list_state.select_previous(),
                    KeyCode::Down => self.list_state.select_next(),
                    KeyCode::Esc => {
                        // self.clean_search(); TODO
                        self.next_state()
                    }
                    _ => {}
                },
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if let Action::Group(gid) = action {
            self.gid = Some(gid);
            self.mode_holder.set_mode(Mode::GroupManager);
            match proxy::group::detail(gid) {
                Ok(detail) => {
                    self.detail = Arc::new(Mutex::new(detail));
                }
                Err(err) => error!("fail to fetch group detail: {}", err),
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() != Mode::GroupManager {
            return Ok(());
        }
        let area = area_util::group_manager_area(area);
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
        let user_input = Paragraph::new(self.user_input.input.clone().unwrap_or("".to_string()))
            .style(self.user_input.select_style())
            .block(search_block);
        frame.render_widget(user_input, search_area);
        match self.state {
            State::GroupDetail => {
                self.render_friends(frame, friend_area, list_block);
            }
            State::InviteFriend => {
                self.user_input.set_cursor_position(search_area);
                self.render_friend_search_res(frame, friend_area, list_block);
            }
        }
        Ok(())
    }
}

impl GroupManager {
    pub(crate) fn new(mode_holder: ModeHolderLock) -> Self {
        Self {
            mode_holder,
            user_input: UserInput::new(InputData::Search {
                label: Some("Invite Friend To Join Group".to_string()),
                data: None,
            }),
            gid: None,
            detail: Arc::new(Mutex::new(DetailRes {
                group_id: 0,
                name: "".to_string(),
                users: vec![],
            })),
            state: State::GroupDetail,
            list_state: Default::default(),
        }
    }

    fn render_friends(&mut self, frame: &mut Frame, friend_area: Rect, block: Block) {
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
        frame.render_stateful_widget(list, friend_area, &mut self.list_state);
    }
    fn render_friend_search_res(&mut self, frame: &mut Frame, friend_area: Rect, block: Block) {
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
        frame.render_stateful_widget(list, friend_area, &mut self.list_state);
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
