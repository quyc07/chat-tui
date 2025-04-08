use crate::proxy::HOST;
use crate::proxy::send_request;
use crate::token::CURRENT_USER;
use color_eyre::eyre::format_err;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

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
            .get(format!("{HOST}/friend"))
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

pub(crate) fn add_friend(uid: i32, friend_uid: i32) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .post(format!("{HOST}/friend/req/{friend_uid}"))
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({}))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                StatusCode::NOT_MODIFIED => {
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
