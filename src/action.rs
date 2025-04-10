use crate::components::contact::ToChat;
use crate::components::group_manager::ManageAction;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    ToChat(ToChat),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ConfirmEvent {
    InviteFriend,
    GroupManage(Option<ManageAction>),
    AddFriend(i32),
    ConfirmFriendReq(Option<bool>),
}
