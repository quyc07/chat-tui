use crate::proxy::send_request;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use color_eyre::eyre::format_err;
use reqwest::blocking::Client;
use reqwest::StatusCode;
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
            .get(format!("{}/group/{gid}", HOST.as_str()))
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
            .put(format!("{}/group/{gid}/{uid}", HOST.as_str()))
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

pub(crate) fn evict(gid: i32, uid: i32) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .delete(format!("{}/group/{gid}/{uid}", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                _ => Err(format_err!(
                    "Failed to evict group member, gid: {gid}, uid: {uid}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to evict group member, gid: {gid}, uid: {uid}, err: {err}",
            )),
        }
    })?
}

pub(crate) fn forbid(gid: i32, uid: i32) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .put(format!("{}/group/{gid}/forbid/{uid}", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                _ => Err(format_err!(
                    "Failed to forbid group member, gid: {gid}, uid: {uid}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to forbid group member, gid: {gid}, uid: {uid}, err: {err}",
            )),
        }
    })?
}

pub(crate) fn un_forbid(gid: i32, uid: i32) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .delete(format!("{}/group/{gid}/forbid/{uid}", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                _ => Err(format_err!(
                    "Failed to un forbid group member, gid: {gid}, uid: {uid}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to un forbid group member, gid: {gid}, uid: {uid}, err: {err}",
            )),
        }
    })?
}

pub(crate) fn set_manager(gid: i32, uid: i32) -> color_eyre::Result<()> {
    send_request(move || {
        let current_user = CURRENT_USER.get_user();
        let token = current_user.token.clone().unwrap();
        let res = Client::new()
            .patch(format!("{}/group/{gid}/admin/{uid}", HOST.as_str()))
            .header("Authorization", format!("Bearer {token}"))
            .send();
        match res {
            Ok(res) => match res.status() {
                StatusCode::OK => Ok(()),
                _ => Err(format_err!(
                    "Failed to set manager group member, gid: {gid}, uid: {uid}, status: {}",
                    res.status()
                )),
            },
            Err(err) => Err(format_err!(
                "Failed to set manager group member, gid: {gid}, uid: {uid}, err: {err}",
            )),
        }
    })?
}
