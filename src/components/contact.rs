use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::recent_chat::SELECTED_STYLE;
use crate::components::user_input::{InputData, UserInput};
use crate::components::{area_util, Component};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Color, Line, Span, Style, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph};
use ratatui::{symbols, Frame};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tracing::info;

pub(crate) struct Contact {
    mode_holder: ModeHolderLock,
    friends: Arc<Mutex<Vec<Friend>>>,
    list_state: ListState,
    user_input: UserInput,
    state: State,
}

#[derive(Default, Eq, PartialEq)]
enum State {
    #[default]
    Friends,
    Search,
}

#[derive(Serialize, Deserialize)]
struct Friend {
    id: i32,
    name: String,
}

impl From<&Friend> for Text<'_> {
    fn from(friend: &Friend) -> Self {
        Line::from(Span::styled(
            format!("好友: {}\n", friend.name),
            Style::default().fg(Color::White),
        ))
        .into()
    }
}

impl Contact {
    pub(crate) fn new(mode_holder: ModeHolderLock) -> Self {
        Self {
            mode_holder,
            friends: Arc::new(Mutex::new(vec![])),
            list_state: Default::default(),
            user_input: UserInput::new(InputData::Search {
                label: Some("Press e to search new friend here.".to_string()),
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

    pub(crate) fn search(&self) {
        info!("Searching...");
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
                    _ => {}
                },
                State::Search => {}
            }
            match key.code {
                KeyCode::Enter => {
                    self.user_input.submit_message();
                    self.search();
                    self.user_input.reset();
                }
                KeyCode::Char(to_insert) => self.user_input.enter_char(to_insert),
                KeyCode::Backspace => self.user_input.delete_char(),
                KeyCode::Left => self.user_input.move_cursor_left(),
                KeyCode::Right => self.user_input.move_cursor_right(),
                KeyCode::Esc => self.next_state(),
                _ => {}
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() == Mode::Contact {
            let area = area_util::contact_area(area);
            let [search_area, friend_area] =
                Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(area);
            let block = Block::new()
                .title("↑↓ To Switch, Enter to select friend.")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_set(symbols::border::ROUNDED);

            // Iterate through all elements in the `items` and stylize them.
            let items: Vec<ListItem> = self
                .friends
                .lock()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(_, friend)| ListItem::new(Text::from(friend)))
                .collect();

            // Create a List from all list items and highlight the currently selected one
            let list = List::new(items)
                .block(block)
                .highlight_style(SELECTED_STYLE)
                .highlight_spacing(HighlightSpacing::Always);
            frame.render_stateful_widget(list, friend_area, &mut self.list_state);

            let block = Block::new()
                .title(self.user_input.input_data.label())
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_set(symbols::border::ROUNDED);
            let user_input =
                Paragraph::new(self.user_input.input.clone().unwrap_or("".to_string()))
                    .style(self.user_input.select_style())
                    .block(block);
            frame.render_widget(user_input, search_area);
            if self.state == State::Search {
                self.user_input.is_editing = true;
                self.user_input.set_cursor_position(search_area)
            }
        }
        Ok(())
    }
}
