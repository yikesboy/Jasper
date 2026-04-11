use super::error::SpotifyClientError;
use super::models::{FullTrack, PlaylistPage, TokenResponse};
use reqwest::{header, Client};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use url::Url;

#[derive(Clone)]
pub struct SpotifyClient {
    http: Client,
    base_url: String,
    client_id: String,
    client_secret: String,
    bearer_token: Arc<RwLock<Option<(String, Instant)>>>,
}

impl SpotifyClient {
    const BASE_URL: &'static str = "https://api.spotify.com/v1";
    const TOKEN_URL: &'static str = "https://accounts.spotify.com/api/token";
    pub async fn new(
        client_id: &str,
        client_secret: &str,
        base_url: Option<&str>,
    ) -> Result<Self, SpotifyClientError> {
        let client = Client::new();
        let base_url = base_url.unwrap_or(Self::BASE_URL).to_string();

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
        playlist_url: Url,
    ) -> Result<Vec<FullTrack>, SpotifyClientError> {
        let playlist_id = Self::retrieve_playlist_id(playlist_url)?;
        let initial_page_url = format!("{}/playlists/{}/tracks", self.base_url, playlist_id);
        let mut next_page_url = Some(initial_page_url.clone());
        let bearer_token = self.get_bearer_token().await?;
        let mut tracks = Vec::new();

        while let Some(page_url) = next_page_url.take() {
            let request = self
                .http
                .get(&page_url)
                .bearer_auth(&bearer_token)
                .header(header::ACCEPT, "application/json");

            let response = if page_url == initial_page_url {
                request
                    .query(&[
                        ("fields", "items(track(name,artists(name))),next"),
                        ("limit", "100"),
                    ])
                    .send()
                    .await
            } else {
                request.send().await
            }
            .map_err(SpotifyClientError::RequestFailed)?;

            if !response.status().is_success() {
                return Err(SpotifyClientError::FailedToRetrievePlaylistTracks);
            }

            let page: PlaylistPage = response
                .json()
                .await
                .map_err(SpotifyClientError::InvalidPlaylistResponse)?;

            tracks.extend(page.items.into_iter().filter_map(|item| item.track));
            next_page_url = page.next;
        }

        Ok(tracks)
    }

    async fn get_bearer_token(&self) -> Result<String, SpotifyClientError> {
        {
            let token_guard = self.bearer_token.read().await;

            if let Some((token, expires_at)) = &*token_guard {
                if expires_at > &Instant::now() {
                    return Ok(token.clone());
                }
            }
        }

        let response = self
            .http
            .post(Self::TOKEN_URL)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&[("grant_type", "client_credentials")])
            .send()
            .await
            .map_err(SpotifyClientError::AuthenticationRequestFailed)?;

        let status = response.status();
        if !status.is_success() {
            return Err(SpotifyClientError::AuthenticationRejected(status));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(SpotifyClientError::AuthenticationResponseInvalid)?;

        let expires_at = Instant::now() + Duration::from_secs(token_response.expires_in as u64);

        {
            let mut token_guard = self.bearer_token.write().await;
            *token_guard = Some((token_response.access_token.clone(), expires_at));
        }

        Ok(token_response.access_token)
    }

    fn retrieve_playlist_id(playlist_url: Url) -> Result<String, SpotifyClientError> {
        if playlist_url.host_str() != Some("open.spotify.com") {
            return Err(SpotifyClientError::InvalidLink(playlist_url));
        }

        let mut segments = match playlist_url.path_segments() {
            Some(segments) => segments,
            None => return Err(SpotifyClientError::MalformedUrl(playlist_url.to_string())),
        };

        if segments.next() != Some("playlist") {
            return Err(SpotifyClientError::NotPlaylistLink(playlist_url));
        }

        let id = match segments.next() {
            Some(id) => id.to_string(),
            None => return Err(SpotifyClientError::PlaylistIdMissing),
        };

        if let Some(segment) = segments.next() {
            return Err(SpotifyClientError::UnexpectedSegment(segment.to_string()));
        }

        Ok(id)
    }
}
