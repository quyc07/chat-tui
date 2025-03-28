use chrono::{DateTime, Local};
use jsonwebtoken::{DecodingKey, EncodingKey, TokenData, Validation, decode};
use serde::{Deserialize, Serialize};
use std::ops::Add;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

// 存储当前用户信息
pub(crate) static CURRENT_USER: LazyLock<CurrentUserLock> = LazyLock::new(|| {
    CurrentUserLock(Arc::new(Mutex::new(CurrentUserHolder {
        user: CurrentUser {
            user: None,
            token: None,
        },
    })))
});

pub(crate) struct CurrentUserLock(Arc<Mutex<CurrentUserHolder>>);

impl CurrentUserLock {
    pub(crate) fn get_user(&self) -> CurrentUser {
        self.0.lock().unwrap().user.clone()
    }

    pub(crate) fn set_user(&self, user: Option<User>, token: Option<String>) {
        let mut user_guard = self.0.lock().unwrap();
        user_guard.user = CurrentUser { user, token }
    }
}

pub(crate) struct CurrentUserHolder {
    pub(crate) user: CurrentUser,
}

#[derive(Clone)]
pub(crate) struct CurrentUser {
    pub(crate) user: Option<User>,
    pub(crate) token: Option<String>,
}

static KEYS: LazyLock<Keys, fn() -> Keys> = LazyLock::new(|| {
    let secret = std::env::var("JWT_SECRET").unwrap_or("abc".to_string());
    Keys::new(secret.as_bytes())
});

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub role: Role,
    // 失效时间，timestamp
    exp: i64,
}

impl Default for User {
    fn default() -> Self {
        Self {
            id: 0,
            name: "".to_string(),
            email: None,
            phone: None,
            role: Role::User,
            exp: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    User,
    Admin,
}

const SECOND_TO_EXPIRED: u64 = 60;
fn expire_timestamp() -> i64 {
    Local::now()
        .add(Duration::from_secs(SECOND_TO_EXPIRED))
        .timestamp()
}

fn expire() -> DateTime<Local> {
    Local::now().add(Duration::from_secs(SECOND_TO_EXPIRED))
}

pub(crate) fn parse_token(token: &str) -> Result<TokenData<User>, String> {
    let mut validation = Validation::default();
    // 修改leeway=0，让exp校验使用绝对时间，参考Validation.leeway的使用
    validation.leeway = 0;
    decode(token, &KEYS.decoding, &validation).map_err(|_| "token invalid".to_string())
}

struct Keys {
    pub(crate) encoding: EncodingKey,
    pub(crate) decoding: DecodingKey,
}

impl Keys {
    fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}
