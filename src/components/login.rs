use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::user_input::{InputData, UserInput};
use crate::components::{area_util, Component};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use tracing::info;

pub(crate) struct Login {
    mode_holder: ModeHolderLock,
    user_name_input: UserInput,
    password_input: UserInput,
    state: State,
}

impl Login {
    pub fn new(mode_holder: ModeHolderLock) -> Self {
        Self {
            mode_holder,
            user_name_input: UserInput::new(InputData::UserName {
                label: Some("用户名".to_string()),
                data: None,
            }),
            password_input: UserInput::new(InputData::Password {
                label: Some("密码".to_string()),
                data: None,
            }),
            state: State::Normal,
        }
    }

    fn next_state(&mut self) {
        match self.state {
            State::Normal => {
                self.state = State::UserNameEditing;
                self.user_name_input.is_editing = true;
            }
            State::UserNameEditing => {
                self.state = State::PasswordEditing;
                self.user_name_input.is_editing = false;
                self.password_input.is_editing = true;
            }
            State::PasswordEditing => {
                self.state = State::Normal;
                self.password_input.is_editing = false;
            }
        }
    }
}

#[derive(PartialEq, Eq)]
enum State {
    Normal,
    UserNameEditing,
    PasswordEditing,
}

impl Component for Login {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Login {
            match self.state {
                State::Normal => {
                    if let KeyCode::Char('e') = key.code {
                        self.next_state();
                        info!("start editing user");
                    }
                }
                State::UserNameEditing => match key.code {
                    KeyCode::Enter => {
                        self.user_name_input.submit_message();
                        self.next_state();
                    }
                    KeyCode::Char(to_insert) => self.user_name_input.enter_char(to_insert),
                    KeyCode::Backspace => self.user_name_input.delete_char(),
                    KeyCode::Left => self.user_name_input.move_cursor_left(),
                    KeyCode::Right => self.user_name_input.move_cursor_right(),
                    _ => {}
                },
                State::PasswordEditing => match key.code {
                    KeyCode::Enter => {
                        self.password_input.submit_message();
                        self.next_state();
                    }
                    KeyCode::Char(to_insert) => self.password_input.enter_char(to_insert),
                    KeyCode::Backspace => self.password_input.delete_char(),
                    KeyCode::Left => self.password_input.move_cursor_left(),
                    KeyCode::Right => self.password_input.move_cursor_right(),
                    _ => {}
                },
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Login {
            if action == Action::Confirm {
                // todo!("调用login接口")
                info!(
                    "Username: {}, Password: {}",
                    self.user_name_input.data().unwrap_or("***".to_string()),
                    self.password_input.data().unwrap_or("***".to_string())
                );
                self.mode_holder.set_mode(Mode::RecentChat);
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() != Mode::Login {
            return Ok(());
        }
        let bg_block = Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Green));

        let area = area_util::total_area(area);
        frame.render_widget(bg_block, area);

        let [cli_name_area, help_area, user_name_area, password_area, _] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Max(2),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .areas(area);

        let banner = r#"
 ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░▒▓██████▓▒░▒▓████████▓▒░▒▓██████▓▒░░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░  ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░  ░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓█▓▒░      ░▒▓████████▓▒░▒▓████████▓▒░ ░▒▓█▓▒░  ░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓█▓▒░      ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░  ░▒▓█▓▒░      ░▒▓█▓▒░      ░▒▓█▓▒░
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░  ░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░      ░▒▓█▓▒░
 ░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ ░▒▓█▓▒░   ░▒▓██████▓▒░░▒▓████████▓▒░▒▓█▓▒░
        "#;

        let banner_paragraph = Paragraph::new(banner)
            .block(Block::default().borders(Borders::NONE))
            .centered();

        frame.render_widget(
            banner_paragraph,
            area_util::centered_rect(100, 50, cli_name_area),
        );

        let (msg, style) = match self.state {
            State::Normal => (
                vec![
                    "Press e to start editing, ".bold(),
                    "Ctrl+S to login.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            State::UserNameEditing | State::PasswordEditing => (
                vec!["Press Enter to move to next. ".into()],
                Style::default(),
            ),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text).wrap(ratatui::widgets::Wrap { trim: true }); // 添加自动换行
        frame.render_widget(help_message, area_util::centered_rect(50, 100, help_area));

        let user_name_area = area_util::centered_rect(50, 100, user_name_area);
        let password_area = area_util::centered_rect(50, 100, password_area);
        let user_name =
            Paragraph::new(self.user_name_input.input.clone().unwrap_or("".to_string()))
                .style(self.user_name_input.select_style())
                .block(Block::bordered().title(self.user_name_input.input_data.label()));
        frame.render_widget(user_name, user_name_area);

        let password = Paragraph::new(self.password_input.input.clone().unwrap_or("".to_string()))
            .style(self.password_input.select_style())
            .block(Block::bordered().title(self.password_input.input_data.label()));
        frame.render_widget(password, password_area);
        match self.state {
            State::Normal => {}
            State::UserNameEditing => {
                self.user_name_input.set_cursor_position(user_name_area);
            }
            State::PasswordEditing => {
                self.password_input.set_cursor_position(password_area);
            }
        }
        Ok(())
    }
}
