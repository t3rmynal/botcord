use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use super::http_client::{build_client, ProxyDef};
use super::identity::Identity;

pub const API: &str = "https://discord.com/api/v10";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeUser {
    pub id: String,
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub flags: Option<u64>,
    #[serde(default)]
    pub public_flags: Option<u64>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub verified: Option<bool>,
    #[serde(default)]
    pub mfa_enabled: Option<bool>,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub premium_type: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PartialGuild {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub owner: Option<bool>,
    #[serde(default)]
    pub permissions: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: u32,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DmRecipient {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub global_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateChannel {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: u32,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub recipients: Vec<DmRecipient>,
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limited, retry after {0}s")]
    RateLimited(f32),
    #[error("network: {0}")]
    Net(String),
    #[error("status {0}: {1}")]
    Status(u16, String),
}

pub async fn fetch_me(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
) -> Result<MeUser, ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let r = c
        .get(format!("{API}/users/@me"))
        .header("Authorization", token)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;

    let st = r.status();
    if st == StatusCode::UNAUTHORIZED {
        return Err(ApiError::Unauthorized);
    }
    if st == StatusCode::TOO_MANY_REQUESTS {
        let ra = retry_after(&r).unwrap_or(5.0);
        return Err(ApiError::RateLimited(ra));
    }
    if !st.is_success() {
        let body = r.text().await.unwrap_or_default();
        return Err(ApiError::Status(st.as_u16(), body));
    }
    r.json::<MeUser>()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))
}

pub async fn fetch_guilds(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
) -> Result<Vec<PartialGuild>, ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let r = c
        .get(format!("{API}/users/@me/guilds"))
        .header("Authorization", token)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    handle_json(r).await
}

pub async fn patch_self(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
    body: serde_json::Value,
) -> Result<(), ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let r = c
        .patch(format!("{API}/users/@me"))
        .header("Authorization", token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    handle_empty(r).await
}

pub async fn patch_guild_nick(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
    guild_id: &str,
    nick: &str,
) -> Result<(), ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let body = serde_json::json!({ "nick": nick });
    let r = c
        .patch(format!("{API}/guilds/{guild_id}/members/@me"))
        .header("Authorization", token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    handle_empty(r).await
}

pub async fn fetch_channels(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
    guild_id: &str,
) -> Result<Vec<Channel>, ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let r = c
        .get(format!("{API}/guilds/{guild_id}/channels"))
        .header("Authorization", token)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    handle_json(r).await
}

pub async fn fetch_dms(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
) -> Result<Vec<PrivateChannel>, ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let r = c
        .get(format!("{API}/users/@me/channels"))
        .header("Authorization", token)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    handle_json(r).await
}

pub async fn send_dm_text(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
    channel_id: &str,
    content: &str,
) -> Result<(), ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let body = serde_json::json!({ "content": content });
    let r = c
        .post(format!("{API}/channels/{channel_id}/messages"))
        .header("Authorization", token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    handle_empty(r).await
}

pub async fn send_dm_attachment(
    token: &str,
    identity: &Identity,
    proxy: Option<&ProxyDef>,
    channel_id: &str,
    content: &str,
    file_name: &str,
    file_bytes: &[u8],
    mime: &str,
) -> Result<(), ApiError> {
    let c = build_client(proxy, identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let payload_json = serde_json::json!({
        "content": content,
        "attachments": [{ "id": 0, "filename": file_name }],
    })
    .to_string();

    let file_part = reqwest::multipart::Part::bytes(file_bytes.to_vec())
        .file_name(file_name.to_string())
        .mime_str(mime)
        .map_err(|e| ApiError::Net(e.to_string()))?;

    let form = reqwest::multipart::Form::new()
        .text("payload_json", payload_json)
        .part("files[0]", file_part);

    let r = c
        .post(format!("{API}/channels/{channel_id}/messages"))
        .header("Authorization", token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    handle_empty(r).await
}

async fn handle_empty(r: reqwest::Response) -> Result<(), ApiError> {
    let st = r.status();
    if st == StatusCode::UNAUTHORIZED {
        return Err(ApiError::Unauthorized);
    }
    if st == StatusCode::TOO_MANY_REQUESTS {
        let ra = retry_after(&r).unwrap_or(5.0);
        return Err(ApiError::RateLimited(ra));
    }
    if !st.is_success() {
        let body = r.text().await.unwrap_or_default();
        let hint = friendly_hint(&body);
        return Err(ApiError::Status(st.as_u16(), hint.unwrap_or(body)));
    }
    Ok(())
}

fn friendly_hint(body: &str) -> Option<String> {
    let low = body.to_ascii_lowercase();
    if low.contains("password") {
        return Some("discord requires the account password for this change, edit via web".into());
    }
    if low.contains("captcha") {
        return Some("discord wants a captcha, log in via web once and retry".into());
    }
    if low.contains("mfa") || low.contains("two-factor") {
        return Some("discord wants 2fa, edit via web".into());
    }
    None
}

pub async fn ping_via_proxy(proxy: &ProxyDef, identity: &Identity) -> Result<u64, ApiError> {
    let c = build_client(Some(proxy), identity).map_err(|e| ApiError::Net(e.to_string()))?;
    let t = std::time::Instant::now();
    let r = c
        .get(format!("{API}/gateway"))
        .send()
        .await
        .map_err(|e| ApiError::Net(e.to_string()))?;
    if !r.status().is_success() {
        return Err(ApiError::Status(r.status().as_u16(), String::new()));
    }
    Ok(t.elapsed().as_millis() as u64)
}

async fn handle_json<T: for<'de> Deserialize<'de>>(r: reqwest::Response) -> Result<T, ApiError> {
    let st = r.status();
    if st == StatusCode::UNAUTHORIZED {
        return Err(ApiError::Unauthorized);
    }
    if st == StatusCode::TOO_MANY_REQUESTS {
        let ra = retry_after(&r).unwrap_or(5.0);
        return Err(ApiError::RateLimited(ra));
    }
    if !st.is_success() {
        let body = r.text().await.unwrap_or_default();
        let hint = friendly_hint(&body);
        return Err(ApiError::Status(st.as_u16(), hint.unwrap_or(body)));
    }
    r.json::<T>().await.map_err(|e| ApiError::Net(e.to_string()))
}

fn retry_after(r: &reqwest::Response) -> Option<f32> {
    r.headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
}
