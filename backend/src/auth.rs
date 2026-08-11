use std::{
    collections::HashMap,
    env, io,
    sync::Arc,
    time::{Duration, Instant},
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{AppState, error::AppError};

const COOKIE_NAME: &str = "amstock_session";
const DEFAULT_SESSION_HOURS: u64 = 24 * 30;

#[derive(Clone)]
pub struct AuthState {
    username: Arc<str>,
    password_hash: Arc<str>,
    sessions: Arc<Mutex<HashMap<String, Instant>>>,
    session_ttl: Duration,
    secure_cookie: bool,
}

impl AuthState {
    pub fn from_env() -> io::Result<Self> {
        let username = env::var("AMSTOCK_USERNAME").unwrap_or_else(|_| "admin".into());
        if username.trim().is_empty() {
            return Err(config_error("AMSTOCK_USERNAME 不能为空"));
        }

        let password_hash = match env::var("AMSTOCK_PASSWORD_HASH") {
            Ok(value) if !value.trim().is_empty() => {
                PasswordHash::new(&value).map_err(|error| {
                    config_error(format!(
                        "AMSTOCK_PASSWORD_HASH 不是有效的 PHC 密码哈希：{error}"
                    ))
                })?;
                value
            }
            _ => {
                let password = env::var("AMSTOCK_PASSWORD").map_err(|_| {
                    config_error("必须设置 AMSTOCK_PASSWORD 或 AMSTOCK_PASSWORD_HASH")
                })?;
                if password.len() < 8 {
                    return Err(config_error("AMSTOCK_PASSWORD 至少需要 8 个字符"));
                }
                hash_password(&password)?
            }
        };

        let session_hours = env::var("AMSTOCK_SESSION_TTL_HOURS")
            .ok()
            .map(|value| value.parse::<u64>())
            .transpose()
            .map_err(|_| config_error("AMSTOCK_SESSION_TTL_HOURS 必须是正整数"))?
            .unwrap_or(DEFAULT_SESSION_HOURS);
        if session_hours == 0 {
            return Err(config_error("AMSTOCK_SESSION_TTL_HOURS 必须大于 0"));
        }

        let secure_cookie = env::var("AMSTOCK_COOKIE_SECURE")
            .ok()
            .map(|value| parse_bool(&value))
            .transpose()?
            .unwrap_or(true);

        Ok(Self {
            username: username.into(),
            password_hash: password_hash.into(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            session_ttl: Duration::from_secs(session_hours.saturating_mul(3600)),
            secure_cookie,
        })
    }

    async fn authenticate(&self, headers: &HeaderMap) -> bool {
        let Some(token) = session_token(headers) else {
            return false;
        };
        let now = Instant::now();
        let mut sessions = self.sessions.lock().await;
        sessions.retain(|_, expires_at| *expires_at > now);
        sessions
            .get(token)
            .is_some_and(|expires_at| *expires_at > now)
    }

    async fn create_session(&self) -> Result<String, AppError> {
        let mut bytes = [0_u8; 32];
        getrandom::fill(&mut bytes)
            .map_err(|error| AppError::Internal(format!("无法生成登录会话：{error}")))?;
        let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        self.sessions
            .lock()
            .await
            .insert(token.clone(), Instant::now() + self.session_ttl);
        Ok(token)
    }

    async fn remove_session(&self, headers: &HeaderMap) {
        if let Some(token) = session_token(headers) {
            self.sessions.lock().await.remove(token);
        }
    }

    fn session_cookie(&self, token: &str) -> String {
        let mut cookie = format!(
            "{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
            self.session_ttl.as_secs()
        );
        if self.secure_cookie {
            cookie.push_str("; Secure");
        }
        cookie
    }

    fn expired_cookie(&self) -> String {
        let mut cookie = format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0");
        if self.secure_cookie {
            cookie.push_str("; Secure");
        }
        cookie
    }
}

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    username: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let password_valid = PasswordHash::new(&state.auth.password_hash)
        .ok()
        .and_then(|hash| {
            Argon2::default()
                .verify_password(request.password.as_bytes(), &hash)
                .ok()
        })
        .is_some();
    if request.username != state.auth.username.as_ref() || !password_valid {
        // A small fixed delay is enough for this single-user service and avoids
        // adding a separate rate-limiter or cache service.
        tokio::time::sleep(Duration::from_millis(500)).await;
        return Err(AppError::Unauthorized("用户名或密码错误".into()));
    }

    let token = state.auth.create_session().await?;
    let mut response = Json(SessionResponse {
        username: state.auth.username.to_string(),
    })
    .into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&state.auth.session_cookie(&token))
            .map_err(|_| AppError::Internal("无法生成登录 Cookie".into()))?,
    );
    Ok(response)
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    state.auth.remove_session(&headers).await;
    let mut response = StatusCode::NO_CONTENT.into_response();
    if let Ok(value) = HeaderValue::from_str(&state.auth.expired_cookie()) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

pub async fn session(State(state): State<AppState>) -> Json<SessionResponse> {
    Json(SessionResponse {
        username: state.auth.username.to_string(),
    })
}

pub async fn require_auth(State(state): State<AppState>, request: Request, next: Next) -> Response {
    if state.auth.authenticate(request.headers()).await {
        next.run(request).await
    } else {
        AppError::Unauthorized("登录已失效，请重新登录".into()).into_response()
    }
}

fn session_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&format!("{COOKIE_NAME}=")))
        .filter(|token| token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn hash_password(password: &str) -> io::Result<String> {
    let mut salt_bytes = [0_u8; 16];
    getrandom::fill(&mut salt_bytes)
        .map_err(|error| config_error(format!("无法生成密码盐：{error}")))?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|error| config_error(format!("无法编码密码盐：{error}")))?;
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| config_error(format!("无法计算密码哈希：{error}")))?
        .to_string())
}

fn parse_bool(value: &str) -> io::Result<bool> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(config_error(format!("无效的布尔值：{value}"))),
    }
}

fn config_error(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_valid_session_cookie_from_multiple_cookies() {
        let token = "a".repeat(64);
        let mut headers = HeaderMap::new();
        headers.insert(
            header::COOKIE,
            format!("theme=light; {COOKIE_NAME}={token}; other=1")
                .parse()
                .unwrap(),
        );
        assert_eq!(session_token(&headers), Some(token.as_str()));
    }

    #[test]
    fn hashes_and_verifies_password() {
        let hash = hash_password("correct horse battery staple").unwrap();
        let parsed = PasswordHash::new(&hash).unwrap();
        assert!(
            Argon2::default()
                .verify_password(b"correct horse battery staple", &parsed)
                .is_ok()
        );
    }
}
