use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Client, Proxy};

use super::identity::Identity;

#[derive(Clone, Debug)]
pub struct ProxyDef {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub pass: Option<String>,
}

impl ProxyDef {
    pub fn url(&self) -> String {
        match (&self.user, &self.pass) {
            (Some(u), Some(p)) => format!("{}://{}:{}@{}:{}", self.scheme, u, p, self.host, self.port),
            _ => format!("{}://{}:{}", self.scheme, self.host, self.port),
        }
    }
}

pub fn build_client(proxy: Option<&ProxyDef>, identity: &Identity) -> reqwest::Result<Client> {
    let mut b = Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(8))
        .pool_idle_timeout(Duration::from_secs(30))
        .user_agent(&identity.browser_user_agent)
        .default_headers(base_headers(identity))
        .tcp_nodelay(true)
        .gzip(true)
        .brotli(true)
        .cookie_store(true);

    if let Some(p) = proxy {
        b = b.proxy(Proxy::all(p.url())?);
    }

    b.build()
}

fn base_headers(id: &Identity) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        HeaderName::from_static("accept-language"),
        HeaderValue::from_static("en-US,en;q=0.9"),
    );
    h.insert(
        HeaderName::from_static("origin"),
        HeaderValue::from_static("https://discord.com"),
    );
    h.insert(
        HeaderName::from_static("referer"),
        HeaderValue::from_static("https://discord.com/channels/@me"),
    );
    h.insert(
        HeaderName::from_static("x-discord-locale"),
        HeaderValue::from_static("en-US"),
    );
    h.insert(
        HeaderName::from_static("x-discord-timezone"),
        HeaderValue::from_static("Etc/GMT"),
    );
    h.insert(
        HeaderName::from_static("sec-fetch-dest"),
        HeaderValue::from_static("empty"),
    );
    h.insert(
        HeaderName::from_static("sec-fetch-mode"),
        HeaderValue::from_static("cors"),
    );
    h.insert(
        HeaderName::from_static("sec-fetch-site"),
        HeaderValue::from_static("same-origin"),
    );
    if let Ok(v) = HeaderValue::from_str(&id.super_properties_b64()) {
        h.insert(HeaderName::from_static("x-super-properties"), v);
    }
    let ua_full = format!("\"Chromium\";v=\"{0}\", \"Google Chrome\";v=\"{0}\", \"Not=A?Brand\";v=\"99\"", short_ver(&id.browser_version));
    if let Ok(v) = HeaderValue::from_str(&ua_full) {
        h.insert(HeaderName::from_static("sec-ch-ua"), v);
    }
    h.insert(
        HeaderName::from_static("sec-ch-ua-mobile"),
        HeaderValue::from_static("?0"),
    );
    let plat = match id.os.as_str() {
        "Windows" => "\"Windows\"",
        "Mac OS X" => "\"macOS\"",
        _ => "\"Linux\"",
    };
    if let Ok(v) = HeaderValue::from_str(plat) {
        h.insert(HeaderName::from_static("sec-ch-ua-platform"), v);
    }
    h
}

fn short_ver(v: &str) -> &str {
    match v.find('.') {
        Some(i) => &v[..i],
        None => v,
    }
}
