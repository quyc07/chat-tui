use crate::proxy::send_request;
use crate::proxy::HOST;
use crate::token::CURRENT_USER;
use color_eyre::eyre::format_err;
use ratatui::prelude::{Color, Line, Span, Style, Text};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct Friend {
    id: i32,
    name: String,
}

impl From<&Friend> for Text<'_> {
    fn from(friend: &Friend) -> Self {
        Line::from(Span::styled(
            format!("好友: {}\n", friend.name),
            Style::default().fg(Color::White),
        ))
        .into()
    }
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
                    res.or_else(|err| {
                        Err(format_err!(
                            "Failed to get friends, id: {}, err: {}",
                            current_user.user.unwrap().id,
                            err
                        ))
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
