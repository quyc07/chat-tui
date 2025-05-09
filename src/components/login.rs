use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::user_input::{InputData, UserInput};
use crate::components::{Component, area_util};
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use crate::{proxy, token};
use color_eyre::eyre::format_err;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, Paragraph};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use tracing::error;

pub(crate) struct Login {
    mode_holder: ModeHolderLock,
    user_name_input: UserInput,
    password_input: UserInput,
    state: State,
    // 终止程序信号
    quit_tx: Option<Sender<()>>,
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
            quit_tx: None,
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

/// Register New User
#[derive(Debug, Serialize, Deserialize)]
struct UserRegisterReq {
    /// name
    name: String,
    /// email
    email: Option<String>,
    /// password
    password: String,
    /// phone
    phone: Option<String>,
}

fn register(req: UserRegisterReq) -> color_eyre::Result<i32> {
    let register_url = format!("{}/user/register", HOST.as_str());
    let client = Client::new();
    let response = client.post(register_url).json(&req).send();

    match response {
        Ok(res) => {
            if res.status().is_success() {
                match res.text() {
                    Ok(uid) => Ok(uid.parse::<i32>()?),
                    Err(e) => Err(format_err!("Failed to parse response: {}", e)),
                }
            } else {
                Err(format_err!("Register failed: HTTP {}", res.status()))
            }
        }
        Err(e) => Err(format_err!("Failed to send register request: {}", e)),
    }
}

struct LoginReq {
    user_name: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRes {
    pub access_token: String,
}

fn login(login: LoginReq) -> color_eyre::Result<String> {
    let url = format!("{}/token/login", HOST.as_str());
    let client = Client::new();
    let response = client
        .post(url)
        .json(&serde_json::json!({
            "name": login.user_name,
            "password": login.password,
        }))
        .send();

    match response {
        Ok(res) => {
            if res.status().is_success() {
                match res.json::<LoginRes>() {
                    Ok(LoginRes { access_token }) => Ok(access_token),
                    Err(e) => Err(format_err!("Failed to parse response: {}", e)),
                }
            } else if res.status() == StatusCode::UNAUTHORIZED {
                Err(format_err!("用户名或密码错误"))
            } else {
                Err(format_err!("Login failed: HTTP {}", res.status()))
            }
        }
        Err(e) => Err(format_err!("Failed to send login request: {}", e)),
    }
}

fn renew(quit_rx: Receiver<()>) {
    // 启动异步线程，定时刷新token过期时间
    thread::spawn(move || {
        loop {
            match quit_rx.recv_timeout(Duration::from_secs(60)) {
                Ok(_) => {
                    break;
                }
                Err(_) => {
                    let token = match CURRENT_USER.get_user().token.clone() {
                        None => {
                            break;
                        }
                        Some(token) => token,
                    };
                    let token = format!("Bearer {token}");
                    let renew_url = format!("{}/token/renew", HOST.as_str());
                    let client = Client::new();
                    let response = client
                        .patch(renew_url)
                        .header("Authorization", token.clone())
                        .send();
                    match response {
                        Ok(res) => {
                            if res.status().is_success() {
                                match res.text() {
                                    Ok(t) => {
                                        let token_data = token::parse_token(t.as_str()).unwrap();
                                        CURRENT_USER.set_user(Some(token_data.claims), Some(t));
                                    }
                                    Err(e) => {
                                        error!("Failed to parse response: {}", e);
                                    }
                                }
                            } else {
                                error!("Token refresh failed: HTTP {}", res.status());
                            }
                        }
                        Err(err) => {
                            error!("Failed to send token refresh request: {}", err);
                        }
                    }
                }
            };
        }
    });
}

impl Component for Login {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() == Mode::Login {
            match self.state {
                State::Normal => {
                    if let KeyCode::Char('e') = key.code {
                        self.next_state();
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
        if action == Action::Quit && self.quit_tx.is_some() {
            self.quit_tx.clone().unwrap().send(())?;
            return Ok(None);
        }
        if self.mode_holder.get_mode() == Mode::Login {
            return match action {
                Action::Submit => {
                    match (self.user_name_input.data(), self.password_input.data()) {
                        (None, None) => {
                            return Ok(Some(Action::Alert("请输入用户名和密码".to_string(), None)));
                        }
                        (None, Some(_)) => {
                            return Ok(Some(Action::Alert("请输入用户名".to_string(), None)));
                        }
                        (Some(_), None) => {
                            return Ok(Some(Action::Alert("请输入密码".to_string(), None)));
                        }
                        _ => {
                            let user_name = self.user_name_input.data().unwrap();
                            let password = self.password_input.data().unwrap();
                            // 当前环境为异步环境，但是本方法为同步方法，不能在同步方法中直接调用异步方法，但是reqwest的同步客户端无法在异步环境中使用
                            // 因此此处使用tokio的同步方法结合futures的同步执行器获取结果
                            let result = proxy::send_request(|| {
                                login(LoginReq {
                                    user_name,
                                    password,
                                })
                            })?;
                            match result {
                                Ok(token) => {
                                    let token_data = token::parse_token(token.as_str()).unwrap();
                                    CURRENT_USER.set_user(Some(token_data.claims), Some(token));
                                    let (quit_tx, quit_rx) = mpsc::channel();
                                    self.quit_tx = Some(quit_tx);
                                    renew(quit_rx);
                                    self.mode_holder.set_mode(Mode::RecentChat);
                                    Ok(Some(Action::LoginSuccess))
                                }
                                Err(err) => {
                                    error!("login failed, {err}");
                                    Ok(Some(Action::Alert(format!("{err}"), None)))
                                }
                            }
                        }
                    }
                }
                Action::Register => {
                    let user_name = self.user_name_input.data().unwrap();
                    let password = self.password_input.data().unwrap();
                    let result = proxy::send_request(|| {
                        register(UserRegisterReq {
                            name: user_name,
                            email: None,
                            password,
                            phone: None,
                        })
                    })?;
                    match result {
                        Ok(_) => Ok(Some(Action::Submit)),
                        Err(e) => {
                            error!("register failed, {e}");
                            Ok(Some(Action::Alert(format!("{e}"), None)))
                        }
                    }
                }
                _ => Ok(None),
            };
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

        let [banner_area, help_area, user_name_area, password_area, _] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Max(2),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .areas(area);

        let banner = include_str!("../../banner.txt");

        let banner_paragraph = Paragraph::new(banner)
            .block(Block::default().borders(Borders::NONE))
            .centered();

        frame.render_widget(
            banner_paragraph,
            area_util::centered_rect(100, 50, banner_area),
        );

        let (msg, style) = match self.state {
            State::Normal => (
                vec![
                    "Press e To Start Editing, ".bold(),
                    "Ctrl+S To Login, ".bold(),
                    "Ctrl+R To Register.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            State::UserNameEditing | State::PasswordEditing => (
                vec!["Press Enter To Move To Next. ".into()],
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
