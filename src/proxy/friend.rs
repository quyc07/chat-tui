use crate::datetime::datetime_format;
use crate::proxy::send_request;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Serialize, Deserialize)]
pub(crate) struct Friend {
    pub(crate) id: i32,
    pub(crate) name: String,
}

pub(crate) fn friends() -> color_eyre::Result<Vec<Friend>> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .get(format!("{}/friend", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => {
                    let res = res.json::<Vec<Friend>>();
                    res.map_err(|err| {
                        format_err!(
                            "Failed to get friends, id: {}, err: {}",
                            current_user.user.unwrap().id,
                            err
                        )
                    })
                }
                _ => Err(format_err!(
                    "Failed to get friends, id: {}, status: {}",
                    current_user.user.unwrap().id,
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to get friends, id: {}, err: {}",
                current_user.user.unwrap().id,
                err
            )),
        }
    })?
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FriendReq {
    pub(crate) id: i32,
    pub(crate) request_id: i32,
    pub(crate) request_name: String,
    #[serde(with = "datetime_format")]
    pub(crate) create_time: DateTime<Local>,
    pub(crate) reason: Option<String>,
    pub(crate) status: FriendRequestStatus,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
pub(crate) enum FriendRequestStatus {
    #[strum(to_string = "等待处理")]
    WAIT,
    #[strum(to_string = "已通过")]
    APPROVE,
    #[strum(to_string = "已拒绝")]
    REJECT,
}

pub(crate) fn friend_reqs() -> color_eyre::Result<Vec<FriendReq>> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .get(format!("{}/friend/req", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => {
                    let res = res.json::<Vec<FriendReq>>();
                    res.map_err(|err| {
                        format_err!(
                            "Failed to get friend reqs, id: {}, err: {}",
                            current_user.user.unwrap().id,
                            err
                        )
                    })
                }
                _ => Err(format_err!(
                    "Failed to get friend reqs, id: {}, status: {}",
                    current_user.user.unwrap().id,
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to get friend reqs, id: {}, err: {}",
                current_user.user.unwrap().id,
                err
            )),
        }
    })?
}

pub(crate) fn add_friend(uid: i32, friend_uid: i32) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .post(format!("{}/friend/req/{friend_uid}", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({}))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                StatusCode::CREATED => {
                    Err(format_err!("Failed to add friend, {}", res.text().unwrap()))
                }
                _ => Err(format_err!(
                    "Failed to add friend, uid: {uid}, friend_uid: {friend_uid}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to add friend, uid: {uid}, friend_uid: {friend_uid}, err: {err}"
            )),
        }
    })?
}

pub(crate) fn review_friend_req(
    req_id: i32,
    status: FriendRequestStatus,
) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .post(format!("{}/friend/req", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({
                "id": req_id,
                "status": status,
            }))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                _ => Err(format_err!(
                    "Failed to review friend req, req_id: {req_id}, status: {status}",
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to review friend req, req_id: {req_id}, status: {status}, err: {err}"
            )),
        }
    })?
}
