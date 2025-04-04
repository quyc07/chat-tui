use crate::datetime::datetime_format;
use crate::datetime::opt_datetime_format;
use crate::proxy::{HOST, send_request};
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct UserDetail {
    /// User id
    pub id: i32,
    /// User name
    pub name: String,
    /// User email
    pub email: Option<String>,
    /// User phone
    pub phone: Option<String>,
    /// Create time
    #[serde(with = "datetime_format")]
    pub create_time: DateTime<Local>,
    /// Update time
    #[serde(with = "opt_datetime_format")]
    pub update_time: Option<DateTime<Local>>,
    /// User status
    pub status: String,
    /// Is friend
    pub is_friend: bool,
}

pub(crate) fn detail_by_id(uid: i32) -> color_eyre::Result<UserDetail> {
    send_request(move || {
        let token = CURRENT_USER.get_user().token.clone().unwrap();
        let res = Client::new()
            .get(format!("{HOST}/user/detail/{uid}"))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => {
                    let res = res.json::<UserDetail>();
                    res.map_err(|e| format_err!("Failed to get detail by uid {uid} :{}", e))
                }
                _ => Err(format_err!(
                    "Failed to get detail by uid {uid} :{}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!("Failed to get detail by uid {uid} : {}", err)),
        }
    })?
}

pub(crate) fn search(name: String) -> color_eyre::Result<Vec<UserDetail>> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .get(format!("{HOST}/user/search/{name}"))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => {
                    let res = res.json::<Vec<UserDetail>>();
                    res.map_err(|err| {
                        format_err!("Failed to search user, name: {name}, err: {err}")
                    })
                }
                _ => Err(format_err!(
                    "Failed to search user, name: {name}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to search user, name: {name}, err: {err}"
            )),
        }
    })?
}
