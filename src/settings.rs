//! Load/save user settings (colours, style) from config file.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::theme::{AccentColor, BorderColor, StatusColor, TextColor, TextStyle};

fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("ratlog").join("settings.json"))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SavedSettings {
    pub accent: String,
    pub text_color: String,
    pub text_style: String,
    pub border_color: String,
    pub status_color: String,
}

pub fn load_settings() -> (AccentColor, TextColor, TextStyle, BorderColor, StatusColor) {
    let path = match settings_path() {
        Some(p) => p,
        None => {
            return (
                AccentColor::default(),
                TextColor::default(),
                TextStyle::default(),
                BorderColor::default(),
                StatusColor::default(),
            );
        }
    };
    let s = match fs::read_to_string(&path) {
        Ok(x) => x,
        Err(_) => {
            return (
                AccentColor::default(),
                TextColor::default(),
                TextStyle::default(),
                BorderColor::default(),
                StatusColor::default(),
            );
        }
    };
    let saved: SavedSettings = match serde_json::from_str(&s) {
        Ok(x) => x,
        Err(_) => {
            return (
                AccentColor::default(),
                TextColor::default(),
                TextStyle::default(),
                BorderColor::default(),
                StatusColor::default(),
            );
        }
    };
    let parse_accent = |v: &str| {
        AccentColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_text_color = |v: &str| {
        TextColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_text_style = |v: &str| {
        TextStyle::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_border = |v: &str| {
        BorderColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    let parse_status = |v: &str| {
        StatusColor::all()
            .iter()
            .find(|c| c.name().eq_ignore_ascii_case(v))
            .copied()
            .unwrap_or_default()
    };
    (
        parse_accent(&saved.accent),
        parse_text_color(&saved.text_color),
        parse_text_style(&saved.text_style),
        parse_border(&saved.border_color),
        parse_status(&saved.status_color),
    )
}

pub fn save_settings(
    accent: AccentColor,
    text_color: TextColor,
    text_style: TextStyle,
    border_color: BorderColor,
    status_color: StatusColor,
) {
    let path = match settings_path() {
        Some(p) => p,
        None => return,
    };
    let saved = SavedSettings {
        accent: accent.name().to_string(),
        text_color: text_color.name().to_string(),
        text_style: text_style.name().to_string(),
        border_color: border_color.name().to_string(),
        status_color: status_color.name().to_string(),
    };
    let s = match serde_json::to_string_pretty(&saved) {
        Ok(x) => x,
        Err(_) => return,
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .and_then(|mut f| f.write_all(s.as_bytes()));
}
