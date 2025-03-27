use crate::action::Action;
use crate::app::{Mode, ModeHolderLock};
use crate::components::{Component, area_util};
use crate::datetime::datetime_format;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::palette::tailwind::{BLUE, GREEN, SLATE, TEAL};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState};
use ratatui::{Frame, symbols};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::time::Duration;
use tracing::info;

pub(crate) struct RecentChat {
    mode_holder: ModeHolderLock,
    items: Arc<Mutex<Vec<ChatVo>>>,
    list_state: ListState,
}

/// 聊天记录
#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
enum ChatVo {
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
                        Style::default().fg(Color::LightBlue),
                    )),
                    Line::from(Span::styled(
                        format!("时间: {}\n", msg_time),
                        Style::default().fg(Color::LightBlue),
                    )),
                    Line::from(Span::styled(
                        format!("{}\n", msg),
                        Style::default().fg(TEXT_FG_COLOR),
                    )),
                ];
                if let Some(unread) = unread {
                    content.push(Line::from(Span::styled(
                        format!("未读: {}\n", unread),
                        Style::default().fg(Color::LightBlue),
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
                        Style::default().fg(Color::LightBlue),
                    )),
                    Line::from(Span::styled(
                        format!("时间: {}\n", msg_time),
                        Style::default().fg(Color::LightBlue),
                    )),
                    Line::from(Span::styled(
                        format!("{}: {}\n", user_name, msg),
                        Style::default().fg(TEXT_FG_COLOR),
                    )),
                ];
                if let Some(unread) = unread {
                    content.push(Line::from(Span::styled(
                        format!("未读: {}\n", unread),
                        Style::default().fg(Color::LightBlue),
                    )))
                }
                Self::from(content)
            }
        }
    }
}

impl RecentChat {
    pub fn new(mode_holder: ModeHolderLock) -> Self {
        let recent_chat = Self {
            mode_holder,
            list_state: Default::default(),
            items: Arc::new(Mutex::new(Vec::new())),
        };
        recent_chat.start_update_thread();
        recent_chat
    }

    fn start_update_thread(&self) {
        let value_clone = self.items.clone();
        tokio::task::spawn_blocking(move || {
            loop {
                if CURRENT_USER.get_user().user.is_some() {
                    let name = CURRENT_USER.get_user().user.unwrap().name;
                    info!("User : {name}");
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
}

impl Component for RecentChat {
    fn handle_key_event(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
        Ok(None)
    }

    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        if self.mode_holder.get_mode() != Mode::RecentChat {
            return Ok(());
        }
        let area = area_util::dynamic_area(area);
        let block = Block::new()
            .borders(Borders::ALL)
            .border_set(symbols::border::ROUNDED)
            .border_style(TODO_HEADER_STYLE)
            .bg(NORMAL_ROW_BG);

        // Iterate through all elements in the `items` and stylize them.
        let items: Vec<ListItem> = self
            .items
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, chat_vo)| {
                let color = alternate_colors(i);
                ListItem::new(Text::from(chat_vo)).bg(color)
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        let list = List::new(items)
            .block(block)
            .highlight_style(SELECTED_STYLE)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        // We need to disambiguate this trait method as both `Widget` and `StatefulWidget` share the
        // same method name `render`.
        frame.render_stateful_widget(list, area, &mut self.list_state);
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

const TODO_HEADER_STYLE: Style = Style::new().fg(SLATE.c100).bg(BLUE.c800);
const NORMAL_ROW_BG: Color = SLATE.c200;
const ALT_ROW_BG_COLOR: Color = SLATE.c300;
const SELECTED_STYLE: Style = Style::new().bg(TEAL.c200).add_modifier(Modifier::BOLD);
const TEXT_FG_COLOR: Color = SLATE.c600;
const COMPLETED_TEXT_FG_COLOR: Color = GREEN.c500;
