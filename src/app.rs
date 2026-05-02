use crate::api::{Api, Listen, TopArtist, TopRecording, TopRelease};
use crate::config::{Config, Range};
use anyhow::Result;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Recent,
    Artists,
    Tracks,
    Releases,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Recent, Tab::Artists, Tab::Tracks, Tab::Releases];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Recent => "Recent",
            Tab::Artists => "Top Artists",
            Tab::Tracks => "Top Tracks",
            Tab::Releases => "Top Releases",
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn prev(self) -> Self {
        let i = Self::ALL.iter().position(|t| *t == self).unwrap_or(0);
        Self::ALL[(i + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

pub struct App {
    pub cfg: Config,
    pub api: Api,
    pub tab: Tab,
    pub range: Range,
    pub selected: usize,
    pub now_playing: Option<Listen>,
    pub recent: Vec<Listen>,
    pub artists: Vec<TopArtist>,
    pub tracks: Vec<TopRecording>,
    pub releases: Vec<TopRelease>,
    pub total: Option<u64>,
    pub last_refresh: Instant,
    pub status: String,
    pub should_quit: bool,
}

impl App {
    pub fn new(cfg: Config) -> Result<Self> {
        let api = Api::new(cfg.token.clone())?;
        let range = cfg.range;
        Ok(Self {
            cfg,
            api,
            tab: Tab::Recent,
            range,
            selected: 0,
            now_playing: None,
            recent: Vec::new(),
            artists: Vec::new(),
            tracks: Vec::new(),
            releases: Vec::new(),
            total: None,
            last_refresh: Instant::now(),
            status: String::from("loading…"),
            should_quit: false,
        })
    }

    pub fn refresh(&mut self) {
        self.status = format!("refreshing ({})…", self.range.as_str());
        let user = &self.cfg.username.clone();
        let range = self.range.as_str();

        match self.api.playing_now(user) {
            Ok(np) => self.now_playing = np,
            Err(e) => self.status = format!("playing-now: {e}"),
        }
        match self.api.recent_listens(user, 50) {
            Ok(v) => self.recent = v,
            Err(e) => self.status = format!("listens: {e}"),
        }
        match self.api.top_artists(user, range, 25) {
            Ok(v) => self.artists = v,
            Err(e) => self.status = format!("artists: {e}"),
        }
        match self.api.top_recordings(user, range, 25) {
            Ok(v) => self.tracks = v,
            Err(e) => self.status = format!("tracks: {e}"),
        }
        match self.api.top_releases(user, range, 25) {
            Ok(v) => self.releases = v,
            Err(e) => self.status = format!("releases: {e}"),
        }
        match self.api.total_listens(user) {
            Ok(n) => self.total = Some(n),
            Err(_) => {}
        }
        self.last_refresh = Instant::now();
        if !self.status.contains(':') {
            self.status = format!("ok ({})", self.range.as_str());
        }
    }

    pub fn list_len(&self) -> usize {
        match self.tab {
            Tab::Recent => self.recent.len(),
            Tab::Artists => self.artists.len(),
            Tab::Tracks => self.tracks.len(),
            Tab::Releases => self.releases.len(),
        }
    }

    pub fn move_down(&mut self) {
        let n = self.list_len();
        if n == 0 {
            return;
        }
        self.selected = (self.selected + 1).min(n - 1);
    }

    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn jump_top(&mut self) {
        self.selected = 0;
    }

    pub fn jump_bottom(&mut self) {
        let n = self.list_len();
        self.selected = n.saturating_sub(1);
    }

    pub fn next_tab(&mut self) {
        self.tab = self.tab.next();
        self.selected = 0;
    }

    pub fn prev_tab(&mut self) {
        self.tab = self.tab.prev();
        self.selected = 0;
    }

    pub fn cycle_range(&mut self) {
        self.range = self.range.cycle();
        self.refresh();
    }

    pub fn open_selected(&self) {
        let url = match self.tab {
            Tab::Recent => self.recent.get(self.selected).and_then(recording_url_from_listen),
            Tab::Tracks => self.tracks.get(self.selected).and_then(|r| {
                r.recording_mbid.as_deref().map(|m| format!("https://musicbrainz.org/recording/{m}"))
            }),
            Tab::Artists => self.artists.get(self.selected).and_then(|a| {
                a.artist_mbid.as_deref().map(|m| format!("https://musicbrainz.org/artist/{m}"))
            }),
            Tab::Releases => self.releases.get(self.selected).and_then(|r| {
                r.release_mbid.as_deref().map(|m| format!("https://musicbrainz.org/release/{m}"))
            }),
        };
        if let Some(url) = url {
            let _ = open::that_detached(url);
        }
    }
}

fn recording_url_from_listen(l: &Listen) -> Option<String> {
    let mbid = l
        .track_metadata
        .mbid_mapping
        .as_ref()
        .and_then(|m| m.recording_mbid.clone())
        .or_else(|| l.track_metadata.additional_info.as_ref().and_then(|a| a.recording_mbid.clone()))?;
    Some(format!("https://musicbrainz.org/recording/{mbid}"))
}
