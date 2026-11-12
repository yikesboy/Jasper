use super::SpotifyClient;
use crate::services::spotify::SpotifyClientError;
use serde_json::json;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
    time::timeout,
};
use url::Url;

const TOKEN: &str = r#"{"access_token":"access","expires_in":3600}"#;

struct MockSpotify {
    client: SpotifyClient,
    requests: JoinHandle<Vec<String>>,
}

impl MockSpotify {
    async fn start(refresh_token: Option<&str>, responses: Vec<(u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let client = SpotifyClient::with_endpoints(
            "client",
            "secret",
            refresh_token,
            &base,
            &format!("{base}/token"),
        );
        let requests = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = timeout(Duration::from_secs(5), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
                let mut bytes = Vec::new();
                loop {
                    let mut buffer = [0_u8; 4096];
                    let n = timeout(Duration::from_secs(5), stream.read(&mut buffer))
                        .await
                        .unwrap()
                        .unwrap();
                    assert!(n > 0, "request ended before body was complete");
                    bytes.extend_from_slice(&buffer[..n]);
                    assert!(bytes.len() < 65536, "unexpectedly large test request");
                    if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                        let length = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length:"))
                            .map(|value| value.trim().parse::<usize>().unwrap())
                            .unwrap_or(0);
                        if bytes.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(bytes).unwrap());
                let body = body.replace("__BASE__", &base);
                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        Self { client, requests }
    }

    async fn requests(self) -> Vec<String> {
        timeout(Duration::from_secs(10), self.requests)
            .await
            .unwrap()
            .unwrap()
    }
}

fn playlist() -> Url {
    Url::parse("https://open.spotify.com/playlist/test").unwrap()
}

fn form(request: &str) -> Vec<(String, String)> {
    url::form_urlencoded::parse(request.split_once("\r\n\r\n").unwrap().1.as_bytes())
        .into_owned()
        .collect()
}

#[tokio::test]
async fn user_auth_fetches_current_items_and_paginates_with_bearer_token() {
    let mock = MockSpotify::start(
        Some("refresh & secret"),
        vec![
            (200, TOKEN.into()),
            (
                200,
                json!({"items": [
            {"item": {"name": "Africa", "artists": [{"name": "Toto"}]}},
            {"item": null}
        ], "next": "__BASE__/playlists/test/items?offset=2"})
                .to_string(),
            ),
            (
                200,
                json!({"items": [
            {"item": {"name": "Numb", "artists": [{"name": "Linkin Park"}]},
             "track": {"name": "Deprecated duplicate", "artists": []}}
        ], "next": null})
                .to_string(),
            ),
        ],
    )
    .await;

    let tracks = mock.client.get_playlist_tracks(playlist()).await.unwrap();
    assert_eq!(
        tracks
            .into_iter()
            .map(|track| track.into_search_query())
            .collect::<Vec<_>>(),
        ["Africa - Toto", "Numb - Linkin Park"]
    );
    let requests = mock.requests().await;
    assert!(requests[0].starts_with("POST /token "));
    assert!(requests[0].contains("Basic Y2xpZW50OnNlY3JldA=="));
    assert_eq!(
        form(&requests[0]),
        [
            ("grant_type".into(), "refresh_token".into()),
            ("refresh_token".into(), "refresh & secret".into())
        ]
    );
    assert!(requests[1].starts_with("GET /playlists/test/items "));
    assert!(requests[2].starts_with("GET /playlists/test/items?offset=2 "));
    for request in &requests[1..] {
        assert!(request.contains("Bearer access"));
        assert!(!request.contains("refresh & secret"));
        assert!(!request.contains("fields="));
    }
}

#[tokio::test]
async fn client_credentials_preserve_extended_quota_playlist_behavior() {
    let mock = MockSpotify::start(
        None,
        vec![
            (200, TOKEN.into()),
            (
                200,
                json!({"tracks": {"items": [
            {"track": {"name": "Africa", "artists": [{"name": "Toto"}]}}
        ], "next": "__BASE__/playlists/test/tracks?offset=1"}})
                .to_string(),
            ),
            (
                200,
                json!({"items": [{"track": null}], "next": null}).to_string(),
            ),
        ],
    )
    .await;

    assert_eq!(
        mock.client
            .get_playlist_tracks(playlist())
            .await
            .unwrap()
            .len(),
        1
    );
    let requests = mock.requests().await;
    assert_eq!(
        form(&requests[0]),
        [("grant_type".into(), "client_credentials".into())]
    );
    assert!(requests[1].starts_with("GET /playlists/test "));
    assert!(requests[2].starts_with("GET /playlists/test/tracks?offset=1 "));
}

#[tokio::test]
async fn metadata_only_is_an_access_error_not_an_empty_playlist() {
    let mock = MockSpotify::start(
        None,
        vec![
            (200, TOKEN.into()),
            (200, r#"{"name":"Restricted playlist"}"#.into()),
        ],
    )
    .await;
    let error = mock
        .client
        .get_playlist_tracks(playlist())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        SpotifyClientError::PlaylistContentsUnavailable
    ));
    assert!(error.to_string().contains("SPOTIFY_REFRESH_TOKEN"));
    assert_eq!(mock.requests().await.len(), 2);
}

#[tokio::test]
async fn forbidden_playlist_has_actionable_error_without_auth_fallback() {
    let mock = MockSpotify::start(
        Some("refresh"),
        vec![
            (200, TOKEN.into()),
            (403, r#"{"error":{"message":"Forbidden"}}"#.into()),
        ],
    )
    .await;
    let error = mock
        .client
        .get_playlist_tracks(playlist())
        .await
        .unwrap_err();
    assert!(matches!(error, SpotifyClientError::PlaylistAccessDenied));
    assert!(error.to_string().contains("own or collaborate"));
    assert_eq!(mock.requests().await.len(), 2);
}

#[tokio::test]
async fn revoked_refresh_token_requires_reauthorization_without_fallback() {
    let mock = MockSpotify::start(
        Some("private-refresh"),
        vec![(
            400,
            r#"{"error":"invalid_grant","error_description":"private-refresh"}"#.into(),
        )],
    )
    .await;
    let error = mock
        .client
        .get_playlist_tracks(playlist())
        .await
        .unwrap_err();
    assert!(matches!(error, SpotifyClientError::ReauthorizationRequired));
    assert!(!error.to_string().contains("private-refresh"));
    assert_eq!(mock.requests().await.len(), 1);
}

#[tokio::test]
async fn refresh_token_rotation_is_shared_and_missing_replacement_keeps_last_token() {
    let mock = MockSpotify::start(
        Some("original"),
        vec![
            (
                200,
                json!({"access_token": "first", "expires_in": 3600, "refresh_token": "rotated"})
                    .to_string(),
            ),
            (200, TOKEN.into()),
            (200, TOKEN.into()),
        ],
    )
    .await;
    let clone = mock.client.clone();
    assert_eq!(mock.client.get_bearer_token().await.unwrap(), "first");
    for _ in 0..2 {
        clone.auth.lock().await.bearer_token = None;
        assert_eq!(clone.get_bearer_token().await.unwrap(), "access");
    }
    let requests = mock.requests().await;
    assert_eq!(form(&requests[0])[1].1, "original");
    assert_eq!(form(&requests[1])[1].1, "rotated");
    assert_eq!(form(&requests[2])[1].1, "rotated");
}

#[tokio::test]
async fn concurrent_clones_share_one_token_refresh() {
    let mock = MockSpotify::start(Some("refresh"), vec![(200, TOKEN.into())]).await;
    let clone = mock.client.clone();
    let (first, second) = tokio::join!(mock.client.get_bearer_token(), clone.get_bearer_token());
    assert_eq!(first.unwrap(), "access");
    assert_eq!(second.unwrap(), "access");
    assert_eq!(mock.requests().await.len(), 1);
}

#[tokio::test]
async fn zero_token_lifetime_is_rejected_instead_of_caching_invalid_credentials() {
    let mock = MockSpotify::start(
        Some("refresh"),
        vec![(
            200,
            json!({"access_token": "access", "expires_in": 0}).to_string(),
        )],
    )
    .await;
    assert!(matches!(
        mock.client.get_bearer_token().await,
        Err(SpotifyClientError::InvalidTokenLifetime)
    ));
    assert_eq!(mock.requests().await.len(), 1);
}
