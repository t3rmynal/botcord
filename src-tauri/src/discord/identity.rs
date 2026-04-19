use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rand::seq::SliceRandom;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const CHROME_VERSIONS: &[&str] = &[
    "120.0.6099.129",
    "121.0.6167.184",
    "122.0.6261.112",
    "123.0.6312.86",
    "124.0.6367.118",
];

pub const CLIENT_BUILD_NUMBER: u32 = 280197;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    pub os: String,
    pub os_version: String,
    pub browser: String,
    pub browser_version: String,
    pub device: String,
    pub system_locale: String,
    pub browser_user_agent: String,
    pub release_channel: String,
    pub client_build_number: u32,
    pub client_event_source: Option<String>,
    pub device_id: String,
}

impl Identity {
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let (os, os_version) = match rng.gen_range(0..3) {
            0 => ("Windows", "10"),
            1 => ("Mac OS X", "10.15.7"),
            _ => ("Linux", ""),
        };
        let ver = CHROME_VERSIONS.choose(&mut rng).copied().unwrap_or("124.0.6367.118");
        let ua = build_ua(os, ver);
        Self {
            os: os.into(),
            os_version: os_version.into(),
            browser: "Chrome".into(),
            browser_version: ver.into(),
            device: String::new(),
            system_locale: "en-US".into(),
            browser_user_agent: ua,
            release_channel: "stable".into(),
            client_build_number: CLIENT_BUILD_NUMBER,
            client_event_source: None,
            device_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn super_properties_b64(&self) -> String {
        let j = serde_json::to_string(self).expect("identity json");
        B64.encode(j)
    }
}

fn build_ua(os: &str, ver: &str) -> String {
    let plat = match os {
        "Windows" => "(Windows NT 10.0; Win64; x64)",
        "Mac OS X" => "(Macintosh; Intel Mac OS X 10_15_7)",
        _ => "(X11; Linux x86_64)",
    };
    format!(
        "Mozilla/5.0 {plat} AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{ver} Safari/537.36"
    )
}
