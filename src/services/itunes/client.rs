use super::error::ItunesClientError;
use super::models::ItunesSearchResponse;
use reqwest::Client;
use url::Url;

#[derive(Debug, Clone)]
pub struct PreviewTrack {
    title: String,
    artist: String,
    preview_url: Url,
}

impl PreviewTrack {
    pub fn new(title: String, artist: String, preview_url: Url) -> Self {
        Self {
            title,
            artist,
            preview_url,
        }
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn preview_url(&self) -> &Url {
        &self.preview_url
    }
}

#[derive(Clone)]
pub struct ItunesClient {
    http: Client,
    base_url: String,
}

impl ItunesClient {
    const BASE_URL: &'static str = "https://itunes.apple.com";
    pub fn new(base_url: Option<&str>) -> Self {
        let client = Client::new();
        let base_url = base_url.unwrap_or(Self::BASE_URL).to_string();

        Self {
            http: client,
            base_url,
        }
    }

    pub async fn search_preview_track(
        &self,
        query: &str,
    ) -> Result<Option<PreviewTrack>, ItunesClientError> {
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
            .map_err(ItunesClientError::RequestFailed)?;

        if !response.status().is_success() {
            return Err(ItunesClientError::RequestUnsuccessful);
        }

        let data: ItunesSearchResponse = response
            .json()
            .await
            .map_err(ItunesClientError::InvalidResponseBody)?;

        let Some(track) = data.results.first() else {
            return Ok(None);
        };

        let preview_url = track
            .preview_url
            .as_deref()
            .map(Url::parse)
            .transpose()
            .map_err(ItunesClientError::InvalidPreviewUrl)?;

        let Some(preview_url) = preview_url else {
            return Ok(None);
        };

        Ok(Some(PreviewTrack::new(
            track.track_name.clone(),
            track.artist_name.clone(),
            preview_url,
        )))
    }
}
