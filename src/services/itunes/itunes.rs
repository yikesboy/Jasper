use crate::services::itunes::error::ITunesAPIError;
use crate::services::itunes::models::{ITunesSearchResponse, TrackInfo};
use reqwest::Client;

#[derive(Clone)]
pub struct ItunesAPI {
    http: Client,
    base_url: String,
}

impl ItunesAPI {
    const BASE_URL: &'static str = "https://itunes.apple.com";
    pub fn new(base_url: Option<&str>) -> Self {
        let client = Client::new();
        let base_url = base_url.unwrap_or(Self::BASE_URL).to_string();

        Self {
            http: client,
            base_url,
        }
    }

    pub async fn search_track(&self, query: &str) -> Result<Option<TrackInfo>, ITunesAPIError> {
        let url = format!("{}/search", self.base_url);
        let params = [
            ("term", query),
            ("media", "music"),
            ("entity", "song"),
            ("limit", "1"),
        ];

        let response = self
            .http
            .get(url)
            .query(&params)
            .send()
            .await
            .map_err(|e| ITunesAPIError::RequestFailed(e))?;

        if !response.status().is_success() {
            return Err(ITunesAPIError::RequestUnsuccessful);
        }

        let data: ITunesSearchResponse = response
            .json()
            .await
            .map_err(|_| ITunesAPIError::InvalidResponseBody)?;

        Ok(data.results.first().map(|track| TrackInfo {
            track_name: track.track_name.clone(),
            artist_name: track.artist_name.clone(),
            preview_url: track.preview_url.clone(),
            is_streamable: track.is_streamable.clone(),
        }))
    }
}
