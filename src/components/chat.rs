use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::recent_chat::ChatVo;
use crate::components::{area_util, Component};
use crate::datetime::datetime_format;
use crate::proxy;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::prelude::{Color, Line, Span, Style, Text};
use ratatui::style::Stylize;
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::{symbols, Frame};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, LazyLock, Mutex};
use tracing::info;

static CHAT_VO: LazyLock<Arc<Mutex<ChatVoHolder>>> =
    LazyLock::new(|| Arc::new(Mutex::new(ChatVoHolder { chat_vo: None })));

struct ChatVoHolder {
    chat_vo: Option<ChatVo>,
}

impl ChatVoHolder {
    fn set_chat_vo(&mut self, chat_vo: ChatVo) {
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
                let target_name = CHAT_VO.lock().unwrap().get_target_name();
                vec![
                    Line::from(Span::styled(
                        format!("{target_name} {time}\n"),
                        Style::default().fg(Color::LightBlue),
                    )),
                    Line::from(Span::styled(
                        format!("{msg}"),
                        Style::default().fg(crate::components::recent_chat::TEXT_FG_COLOR),
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
                        Style::default().fg(Color::LightBlue),
                    )),
                    Line::from(Span::styled(
                        format!("{msg}"),
                        Style::default().fg(crate::components::recent_chat::TEXT_FG_COLOR),
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
        match key.code {
            KeyCode::Esc => {
                self.mode_holder.set_mode(Mode::RecentChat);
            }
            KeyCode::Down => {
                self.scroll_bar.vertical_scroll = self.scroll_bar.vertical_scroll.saturating_add(1);
                self.scroll_bar.vertical_scroll_state = self
                    .scroll_bar
                    .vertical_scroll_state
                    .position(self.scroll_bar.vertical_scroll);
            }
            KeyCode::Up => {
                self.scroll_bar.vertical_scroll = self.scroll_bar.vertical_scroll.saturating_sub(1);
                self.scroll_bar.vertical_scroll_state = self
                    .scroll_bar
                    .vertical_scroll_state
                    .position(self.scroll_bar.vertical_scroll);
            }
            KeyCode::Char('e') => {
                info!("chat editing");
            }
            _ => {}
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Chat(chat_vo) => {
                CHAT_VO.lock().unwrap().set_chat_vo(chat_vo.clone());
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
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED)
                    .border_style(crate::components::recent_chat::TODO_HEADER_STYLE)
                    .bg(crate::components::recent_chat::NORMAL_ROW_BG);
                // Iterate through all elements in the `items` and stylize them.
                let items = self
                    .chat_history
                    .iter()
                    .map(|chat_history| chat_history.convert_lines())
                    .flatten()
                    .collect::<Vec<_>>();
                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("↑"))
                    .end_symbol(Some("↓"));

                self.scroll_bar.vertical_scroll_state = self
                    .scroll_bar
                    .vertical_scroll_state
                    .content_length(items.len());
                // Create a List from all list items and highlight the currently selected one
                let chat_history = Paragraph::new(items)
                    .block(block)
                    .scroll((self.scroll_bar.vertical_scroll as u16, 0));
                frame.render_widget(
                    chat_history,
                    chat_history_area.inner(Margin {
                        horizontal: 1,
                        vertical: 0,
                    }),
                );
                // and the scrollbar, those are separate widgets
                frame.render_stateful_widget(
                    scrollbar,
                    chat_history_area.inner(Margin {
                        // using an inner vertical margin of 1 unit makes the scrollbar inside the block
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut self.scroll_bar.vertical_scroll_state,
                );
                let block = Block::new()
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED)
                    .border_style(crate::components::recent_chat::TODO_HEADER_STYLE)
                    .bg(crate::components::recent_chat::NORMAL_ROW_BG);
                frame.render_widget(block, chat_area.inner(Margin {
                    // using an inner vertical margin of 1 unit makes the scrollbar inside the block
                    vertical: 0,
                    horizontal: 1,
                }));
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
