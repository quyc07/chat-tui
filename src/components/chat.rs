use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::event::{ChatMessage, MessageTarget};
use crate::components::recent_chat::ChatVo;
use crate::components::user_input::{InputData, UserInput};
use crate::components::{Component, area_util};
use crate::datetime::datetime_format;
use crate::proxy;
use crate::proxy::{HOST, user};
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::prelude::{Color, Line, Span, Style};
use ratatui::widgets::{
    Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::{Frame, symbols};
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock, Mutex};
use tokio::sync::broadcast::Receiver;
use tracing::{debug, error};

pub(crate) static CHAT_VO: LazyLock<Arc<Mutex<ChatVoHolder>>> = LazyLock::new(|| {
    Arc::new(Mutex::new(ChatVoHolder {
        chat_vo: None,
        need_fetch: false,
    }))
});

pub(crate) struct ChatVoHolder {
    chat_vo: Option<ChatVo>,
    // 是否需要重新获取历史消息
    need_fetch: bool,
}

impl ChatVoHolder {
    pub(crate) fn set_chat_vo(&mut self, chat_vo: ChatVo) {
        self.chat_vo = Some(chat_vo);
        self.need_fetch = true;
    }
}

pub(crate) struct Chat {
    mode_holder: ModeHolderLock,
    chat_history: Arc<Mutex<Vec<ChatHistory>>>,
    scroll_bar: ScrollBar,
    user_input: UserInput,
    chat_state: ChatState,
    chat_rx: Arc<tokio::sync::Mutex<Receiver<ChatMessage>>>,
}

impl Chat {
    pub(crate) fn new(mode_holder: ModeHolderLock, chat_rx: Receiver<ChatMessage>) -> Self {
        let mut chat = Self {
            mode_holder,
            chat_history: Arc::new(Mutex::new(Vec::new())),
            scroll_bar: ScrollBar::default(),
            user_input: UserInput::new(InputData::ChatMsg {
                label: Some("Press e to edit msg".to_string()),
                data: None,
            }),
            chat_state: Default::default(),
            chat_rx: Arc::new(tokio::sync::Mutex::new(chat_rx)),
        };
        chat.refresh();
        chat
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

impl ScrollBar {
    fn reset(&mut self) {
        self.vertical_scroll_state = Default::default();
        self.vertical_scroll = 0;
    }
}

impl Chat {
    pub(crate) fn send_msg(&self) {
        let guard = CHAT_VO.lock().unwrap();
        if let Some(msg) = self.user_input.data() {
            match guard.chat_vo.clone().unwrap() {
                ChatVo::User { uid, .. } => {
                    if let Err(err) = send_user_msg(uid, msg) {
                        error!("error sending user msg: {}", err);
                    }
                }
                ChatVo::Group { gid, .. } => {
                    if let Err(err) = send_group_msg(gid, msg) {
                        error!("error sending user msg: {}", err);
                    }
                }
            }
        }
    }

    fn refresh(&mut self) {
        let chat_history = Arc::clone(&self.chat_history);
        let chat_vo_current = Arc::clone(&CHAT_VO);
        let chat_rx = self.chat_rx.clone();
        tokio::spawn(async move {
            while let Ok(chat_message) = chat_rx.lock().await.recv().await {
                debug!("received chat_message: {:?}", chat_message);
                match chat_message.payload.target {
                    MessageTarget::User(target_user) => {
                        if let Some(ChatVo::User { .. }) = chat_vo_current.lock().unwrap().chat_vo {
                            let user = CURRENT_USER.get_user().user.unwrap();
                            if user.id == target_user.uid {
                                let history = UserHistoryMsg {
                                    mid: chat_message.mid,
                                    msg: chat_message.payload.detail.get_content(),
                                    time: chat_message.payload.created_at,
                                    from_uid: chat_message.payload.from_uid,
                                    from_name: user::detail_by_id(chat_message.payload.from_uid)
                                        .unwrap()
                                        .name
                                        .clone(),
                                };
                                let mut guard = chat_history.lock().unwrap();
                                guard.push(ChatHistory::User(history));
                            } else if user.id == chat_message.payload.from_uid {
                                let history = UserHistoryMsg {
                                    mid: chat_message.mid,
                                    msg: chat_message.payload.detail.get_content(),
                                    time: chat_message.payload.created_at,
                                    from_uid: chat_message.payload.from_uid,
                                    from_name: user.name.clone(),
                                };
                                let mut guard = chat_history.lock().unwrap();
                                guard.push(ChatHistory::User(history));
                            };
                        }
                    }
                    MessageTarget::Group(target_group) => {
                        let option = chat_vo_current.lock().unwrap().chat_vo.clone();
                        if let Some(ChatVo::Group { gid, .. }) = option {
                            if gid == target_group.gid {
                                let from_name = if CURRENT_USER.get_user().user.unwrap().id
                                    == chat_message.payload.from_uid
                                {
                                    CURRENT_USER.get_user().user.unwrap().name.clone()
                                } else {
                                    user::detail_by_id(chat_message.payload.from_uid)
                                        .unwrap()
                                        .name
                                        .clone()
                                };
                                let history = GroupHistoryMsg {
                                    mid: chat_message.mid,
                                    msg: chat_message.payload.detail.get_content(),
                                    time: chat_message.payload.created_at,
                                    from_uid: chat_message.payload.from_uid,
                                    name_of_from_uid: from_name,
                                };
                                let mut guard = chat_history.lock().unwrap();
                                guard.push(ChatHistory::Group(history));
                            }
                        }
                    }
                }
            }
        });
    }

    fn fetch_history(&mut self, chat_vo: ChatVo) -> color_eyre::Result<Option<Action>> {
        self.chat_history.lock().unwrap().clear();
        match chat_vo {
            ChatVo::User { uid, .. } => {
                match proxy::send_request(move || fetch_user_history(uid))? {
                    Ok(chat_history) => {
                        let last_mid = chat_history.last().unwrap().mid;
                        let mut guard = self.chat_history.lock().unwrap();
                        chat_history
                            .into_iter()
                            .map(ChatHistory::User)
                            .for_each(|c| guard.push(c));
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
            ChatVo::Group { gid, .. } => {
                match proxy::send_request(move || fetch_group_history(gid))? {
                    Ok(chat_history) => {
                        let last_mid = chat_history.last().unwrap().mid;
                        let mut guard = self.chat_history.lock().unwrap();
                        chat_history
                            .into_iter()
                            .map(ChatHistory::Group)
                            .for_each(|c| guard.push(c));
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
                from_uid: _,
                from_name,
            }) => {
                vec![
                    Line::from(Span::styled(
                        format!("{from_name} {time}\n"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        msg.to_string(),
                        Style::default().fg(Color::Green),
                    )),
                ]
            }
            ChatHistory::Group(GroupHistoryMsg {
                mid: _mid,
                msg,
                time,
                from_uid: _,
                name_of_from_uid,
            }) => {
                vec![
                    Line::from(Span::styled(
                        format!("{name_of_from_uid} {time}\n"),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        msg.to_string(),
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
            Mode::RecentChat | Mode::Chat
                if chat_vo_guard.chat_vo.is_some()
                    && chat_vo_guard.need_fetch
                    && self.chat_state == ChatState::History =>
            {
                debug!("Chat need fetch, chat_vo={:?}", chat_vo_guard.chat_vo);
                chat_vo_guard.need_fetch = false;
                self.scroll_bar.reset();
                let chat_vo = chat_vo_guard.chat_vo.clone().unwrap();
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
                let chat_history = self.chat_history.lock().unwrap();
                let items = chat_history
                    .iter()
                    .flat_map(|chat_history| chat_history.convert_lines())
                    .collect::<Vec<_>>();
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));
                let content_length = chat_history.len();
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

/// Send message request
#[derive(Serialize)]
pub struct SendMsgReq {
    /// Message content
    pub msg: String,
}

fn send_user_msg(uid: i32, msg: String) -> color_eyre::Result<()> {
    let url = format!("{HOST}/user/{uid}/send");
    proxy::send_request(|| send_msg(msg, url))?
}

fn send_group_msg(gid: i32, msg: String) -> color_eyre::Result<()> {
    let url = format!("{HOST}/group/{gid}/send");
    proxy::send_request(|| send_msg(msg, url))?
}

fn send_msg(msg: String, url: String) -> color_eyre::Result<()> {
    let token = CURRENT_USER.get_user().token.clone().unwrap();
    debug!("sending msg to {url}, msg: {msg}");
    let res = Client::new()
        .put(url)
        .json(&SendMsgReq { msg })
        .header("Authorization", format!("Bearer {token}"))
        .send();
    match res {
        Ok(res) => match res.status() {
            StatusCode::OK => {
                debug!("sent msg: {res:?}");
                Ok(())
            }
            _ => Err(format_err!("Failed to send message: {res:?}")),
        },
        Err(err) => Err(format_err!("Failed to send message:{err}")),
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
                res.map_err(|e| format_err!("Failed to get chat history :{}", e))
            }
            _ => Err(format_err!("Failed to get chat history:{}", res.status())),
        },
        Err(err) => Err(format_err!("Failed to get chat history:{}", err)),
    }
}

#[derive(Debug, Deserialize, Serialize)]
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
    /// 消息发送者name
    from_name: String,
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
                res.map_err(|e| format_err!("Failed to get chat history :{}", e))
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
