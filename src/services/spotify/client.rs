use super::error::SpotifyAPIError;
use super::models::{FullTrack, PlaylistTracks, TokenResponse};
use reqwest::{Client, header};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use url::Url;

pub struct SpotifyAPI {
    http: Client,
    base_url: String,
    client_id: String,
    client_secret: String,
    bearer_token: Arc<RwLock<Option<(String, Instant)>>>,
}

impl SpotifyAPI {
    pub async fn new(
        client_id: &str,
        client_secret: &str,
        base_url: Option<&str>,
    ) -> Result<Self, SpotifyAPIError> {
        let client = reqwest::Client::new();
        let base_url = base_url.unwrap_or("https://api.spotify.com/v1").to_string();

        Ok(Self {
            http: client,
            base_url,
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            bearer_token: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn get_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<FullTrack>, SpotifyAPIError> {
        let url = format!("{}/playlist/{}/tracks", self.base_url, playlist_id);
        let filter_query = "fields=items(track(name,artists(name)))";
        let bearer_token = self.get_bearer_token().await?;

        let response = self
            .http
            .get(&url)
            .query(filter_query)
            .bearer_auth(bearer_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(|_| SpotifyAPIError::FailedToRetrievePlaylistTracks)?
            .json::<PlaylistTracks>()
            .await
            .map_err(|_| SpotifyAPIError::FailedToRetrievePlaylistTracks)?;

        Ok(response.items.into_iter().map(|item| item.track).collect())
    }

    async fn get_bearer_token(&self) -> Result<String, SpotifyAPIError> {
        let mut token_guard = self.bearer_token.write().unwrap();
        if let Some((token, expires_at)) = &*token_guard {
            if expires_at > &Instant::now() {
                return Ok(token.clone());
            }
        }

        let token_url = "https://accounts.spotify.com/api/token";
        let response = self
            .http
            .post(token_url)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(|e| SpotifyAPIError::FailedToAuthenticate(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SpotifyAPIError::FailedToAuthenticate(
                response.status().to_string(),
            ));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| SpotifyAPIError::FailedToAuthenticate(e.to_string()))?;

        let expires_at = Instant::now() + Duration::from_secs(token_response.expires_in as u64);

        *token_guard = Some((token_response.access_token.clone(), expires_at));

        Ok(token_response.access_token)
    }

    fn retrieve_playlist_id(playlist_url: Url) -> Result<String, SpotifyAPIError> {
        if playlist_url.host_str() != Some("open.spotify.com") {
            return Err(SpotifyAPIError::InvalidLink(playlist_url));
        }

        let mut segments = match playlist_url.path_segments() {
            Some(segments) => segments,
            None => return Err(SpotifyAPIError::MalformedUrl(playlist_url)),
        };

        if segments.next() != Some("playlist") {
            return Err(SpotifyAPIError::NotPlaylistLink(playlist_url));
        }

        let id = match segments.next() {
            Some(id) => id.to_string(),
            None => return Err(SpotifyAPIError::PlaylistIdMissing),
        };

        if let Some(segment) = segments.next() {
            return Err(SpotifyAPIError::UnexpectedSegment(segment.to_string()));
        }

        Ok(id)
    }
}
