//! Settings and saved session. The file location and keys match the earlier
//! versions (GTK: ~/.config/tormin.json, WinUI: %APPDATA%\Tormin\config.json),
//! so switching to this app keeps the settings and the torrent list.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct SessionTorrent {
    pub source: Option<String>,
    pub is_magnet: bool,
    pub paused: bool,
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_priorities: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_dir: Option<String>,
    pub language: String,
    pub theme: String, // system | light | dark
    pub accent: String, // purple | green | orange | blue | #rrggbb
    pub compact_mode: bool,
    pub compact_window_size: bool,
    pub progress_style: u8, // 1 text, 2 circle + text, 3 circle clock
    pub monochrome: bool,
    pub shutdown_complete: bool,
    pub start_with_windows: bool,
    pub start_minimized: bool,
    /// Last window size in logical pixels; the presets apply until the user resizes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_height: Option<f64>,
    pub torrents: Vec<SessionTorrent>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            download_dir: None,
            language: "en".into(),
            theme: "system".into(),
            accent: "purple".into(),
            compact_mode: false,
            compact_window_size: true,
            progress_style: 3,
            monochrome: false,
            shutdown_complete: false,
            start_with_windows: false,
            start_minimized: false,
            window_width: None,
            window_height: None,
            torrents: Vec::new(),
        }
    }
}

pub fn config_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    if cfg!(windows) { base.join("Tormin").join("config.json") } else { base.join("tormin.json") }
}

pub fn data_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join(if cfg!(windows) { "Tormin" } else { "tormin" })
}

pub fn resume_dir() -> PathBuf {
    data_dir().join("resume")
}

impl Config {
    pub fn load() -> Config {
        match fs::read_to_string(config_path()) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                crate::log(&format!("Error loading config: {e}"));
                Config::default()
            }),
            Err(_) => Config::default(),
        }
    }

    pub fn save(&self) {
        let path = config_path();
        let result = (|| -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
            fs::rename(&tmp, &path)
        })();
        if let Err(e) = result {
            crate::log(&format!("Error saving config: {e}"));
        }
    }

    pub fn download_dir(&self) -> String {
        self.download_dir
            .clone()
            .filter(|d| !d.is_empty())
            .or_else(|| dirs::download_dir().map(|d| d.to_string_lossy().into_owned()))
            .unwrap_or_else(|| ".".into())
    }
}
