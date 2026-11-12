use super::error::SpotifyClientError;
use super::models::{FullTrack, PlaylistPage, PlaylistResponse, TokenResponse};
use reqwest::{Client, header};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, warn};
use url::Url;

#[derive(Debug, Clone)]
pub struct PlaylistTrack {
    title: String,
    artists: Vec<String>,
}

impl PlaylistTrack {
    pub fn into_search_query(self) -> String {
        format!("{} - {}", self.title, self.artists.join(", "))
    }
}

impl From<FullTrack> for PlaylistTrack {
    fn from(track: FullTrack) -> Self {
        Self {
            title: track.name,
            artists: track
                .artists
                .into_iter()
                .map(|artist| artist.name)
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct SpotifyClient {
    http: Client,
    base_url: String,
    token_url: String,
    client_id: String,
    client_secret: String,
    user_authorization: bool,
    auth: Arc<Mutex<SpotifyAuthState>>,
}

struct SpotifyAuthState {
    bearer_token: Option<(String, Instant)>,
    refresh_token: Option<String>,
}

impl SpotifyClient {
    const BASE_URL: &'static str = "https://api.spotify.com/v1";
    const TOKEN_URL: &'static str = "https://accounts.spotify.com/api/token";
    pub async fn new(
        client_id: &str,
        client_secret: &str,
        refresh_token: Option<&str>,
        base_url: Option<&str>,
    ) -> Result<Self, SpotifyClientError> {
        Ok(Self::with_endpoints(
            client_id,
            client_secret,
            refresh_token,
            base_url.unwrap_or(Self::BASE_URL),
            Self::TOKEN_URL,
        ))
    }

    fn with_endpoints(
        client_id: &str,
        client_secret: &str,
        refresh_token: Option<&str>,
        base_url: &str,
        token_url: &str,
    ) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.to_string(),
            token_url: token_url.to_string(),
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            user_authorization: refresh_token.is_some(),
            auth: Arc::new(Mutex::new(SpotifyAuthState {
                bearer_token: None,
                refresh_token: refresh_token.map(str::to_owned),
            })),
        }
    }

    pub async fn get_playlist_tracks(
        &self,
        playlist_url: Url,
    ) -> Result<Vec<PlaylistTrack>, SpotifyClientError> {
        let playlist_id = Self::retrieve_playlist_id(playlist_url)?;
        debug!(playlist_id = %playlist_id, "Fetching Spotify playlist tracks");
        let bearer_token = self.get_bearer_token().await?;
        let playlist_endpoint = format!("{}/playlists/{}", self.base_url, playlist_id);
        let first_page = if self.user_authorization {
            self.fetch_playlist_page_response::<PlaylistPage>(
                &format!("{playlist_endpoint}/items"),
                &bearer_token,
                &playlist_id,
            )
            .await?
        } else {
            // Extended Quota apps retain the legacy client-credentials contract.
            let response = self
                .fetch_playlist_page_response::<PlaylistResponse>(
                    &playlist_endpoint,
                    &bearer_token,
                    &playlist_id,
                )
                .await?;
            response
                .items
                .or(response.tracks)
                .ok_or(SpotifyClientError::PlaylistContentsUnavailable)?
        };
        let mut tracks = Self::collect_playlist_tracks(first_page.items);
        let mut next_page_url: Option<String> = first_page.next;

        while let Some(next_page) = next_page_url.take() {
            let page = self
                .fetch_playlist_page_response::<PlaylistPage>(
                    &next_page,
                    &bearer_token,
                    &playlist_id,
                )
                .await?;
            tracks.extend(Self::collect_playlist_tracks(page.items));
            next_page_url = page.next;
        }

        debug!(
            playlist_id = %playlist_id,
            track_count = tracks.len(),
            "Fetched Spotify playlist tracks"
        );
        Ok(tracks)
    }

    async fn fetch_playlist_page_response<T>(
        &self,
        page_url: &str,
        bearer_token: &str,
        playlist_id: &str,
    ) -> Result<T, SpotifyClientError>
    where
        T: serde::de::DeserializeOwned,
    {
        let response = self
            .http
            .get(page_url)
            .bearer_auth(bearer_token)
            .header(header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(SpotifyClientError::RequestFailed)?;

        let status = response.status();
        if status == reqwest::StatusCode::FORBIDDEN {
            return Err(SpotifyClientError::PlaylistAccessDenied);
        }
        if !status.is_success() {
            let body = Self::read_error_response_body(response).await;
            warn!(
                playlist_id = %playlist_id,
                page_url = %page_url,
                %status,
                response_body = %body,
                "Spotify playlist request rejected"
            );
            return Err(SpotifyClientError::FailedToRetrievePlaylistTracks { status, body });
        }

        response
            .json()
            .await
            .map_err(SpotifyClientError::InvalidPlaylistResponse)
    }

    fn collect_playlist_tracks(items: Vec<super::models::PlaylistItem>) -> Vec<PlaylistTrack> {
        items
            .into_iter()
            .filter_map(|item| item.item.or(item.track))
            .map(PlaylistTrack::from)
            .collect()
    }

    async fn get_bearer_token(&self) -> Result<String, SpotifyClientError> {
        // Serialize refreshes so a rotated refresh token is used by every clone.
        let mut auth = self.auth.lock().await;
        if let Some((token, expires_at)) = &auth.bearer_token {
            if *expires_at > Instant::now() {
                return Ok(token.clone());
            }
        }

        debug!("Refreshing Spotify bearer token");
        let mut form = vec![("grant_type", "client_credentials")];
        if let Some(refresh_token) = auth.refresh_token.as_deref() {
            form = vec![
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh_token),
            ];
        }
        let requested_at = Instant::now();
        let response = self
            .http
            .post(&self.token_url)
            .basic_auth(&self.client_id, Some(&self.client_secret))
            .form(&form)
            .send()
            .await
            .map_err(SpotifyClientError::AuthenticationRequestFailed)?;

        let status = response.status();
        if !status.is_success() {
            if self.user_authorization {
                // Do not log the token response or fall back to app credentials.
                let body: serde_json::Value = response.json().await.unwrap_or_default();
                if body.get("error").and_then(|value| value.as_str()) == Some("invalid_grant") {
                    return Err(SpotifyClientError::ReauthorizationRequired);
                }
            }
            return Err(SpotifyClientError::AuthenticationRejected(status));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(SpotifyClientError::AuthenticationResponseInvalid)?;

        if token_response.access_token.is_empty() || token_response.expires_in == 0 {
            return Err(SpotifyClientError::InvalidTokenLifetime);
        }
        let expires_at = requested_at
            .checked_add(Duration::from_secs(
                token_response.expires_in.saturating_sub(30),
            ))
            .ok_or(SpotifyClientError::InvalidTokenLifetime)?;
        if self.user_authorization {
            if let Some(refresh_token) = token_response.refresh_token {
                if refresh_token.is_empty() {
                    return Err(SpotifyClientError::InvalidTokenLifetime);
                }
                auth.refresh_token = Some(refresh_token);
            }
        }
        auth.bearer_token = Some((token_response.access_token.clone(), expires_at));

        Ok(token_response.access_token)
    }

    async fn read_error_response_body(response: reqwest::Response) -> String {
        match response.text().await {
            Ok(body) => Self::truncate_error_body(&body),
            Err(error) => format!("<failed to read response body: {error}>"),
        }
    }

    fn truncate_error_body(body: &str) -> String {
        let body = body.trim();
        if body.is_empty() {
            return "<empty response body>".to_string();
        }

        const MAX_ERROR_BODY_LEN: usize = 512;
        if body.len() <= MAX_ERROR_BODY_LEN {
            return body.to_string();
        }

        let truncated = body
            .char_indices()
            .take_while(|(idx, _)| *idx < MAX_ERROR_BODY_LEN)
            .map(|(_, ch)| ch)
            .collect::<String>();
        format!("{truncated}...")
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

#[cfg(test)]
#[path = "client_tests.rs"]
mod http_tests;

#[cfg(test)]
mod tests {
    use super::SpotifyClient;
    use crate::services::spotify::SpotifyClientError;
    use url::Url;

    #[test]
    fn retrieve_playlist_id_accepts_valid_playlist_url() {
        let url = Url::parse("https://open.spotify.com/playlist/abc123").unwrap();

        let playlist_id = SpotifyClient::retrieve_playlist_id(url).unwrap();

        assert_eq!(playlist_id, "abc123");
    }

    #[test]
    fn retrieve_playlist_id_accepts_shared_playlist_url_with_query_string() {
        let url = Url::parse(
            "https://open.spotify.com/playlist/37i9dQZF1DX0Yxoavh5qJV?si=c49cc0082b34401c",
        )
        .unwrap();

        let playlist_id = SpotifyClient::retrieve_playlist_id(url).unwrap();

        assert_eq!(playlist_id, "37i9dQZF1DX0Yxoavh5qJV");
    }

    #[test]
    fn retrieve_playlist_id_rejects_non_spotify_hosts() {
        let url = Url::parse("https://example.com/playlist/abc123").unwrap();

        let error = SpotifyClient::retrieve_playlist_id(url).unwrap_err();

        assert!(matches!(error, SpotifyClientError::InvalidLink(_)));
    }

    #[test]
    fn retrieve_playlist_id_rejects_extra_path_segments() {
        let url = Url::parse("https://open.spotify.com/playlist/abc123/tracks").unwrap();

        let error = SpotifyClient::retrieve_playlist_id(url).unwrap_err();

        assert!(
            matches!(error, SpotifyClientError::UnexpectedSegment(segment) if segment == "tracks")
        );
    }

    #[test]
    fn truncate_error_body_replaces_empty_bodies() {
        let body = SpotifyClient::truncate_error_body("   ");

        assert_eq!(body, "<empty response body>");
    }

    #[test]
    fn truncate_error_body_limits_logged_response_size() {
        let body = SpotifyClient::truncate_error_body(&"a".repeat(600));

        assert_eq!(body.len(), 515);
        assert!(body.ends_with("..."));
    }
}
