use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::recent_chat::ChatVo;
use crate::components::user_input::{InputData, UserInput};
use crate::components::{area_util, Component};
use crate::datetime::datetime_format;
use crate::proxy;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Color, Line, Span, Style};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::{symbols, Frame};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock, Mutex};
use tracing::info;

pub(crate) static CHAT_VO: LazyLock<Arc<Mutex<ChatVoHolder>>> =
    LazyLock::new(|| Arc::new(Mutex::new(ChatVoHolder { chat_vo: None })));

pub(crate) struct ChatVoHolder {
    chat_vo: Option<ChatVo>,
}

impl ChatVoHolder {
    pub(crate) fn set_chat_vo(&mut self, chat_vo: ChatVo) {
        self.chat_vo = Some(chat_vo);
    }

    fn get_target_name(&self) -> String {
        match &self.chat_vo {
            None => "未知对象".to_string(),
            Some(chat_vo) => match chat_vo {
                ChatVo::User { user_name, .. } => user_name.to_string(),
                ChatVo::Group { group_name, .. } => group_name.to_string(),
            },
        }
    }
}

pub(crate) struct Chat {
    mode_holder: ModeHolderLock,
    chat_history: Vec<ChatHistory>,
    scroll_bar: ScrollBar,
    user_input: UserInput,
    chat_state: ChatState,
}

impl Chat {
    pub(crate) fn send_msg(&self) {
        info!(
            "sending message: {:?}",
            self.user_input.data().unwrap_or_default()
        );
    }

    fn next_state(&mut self) {
        match self.chat_state {
            ChatState::History => {
                self.chat_state = ChatState::Chat;
                self.user_input.is_editing = true;
            }
            ChatState::Chat => {
                self.chat_state = ChatState::History;
                self.user_input.is_editing = false;
            }
        }
    }
}

#[derive(Eq, PartialEq, Default)]
enum ChatState {
    #[default]
    History,
    Chat,
}

#[derive(Default)]
struct ScrollBar {
    vertical_scroll_state: ScrollbarState,
    vertical_scroll: usize,
}

impl Chat {
    pub(crate) fn new(mode_holder: ModeHolderLock) -> Self {
        Self {
            mode_holder,
            chat_history: Vec::new(),
            scroll_bar: ScrollBar::default(),
            user_input: UserInput::new(InputData::ChatMsg {
                label: Some("Press e to edit msg".to_string()),
                data: None,
            }),
            chat_state: Default::default(),
        }
    }

    fn fetch_history(&mut self, chat_vo: ChatVo) -> color_eyre::Result<Option<Action>> {
        match chat_vo {
            ChatVo::User { uid, user_name, .. } => {
                match proxy::send_request(move || fetch_user_history(uid))? {
                    Ok(chat_history) => {
                        let last_mid = chat_history.last().unwrap().mid;
                        self.chat_history = chat_history
                            .into_iter()
                            .map(|m| ChatHistory::User(m))
                            .collect();
                        // 更新 已读索引
                        proxy::send_request(move || {
                            set_read_index(UpdateReadIndex::User {
                                target_uid: uid,
                                mid: last_mid,
                            })
                            .expect("fail to set read index");
                        })?;
                        Ok(None)
                    }
                    Err(err) => Err(format_err!("Failed to fetch chat history:{}", err)),
                }
            }
            ChatVo::Group {
                gid, group_name, ..
            } => {
                match proxy::send_request(move || fetch_group_history(gid))? {
                    Ok(chat_history) => {
                        let last_mid = chat_history.last().unwrap().mid;
                        self.chat_history = chat_history
                            .into_iter()
                            .map(|m| ChatHistory::Group(m))
                            .collect();
                        // 更新 已读索引
                        proxy::send_request(move || {
                            set_read_index(UpdateReadIndex::Group {
                                target_gid: gid,
                                mid: last_mid,
                            })
                            .expect("fail to set read index");
                        })?;
                        Ok(None)
                    }
                    Err(err) => Err(format_err!("Failed to fetch chat history:{}", err)),
                }
            }
        }
    }
}

impl ChatHistory {
    fn convert_lines(&self) -> Vec<Line> {
        match self {
            ChatHistory::User(UserHistoryMsg {
                mid: _mid,
                msg,
                time,
                from_uid,
            }) => {
                // TODO 添加一个用户名字段
                let target_name = CHAT_VO.lock().unwrap().get_target_name();
                vec![
                    Line::from(Span::styled(
                        format!("{target_name} {time}\n"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("{msg}"),
                        Style::default().fg(Color::Green),
                    )),
                ]
            }
            ChatHistory::Group(GroupHistoryMsg {
                mid: _mid,
                msg,
                time,
                from_uid,
                name_of_from_uid,
            }) => {
                vec![
                    Line::from(Span::styled(
                        format!("{name_of_from_uid} {time}\n"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("{msg}"),
                        Style::default().fg(Color::Green),
                    )),
                ]
            }
        }
    }
}

impl Component for Chat {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() != Mode::Chat {
            return Ok(None);
        }
        match self.chat_state {
            ChatState::History => match key.code {
                KeyCode::Esc => {
                    self.mode_holder.set_mode(Mode::RecentChat);
                }
                KeyCode::Down => {
                    self.scroll_bar.vertical_scroll =
                        self.scroll_bar.vertical_scroll.saturating_add(1);
                    self.scroll_bar.vertical_scroll_state = self
                        .scroll_bar
                        .vertical_scroll_state
                        .position(self.scroll_bar.vertical_scroll);
                }
                KeyCode::Up => {
                    self.scroll_bar.vertical_scroll =
                        self.scroll_bar.vertical_scroll.saturating_sub(1);
                    self.scroll_bar.vertical_scroll_state = self
                        .scroll_bar
                        .vertical_scroll_state
                        .position(self.scroll_bar.vertical_scroll);
                }
                KeyCode::Char('e') => {
                    self.next_state();
                }
                _ => {}
            },
            ChatState::Chat => match key.code {
                KeyCode::Enter => {
                    self.user_input.submit_message();
                    self.send_msg();
                    self.user_input.reset();
                }
                KeyCode::Char(to_insert) => self.user_input.enter_char(to_insert),
                KeyCode::Backspace => self.user_input.delete_char(),
                KeyCode::Left => self.user_input.move_cursor_left(),
                KeyCode::Right => self.user_input.move_cursor_right(),
                KeyCode::Esc => self.next_state(),
                _ => {}
            },
        }
        Ok(None)
    }

    fn update(&mut self, _action: Action) -> color_eyre::Result<Option<Action>> {
        let mut chat_vo_guard = CHAT_VO.lock().unwrap();
        match self.mode_holder.get_mode() {
            Mode::RecentChat | Mode::Chat if chat_vo_guard.chat_vo.is_some() => {
                let chat_vo = chat_vo_guard.chat_vo.take().unwrap();
                self.fetch_history(chat_vo)
            }
            _ => Ok(None),
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        match self.mode_holder.get_mode() {
            Mode::RecentChat | Mode::Chat => {
                let area = area_util::chat(area);
                let [chat_history_area, chat_area] =
                    Layout::vertical([Constraint::Fill(1), Constraint::Length(6)]).areas(area);
                let block = Block::new()
                    .title("Press ↑↓ To Scroll.")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED);
                let items = self
                    .chat_history
                    .iter()
                    .map(|chat_history| chat_history.convert_lines())
                    .flatten()
                    .collect::<Vec<_>>();
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));
                let content_length = self.chat_history.len();
                // let view_length = (chat_history_area.height as usize - 2) / 2;
                // info!("view_length: {}", view_length);
                self.scroll_bar.vertical_scroll_state = self
                    .scroll_bar
                    .vertical_scroll_state
                    .content_length(content_length);
                // .viewport_content_length(view_length);
                let chat_history = Paragraph::new(items)
                    .block(block)
                    .scroll((self.scroll_bar.vertical_scroll as u16, 0));
                frame.render_widget(chat_history, chat_history_area);
                frame.render_stateful_widget(
                    scrollbar,
                    chat_history_area,
                    &mut self.scroll_bar.vertical_scroll_state,
                );
                let block = Block::new()
                    .title(self.user_input.input_data.label())
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED);
                let user_input =
                    Paragraph::new(self.user_input.input.clone().unwrap_or("".to_string()))
                        .style(self.user_input.select_style())
                        .block(block);
                frame.render_widget(user_input, chat_area);
                if self.chat_state == ChatState::Chat {
                    self.user_input.set_cursor_position(chat_area)
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
enum ChatHistory {
    User(UserHistoryMsg),
    Group(GroupHistoryMsg),
}

fn fetch_user_history(target_uid: i32) -> color_eyre::Result<Vec<UserHistoryMsg>> {
    let url = format!("{HOST}/user/{target_uid}/history");
    let token = CURRENT_USER.get_user().token.clone().unwrap();
    let res = Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send();
    match res {
        Ok(res) => match res.status() {
            StatusCode::OK => {
                let res = res.json::<Vec<UserHistoryMsg>>();
                res.or_else(|e| Err(format_err!("Failed to get chat history :{}", e)))
            }
            _ => Err(format_err!("Failed to get chat history:{}", res.status())),
        },
        Err(err) => Err(format_err!("Failed to get chat history:{}", err)),
    }
}

#[derive(Deserialize, Serialize)]
struct UserHistoryMsg {
    /// 消息id
    mid: i64,
    /// 消息内容
    msg: String,
    /// 消息发送时间
    #[serde(with = "datetime_format")]
    time: DateTime<Local>,
    /// 消息发送者id
    from_uid: i32,
}

#[derive(Serialize, Deserialize)]
struct GroupHistoryMsg {
    pub mid: i64,
    pub msg: String,
    #[serde(with = "datetime_format")]
    pub time: DateTime<Local>,
    pub from_uid: i32,
    pub name_of_from_uid: String,
}

fn fetch_group_history(gid: i32) -> color_eyre::Result<Vec<GroupHistoryMsg>> {
    let url = format!("{HOST}/group/{gid}/history");
    let token = CURRENT_USER.get_user().token.clone().unwrap();
    let res = Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send();
    match res {
        Ok(res) => match res.status() {
            StatusCode::OK => {
                let res = res.json::<Vec<GroupHistoryMsg>>();
                res.or_else(|e| Err(format_err!("Failed to get chat history :{}", e)))
            }
            _ => Err(format_err!("Failed to get chat history:{}", res.status())),
        },
        Err(err) => Err(format_err!("Failed to get chat history:{}", err)),
    }
}

#[derive(Serialize)]
enum UpdateReadIndex {
    User { target_uid: i32, mid: i64 },
    Group { target_gid: i32, mid: i64 },
}

fn set_read_index(ri: UpdateReadIndex) -> color_eyre::Result<()> {
    let token = CURRENT_USER.get_user().token.clone().unwrap();
    let res = Client::new()
        .put(format!("{HOST}/ri"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&ri)
        .send();
    match res {
        Ok(res) => {
            println!("{}", res.text().unwrap());
            Ok(())
        }
        Err(err) => Err(format_err!("Failed to set read index:{}", err)),
    }
}
