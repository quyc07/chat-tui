use crate::action::Action;
use crate::app::{Mode, ModeHolderLock, SHOULD_QUIT};
use crate::components::chat::CHAT_VO;
use crate::components::event::ChatMessage;
use crate::components::{Component, area_util};
use crate::datetime::datetime_format;
use crate::proxy;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::palette::tailwind::{BLUE, GREEN, SKY, SLATE};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState};
use ratatui::{Frame, symbols};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::Receiver;
use tokio::time::Duration;
use tracing::{debug, error};

pub(crate) struct RecentChat {
    mode_holder: ModeHolderLock,
    items: Arc<Mutex<Vec<ChatVo>>>,
    list_state: ListState,
    chat_rx: Receiver<ChatMessage>,
}

/// 聊天记录
#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq, Clone)]
pub(crate) enum ChatVo {
    /// UserChat
    User {
        /// id of friend
        uid: i32,
        /// name of friend
        user_name: String,
        /// message id
        mid: i64,
        /// message content
        msg: String,
        /// message time
        #[serde(with = "datetime_format")]
        msg_time: DateTime<Local>,
        /// unread message count
        unread: Option<String>,
    },
    /// GroupChat
    Group {
        /// id of group
        gid: i32,
        /// name of group
        group_name: String,
        /// id of friend
        uid: i32,
        /// name of friend
        user_name: String,
        /// message id
        mid: i64,
        /// message content
        msg: String,
        /// message time
        #[serde(with = "datetime_format")]
        msg_time: DateTime<Local>,
        /// unread message count
        unread: Option<String>,
    },
}

fn recent_chat() -> color_eyre::Result<Vec<ChatVo>> {
    let url = format!("{HOST}/user/history");
    let token = CURRENT_USER.get_user().token.clone().unwrap();
    let res = Client::new()
        .post(url)
        .json(&serde_json::json!({
            "page": 1,
            "limit": 10
        }))
        .header("Authorization", format!("Bearer {}", token))
        .send();
    if let Ok(res) = res {
        if res.status().is_success() {
            res.json::<Vec<ChatVo>>()
                .map_err(|err| format_err!("Fail to Parse Recent Chat: {}", err))
        } else {
            Err(format_err!("Fail to Get Recent Chat"))
        }
    } else {
        Err(format_err!("Fail to Get Recent Chat"))
    }
}

impl From<&ChatVo> for Text<'_> {
    fn from(value: &ChatVo) -> Self {
        match value {
            ChatVo::User {
                uid: _uid,
                user_name,
                msg,
                msg_time,
                unread,
                ..
            } => {
                let mut content = vec![
                    Line::from(Span::styled(
                        format!("好友: {}\n", user_name),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("时间: {}\n", msg_time),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("{}\n", msg),
                        Style::default().fg(Color::Green),
                    )),
                ];
                if let Some(unread) = unread {
                    content.push(Line::from(Span::styled(
                        format!("未读: {}\n", unread),
                        Style::default().fg(Color::White),
                    )))
                }
                Self::from(content)
            }
            ChatVo::Group {
                gid: _gid,
                group_name,
                user_name,
                msg,
                msg_time,
                unread,
                ..
            } => {
                let mut content = vec![
                    Line::from(Span::styled(
                        format!("群: {}\n", group_name),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("时间: {}\n", msg_time),
                        Style::default().fg(Color::White),
                    )),
                    Line::from(Span::styled(
                        format!("{}: {}\n", user_name, msg),
                        Style::default().fg(Color::Green),
                    )),
                ];
                if let Some(unread) = unread {
                    content.push(Line::from(Span::styled(
                        format!("未读: {}\n", unread),
                        Style::default().fg(Color::White),
                    )))
                }
                Self::from(content)
            }
        }
    }
}

impl RecentChat {
    pub fn new(mode_holder: ModeHolderLock, chat_rx: Receiver<ChatMessage>) -> Self {
        let recent_chat = Self {
            mode_holder,
            list_state: Default::default(),
            items: Arc::new(Mutex::new(Vec::new())),
            chat_rx,
        };
        // recent_chat.start_update_thread();
        recent_chat
    }

    fn start_update_thread(&self) {
        let value_clone = self.items.clone();
        tokio::task::spawn_blocking(move || {
            loop {
                if SHOULD_QUIT.lock().unwrap().should_quit {
                    break;
                }
                if CURRENT_USER.get_user().user.is_some() {
                    // 尽快释放锁，方便数据呈现
                    {
                        let mut value = value_clone.lock().unwrap();
                        match recent_chat() {
                            Ok(items) => {
                                *value = items;
                            }
                            Err(err) => {
                                eprintln!("Error: {}", err);
                            }
                        }
                    }
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        });
    }

    fn send_chat(&mut self) -> color_eyre::Result<Option<Action>> {
        let chat_vos = self.items.lock().unwrap();
        match self.list_state.selected() {
            Some(i) if i < chat_vos.len() => {
                let chat_vo = chat_vos.get(i).unwrap().clone();
                debug!("Sending Chat message, chat_vo={:?}", chat_vo);
                CHAT_VO.lock().unwrap().set_chat_vo(chat_vo);
            }
            _ => {}
        }
        Ok(None)
    }
}

impl Component for RecentChat {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        if self.mode_holder.get_mode() != Mode::RecentChat {
            return Ok(None);
        }
        match key.code {
            KeyCode::Down => {
                self.list_state.select_next();
                self.send_chat()
            }
            KeyCode::Up => {
                self.list_state.select_previous();
                self.send_chat()
            }
            KeyCode::Enter => {
                self.mode_holder.set_mode(Mode::Chat);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if action == Action::LoginSuccess && CURRENT_USER.get_user().user.is_some() {
            let arc = self.items.clone();
            proxy::send_request(move || match recent_chat() {
                Ok(items) => {
                    let mut chat_vos = arc.lock().unwrap();
                    *chat_vos = items;
                }
                Err(err) => {
                    error!("fail to fetch recent chat: {err}");
                }
            })?;
        }
        if self.mode_holder.get_mode() != Mode::RecentChat {
            return Ok(None);
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        match self.mode_holder.get_mode() {
            Mode::RecentChat | Mode::Chat => {
                let area = area_util::recent_chat(area);
                let block = Block::new()
                    .title("↑↓ To Switch, Enter to Start Chat.")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_set(symbols::border::ROUNDED);
                // .border_style(TODO_HEADER_STYLE)
                // .bg(NORMAL_ROW_BG);

                // Iterate through all elements in the `items` and stylize them.
                let items: Vec<ListItem> = self
                    .items
                    .lock()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .map(|(i, chat_vo)| ListItem::new(Text::from(chat_vo)))
                    .collect();

                // Create a List from all list items and highlight the currently selected one
                let list = List::new(items)
                    .block(block)
                    .highlight_style(SELECTED_STYLE)
                    .highlight_spacing(HighlightSpacing::Always);

                // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
                // same method name `render`.
                frame.render_stateful_widget(list, area, &mut self.list_state);
                // 首次加载默认选中第一个
                // TODO 首次加载第一个会出现双眼皮现象，暂时无法解决，后续再说吧
                // let first_check = FIRST.lock().unwrap();
                // if first_check.check && self.list_state.selected().is_none() {
                //     info!("select first");
                //     self.list_state.select_first();
                // }
            }
            _ => {}
        }
        Ok(())
    }
}

const fn alternate_colors(i: usize) -> Color {
    if i % 2 == 0 {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}

pub(crate) const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
pub(crate) const NORMAL_ROW_BG: Color = SLATE.c200;
const ALT_ROW_BG_COLOR: Color = SLATE.c300;
pub(crate) const SELECTED_STYLE: Style = Style::new().bg(SKY.c500).add_modifier(Modifier::BOLD);
pub(crate) const TEXT_FG_COLOR: Color = SLATE.c600;
const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;
