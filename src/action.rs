use serde::{Deserialize, Serialize};
use strum::Display;
use crate::components::recent_chat::ChatVo;

#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    ClearScreen,
    Error(String),
    Help,
    Alert(String, Option<ConfirmEvent>),
    Submit,
    Confirm(ConfirmEvent),
    NextTab,
    Chat(ChatVo)
}

#[derive(Clone, Debug, Eq, PartialEq, Display, Serialize, Deserialize)]
pub enum ConfirmEvent {
    Nothing,
    Submit,
    Score,
}
