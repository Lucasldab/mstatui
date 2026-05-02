use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::time::Duration;

const UA: &str = concat!("mstatui/", env!("CARGO_PKG_VERSION"), " (https://github.com/Lucasldab/mstatui)");
const BASE: &str = "https://api.listenbrainz.org/1";

pub struct Api {
    client: Client,
    token: Option<String>,
}

impl Api {
    pub fn new(token: Option<String>) -> Result<Self> {
        let client = Client::builder()
            .user_agent(UA)
            .timeout(Duration::from_secs(10))
            .build()?;
        Ok(Self { client, token })
    }

    fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T> {
        let url = format!("{BASE}{path}");
        let mut req = self.client.get(&url);
        if let Some(t) = &self.token {
            req = req.header("Authorization", format!("Token {t}"));
        }
        let res = req.send().with_context(|| format!("GET {url}"))?;
        let status = res.status();
        if !status.is_success() {
            let body = res.text().unwrap_or_default();
            anyhow::bail!("{url} → {status}: {body}");
        }
        res.json::<T>().with_context(|| format!("decode {url}"))
    }

    pub fn recent_listens(&self, user: &str, count: u32) -> Result<Vec<Listen>> {
        let resp: ListensResp = self.get(&format!("/user/{user}/listens?count={count}"))?;
        Ok(resp.payload.listens)
    }

    pub fn playing_now(&self, user: &str) -> Result<Option<Listen>> {
        let resp: ListensResp = self.get(&format!("/user/{user}/playing-now"))?;
        Ok(resp.payload.listens.into_iter().next())
    }

    pub fn top_artists(&self, user: &str, range: &str, count: u32) -> Result<Vec<TopArtist>> {
        let resp: ArtistsResp = self
            .get(&format!("/stats/user/{user}/artists?range={range}&count={count}"))
            .unwrap_or_else(|_| ArtistsResp::default());
        Ok(resp.payload.artists)
    }

    pub fn top_recordings(&self, user: &str, range: &str, count: u32) -> Result<Vec<TopRecording>> {
        let resp: RecordingsResp = self
            .get(&format!("/stats/user/{user}/recordings?range={range}&count={count}"))
            .unwrap_or_else(|_| RecordingsResp::default());
        Ok(resp.payload.recordings)
    }

    pub fn top_releases(&self, user: &str, range: &str, count: u32) -> Result<Vec<TopRelease>> {
        let resp: ReleasesResp = self
            .get(&format!("/stats/user/{user}/releases?range={range}&count={count}"))
            .unwrap_or_else(|_| ReleasesResp::default());
        Ok(resp.payload.releases)
    }

    pub fn total_listens(&self, user: &str) -> Result<u64> {
        let resp: ListenCountResp = self.get(&format!("/user/{user}/listen-count"))?;
        Ok(resp.payload.count)
    }
}

#[derive(Debug, Deserialize)]
struct ListensResp {
    payload: ListensPayload,
}

#[derive(Debug, Deserialize)]
struct ListensPayload {
    #[serde(default)]
    listens: Vec<Listen>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Listen {
    #[serde(default)]
    pub listened_at: i64,
    pub track_metadata: TrackMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TrackMetadata {
    pub artist_name: String,
    pub track_name: String,
    pub release_name: Option<String>,
    pub additional_info: Option<AdditionalInfo>,
    pub mbid_mapping: Option<MbidMapping>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdditionalInfo {
    pub recording_mbid: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MbidMapping {
    pub recording_mbid: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ArtistsResp {
    payload: ArtistsPayload,
}

#[derive(Debug, Default, Deserialize)]
struct ArtistsPayload {
    #[serde(default)]
    artists: Vec<TopArtist>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TopArtist {
    pub artist_name: String,
    pub listen_count: u64,
    #[serde(default)]
    pub artist_mbid: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RecordingsResp {
    payload: RecordingsPayload,
}

#[derive(Debug, Default, Deserialize)]
struct RecordingsPayload {
    #[serde(default)]
    recordings: Vec<TopRecording>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TopRecording {
    pub track_name: String,
    pub artist_name: String,
    pub listen_count: u64,
    #[serde(default)]
    pub recording_mbid: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ReleasesResp {
    payload: ReleasesPayload,
}

#[derive(Debug, Default, Deserialize)]
struct ReleasesPayload {
    #[serde(default)]
    releases: Vec<TopRelease>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TopRelease {
    pub release_name: String,
    pub artist_name: String,
    pub listen_count: u64,
    #[serde(default)]
    pub release_mbid: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListenCountResp {
    payload: ListenCountPayload,
}

#[derive(Debug, Deserialize)]
struct ListenCountPayload {
    count: u64,
}
