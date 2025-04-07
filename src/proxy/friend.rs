use crate::proxy::HOST;
use crate::proxy::send_request;
use crate::token::CURRENT_USER;
use color_eyre::eyre::format_err;
use ratatui::prelude::{Line, Span, Style, Text};
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
