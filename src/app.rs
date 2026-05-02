use crate::api::{Api, Listen, Snapshot, TopArtist, TopRecording, TopRelease};
use crate::cache;
use crate::config::{Config, Range};
use anyhow::Result;
use std::{
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread,
    time::Instant,
};

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
    pub last_refresh: Option<Instant>,
    pub fetched_at: i64,
    pub status: String,
    pub refreshing: bool,
    pub should_quit: bool,
    tx: Sender<Snapshot>,
    rx: Receiver<Snapshot>,
}

impl App {
    pub fn new(cfg: Config) -> Result<Self> {
        let api = Api::new(cfg.token.clone())?;
        let range = cfg.range;
        let (tx, rx) = mpsc::channel();
        let mut app = Self {
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
            last_refresh: None,
            fetched_at: 0,
            status: String::from("starting…"),
            refreshing: false,
            should_quit: false,
            tx,
            rx,
        };
        if let Some(snap) = cache::load() {
            app.apply(snap);
        }
        // Status stays empty on a healthy boot; refreshing badge in the UI carries the signal.
        app.status.clear();
        Ok(app)
    }

    /// Spawn a non-blocking refresh. Snapshot arrives on the channel; the main loop
    /// drains it each frame.
    pub fn spawn_refresh(&mut self) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        let api = self.api.clone();
        let user = self.cfg.username.clone();
        let range = self.range.as_str().to_string();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let snap = api.fetch_all(&user, &range);
            let _ = tx.send(snap);
        });
    }

    /// Pull any pending snapshots from the worker thread and persist to disk.
    /// Returns true if state changed (caller can repaint).
    pub fn drain(&mut self) -> bool {
        let mut changed = false;
        loop {
            match self.rx.try_recv() {
                Ok(snap) => {
                    let _ = cache::save(&snap);
                    self.apply(snap);
                    self.refreshing = false;
                    changed = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }
        changed
    }

    fn apply(&mut self, snap: Snapshot) {
        self.now_playing = snap.now_playing;
        self.recent = snap.recent;
        self.artists = snap.artists;
        self.tracks = snap.tracks;
        self.releases = snap.releases;
        if let Some(t) = snap.total {
            self.total = Some(t);
        }
        self.fetched_at = snap.fetched_at;
        self.last_refresh = Some(Instant::now());
        self.status = if snap.errors.is_empty() {
            String::new()
        } else {
            snap.errors.join(" · ")
        };
        let max = self.list_len();
        if max > 0 && self.selected >= max {
            self.selected = max - 1;
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
        self.spawn_refresh();
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
