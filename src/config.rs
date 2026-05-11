use std::net::IpAddr;

use serde::Deserialize;

use crate::utils::session::SESSION_PROFILE_DIR;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_browser_headless")]
    pub browser_headless: bool,

    #[serde(default = "default_browser_driver_url")]
    pub browser_driver_url: String,

    #[serde(default = "default_browser_profile_dir")]
    pub browser_profile_dir: String,

    #[serde(default = "default_area_code")]
    pub area_code: u32,

    #[serde(default = "default_items_per_page")]
    pub items_per_page: u32,

    #[serde(default = "default_page_timeout")]
    pub page_timeout: u64,

    #[serde(default = "default_hh_locale")]
    pub hh_locale: String,

    #[serde(default = "default_server_host")]
    pub server_host: IpAddr,

    #[serde(default = "default_server_port")]
    pub server_port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, envy::Error> {
        dotenvy::dotenv().ok();
        envy::from_env::<Self>()
    }
}

fn default_browser_headless() -> bool {
    false
}

fn default_browser_driver_url() -> String {
    "http://localhost:9515".to_owned()
}

fn default_browser_profile_dir() -> String {
    SESSION_PROFILE_DIR.to_owned()
}

fn default_area_code() -> u32 {
    113
}

fn default_items_per_page() -> u32 {
    20
}

fn default_page_timeout() -> u64 {
    30_000
}

fn default_hh_locale() -> String {
    "EN".to_owned()
}

fn default_server_host() -> IpAddr {
    IpAddr::from([127, 0, 0, 1])
}

fn default_server_port() -> u16 {
    3000
}
