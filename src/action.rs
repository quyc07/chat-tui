use crate::components::group_manager::ManageAction;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Clone, Debug, PartialEq, Eq, Display, Serialize, Deserialize)]
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
    LoginSuccess,
    Confirm(ConfirmEvent),
    NextTab,
    Register,
    Group(i32),
}

#[derive(Clone, Debug, Eq, PartialEq, Display, Serialize, Deserialize)]
pub enum ConfirmEvent {
    InviteFriend,
    GroupManage(Option<ManageAction>),
    AddFriend(i32),
    ConfirmFriendReq(Option<bool>),
}
