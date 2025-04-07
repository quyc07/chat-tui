use crate::proxy::HOST;
use crate::proxy::send_request;
use crate::token::CURRENT_USER;
use color_eyre::eyre::format_err;
use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct DetailRes {
    pub(crate) group_id: i32,
    pub(crate) name: String,
    pub(crate) users: Vec<GroupUser>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct GroupUser {
    pub(crate) id: i32,
    pub(crate) name: String,
    pub(crate) admin: bool,
    pub(crate) forbid: bool,
}

pub(crate) fn detail(gid: i32) -> color_eyre::Result<DetailRes> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .get(format!("{HOST}/group/{gid}"))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => {
                    let res = res.json::<DetailRes>();
                    res.map_err(|err| {
                        format_err!("Failed to get group detail, gid: {gid}, err: {err}")
                    })
                }
                _ => Err(format_err!(
                    "Failed to get group detail, gid: {gid}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to get group detail, gid: {gid}, err: {err}"
            )),
        }
    })?
}

pub(crate) fn invite(uid: i32, gid: i32) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .put(format!("{HOST}/group/{gid}/{uid}"))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                _ => Err(format_err!(
                    "Failed to invite group member, gid: {gid}, uid: {uid}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to invite group member, gid: {gid}, uid: {uid}, err: {err}",
            )),
        }
    })?
}
