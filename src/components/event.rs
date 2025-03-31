use crate::action::Action;
use crate::components::Component;
use crate::datetime::datetime_format;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use futures::StreamExt;
use ratatui::layout::Rect;
use ratatui::Frame;
use reqwest::header::USER_AGENT;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::broadcast::Sender;
use tokio::time::sleep;
use tokio_util::codec::Decoder;
use tracing::{error, info};

pub(crate) struct Event {
    chat_tx: Sender<ChatMessage>,
    fetch: Arc<Mutex<Fetch>>,
}

#[derive(Default)]
struct Fetch {
    need: bool,
}

impl Component for Event {
    fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
        if action == Action::LoginSuccess {
            self.fetch.lock().unwrap().need = true;
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        Ok(())
    }
}

impl Event {
    pub(crate) fn new(chat_tx: Sender<ChatMessage>) -> Self {
        Self {
            chat_tx,
            fetch: Arc::new(Mutex::new(Fetch::default())),
        }
    }

    pub(crate) async fn run(&self) {
        let arc = self.fetch.clone();
        tokio::task::spawn(async move {
            // 检查是否可以开始fetch消息
            check_need_fetch(arc).await;
            let url = format!("{HOST}/event/stream");
            let token = CURRENT_USER.get_user().token.clone().unwrap();
            let res = Client::new()
                .get(url)
                .header("Authorization", format!("Bearer {token}"))
                .header("User-Agent", "Chat-Tui")
                .header("Accept", "application/event-stream")
                .send()
                .await;
            match res {
                Ok(res) => {
                    match res.status() {
                        StatusCode::OK => {
                            let mut stream = res.bytes_stream();
                            while let Some(item) = stream.next().await {
                                let bytes = item.unwrap();
                                let cow = String::from_utf8_lossy(&bytes);
                                info!("{}", cow);
                                // self.chat_tx.send()
                            }
                        }
                        _ => {
                            let text = res.text().await.unwrap();
                            info!("{text}");
                        }
                    }
                }
                Err(e) => {
                    error!("fail to get event stream: {}", e)
                }
            }
        });
    }
}
async fn check_need_fetch(arc: Arc<Mutex<Fetch>>) {
    if !arc.lock().unwrap().need {
        loop {
            sleep(Duration::from_secs(5)).await;
            if arc.lock().unwrap().need {
                break;
            }
        }
    }
}

/// Chat message
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ChatMessage {
    /// Message id
    pub mid: i64,
    pub payload: ChatMessagePayload,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessagePayload {
    /// Sender id
    pub from_uid: i32,

    #[serde(with = "datetime_format")]
    /// The create time of the message.
    pub created_at: DateTime<Local>,

    /// Message target
    pub target: MessageTarget,

    /// Message detail
    pub detail: MessageDetail,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum MessageTarget {
    User(MessageTargetUser),
    Group(MessageTargetGroup),
}

impl From<MessageTarget> for String {
    fn from(value: MessageTarget) -> Self {
        match value {
            MessageTarget::User(MessageTargetUser { uid }) => format!("MessageTargetUser:{uid}"),
            MessageTarget::Group(MessageTargetGroup { gid }) => {
                format!("MessageTargetGroup:{gid}")
            }
        }
    }
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct MessageTargetUser {
    pub uid: i32,
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct MessageTargetGroup {
    pub gid: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageDetail {
    Normal(MessageNormal),
    Replay(MessageReplay),
}

impl MessageDetail {
    pub fn get_content(&self) -> String {
        match self {
            MessageDetail::Normal(msg) => msg.content.content.clone(),
            MessageDetail::Replay(msg) => msg.content.content.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageNormal {
    pub content: MessageContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageReplay {
    pub mid: i64,
    pub content: MessageContent,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageContent {
    /// Extended attributes
    // pub properties: Option<HashMap<String, Value>>,
    /// Content type
    // pub content_type: String,
    /// Content
    pub(crate) content: String,
}
