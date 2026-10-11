use std::path::PathBuf;

pub const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
pub const CALLBACK_PORT: u16 = 8888;
pub const SEARCH_DEBOUNCE_MS: u64 = 400;

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("spotify-playlist-cli")
}

pub fn token_path() -> PathBuf {
    config_dir().join("token.json")
}
