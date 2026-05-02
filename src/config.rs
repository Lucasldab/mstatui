use anyhow::{anyhow, Context, Result};
use ratatui::style::Color;
use serde::Deserialize;
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Range {
    Week,
    Month,
    Year,
    All,
}

impl Range {
    pub fn as_str(self) -> &'static str {
        match self {
            Range::Week => "week",
            Range::Month => "month",
            Range::Year => "year",
            Range::All => "all_time",
        }
    }

    pub fn cycle(self) -> Self {
        match self {
            Range::Week => Range::Month,
            Range::Month => Range::Year,
            Range::Year => Range::All,
            Range::All => Range::Week,
        }
    }
}

#[derive(Debug, Deserialize, Default)]
struct RawConfig {
    username: Option<String>,
    token: Option<String>,
    range: Option<Range>,
    refresh_interval: Option<u64>,
    keys: Option<HashMap<String, String>>,
    colors: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub username: String,
    pub token: Option<String>,
    pub range: Range,
    pub refresh_interval: u64,
    pub keys: KeyMap,
    pub colors: Palette,
}

#[derive(Debug, Clone)]
pub struct KeyMap {
    pub quit: char,
    pub next_tab: char,
    pub prev_tab: char,
    pub down: char,
    pub up: char,
    pub top: char,
    pub bottom: char,
    pub refresh: char,
    pub toggle_range: char,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            quit: 'q',
            next_tab: 'l',
            prev_tab: 'h',
            down: 'j',
            up: 'k',
            top: 'g',
            bottom: 'G',
            refresh: 'r',
            toggle_range: 't',
        }
    }
}

#[derive(Debug, Clone)]
pub struct Palette {
    pub accent: Color,
    pub accent_deep: Color,
    pub muted: Color,
    pub positive: Color,
    pub warning: Color,
    pub error: Color,
    pub fg: Color,
    pub bg: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            accent: hex("#9A6DD7").unwrap(),
            accent_deep: hex("#5A2A82").unwrap(),
            muted: hex("#6A6A6A").unwrap(),
            positive: hex("#9A6DD7").unwrap(),
            warning: hex("#B83D8E").unwrap(),
            error: hex("#FF6B6B").unwrap(),
            fg: hex("#E5E5E5").unwrap(),
            bg: hex("#0D0D0D").unwrap(),
        }
    }
}

fn hex(s: &str) -> Result<Color> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return Err(anyhow!("color {s:?} must be 6 hex digits"));
    }
    let r = u8::from_str_radix(&s[0..2], 16)?;
    let g = u8::from_str_radix(&s[2..4], 16)?;
    let b = u8::from_str_radix(&s[4..6], 16)?;
    Ok(Color::Rgb(r, g, b))
}

fn parse_key(s: &str) -> Result<char> {
    let mut chars = s.chars();
    let c = chars.next().ok_or_else(|| anyhow!("empty key binding"))?;
    if chars.next().is_some() {
        return Err(anyhow!("key binding {s:?} must be a single character"));
    }
    Ok(c)
}

pub fn load() -> Result<Config> {
    let raw = read_raw_config()?;

    let mut keys = KeyMap::default();
    if let Some(map) = &raw.keys {
        for (k, v) in map {
            let c = parse_key(v).with_context(|| format!("key binding for {k:?}"))?;
            match k.as_str() {
                "quit" => keys.quit = c,
                "next_tab" => keys.next_tab = c,
                "prev_tab" => keys.prev_tab = c,
                "down" => keys.down = c,
                "up" => keys.up = c,
                "top" => keys.top = c,
                "bottom" => keys.bottom = c,
                "refresh" => keys.refresh = c,
                "toggle_range" => keys.toggle_range = c,
                other => return Err(anyhow!("unknown key action {other:?}")),
            }
        }
    }

    let mut colors = Palette::default();
    if let Some(map) = &raw.colors {
        for (k, v) in map {
            let c = hex(v).with_context(|| format!("color {k:?}"))?;
            match k.as_str() {
                "accent" => colors.accent = c,
                "accent_deep" => colors.accent_deep = c,
                "muted" => colors.muted = c,
                "positive" => colors.positive = c,
                "warning" => colors.warning = c,
                "error" => colors.error = c,
                "fg" => colors.fg = c,
                "bg" => colors.bg = c,
                other => return Err(anyhow!("unknown color slot {other:?}")),
            }
        }
    }

    let token = raw.token.or_else(|| std::env::var("MSTATUI_TOKEN").ok()).or_else(read_credentials_token);
    let username = raw
        .username
        .or_else(|| std::env::var("MSTATUI_USERNAME").ok())
        .or_else(read_credentials_username)
        .ok_or_else(|| anyhow!("no username in config, env MSTATUI_USERNAME, or mpris-scrobbler credentials"))?;

    Ok(Config {
        username,
        token,
        range: raw.range.unwrap_or(Range::Week),
        refresh_interval: raw.refresh_interval.unwrap_or(0),
        keys,
        colors,
    })
}

fn read_raw_config() -> Result<RawConfig> {
    let path = config_path();
    if !path.exists() {
        return Ok(RawConfig::default());
    }
    let body = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&body).with_context(|| format!("parse {}", path.display()))
}

pub fn config_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("~/.config")).join("mstatui").join("config.toml")
}

fn credentials_path() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join("mpris-scrobbler").join("credentials"))
}

fn read_credentials_field(field: &str) -> Option<String> {
    let body = fs::read_to_string(credentials_path()?).ok()?;
    let mut in_lb = false;
    for line in body.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_lb = line == "[listenbrainz]";
            continue;
        }
        if !in_lb {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == field {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn read_credentials_token() -> Option<String> {
    read_credentials_field("token")
}

fn read_credentials_username() -> Option<String> {
    read_credentials_field("username")
}
