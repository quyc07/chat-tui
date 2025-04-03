use crate::datetime::datetime_format;
use crate::datetime::opt_datetime_format;
use crate::proxy::{send_request, HOST};
use crate::token::CURRENT_USER;
use chrono::{DateTime, Local};
use color_eyre::eyre::format_err;
use reqwest::blocking::Client;
use reqwest::StatusCode;
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
                    res.or_else(|e| Err(format_err!("Failed to get detail by uid {uid} :{}", e)))
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
