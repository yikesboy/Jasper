use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistPage {
    pub items: Vec<PlaylistItem>,
    pub next: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistResponse {
    pub items: Option<PlaylistPage>,
    pub tracks: Option<PlaylistPage>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistItem {
    pub item: Option<FullTrack>,
    pub track: Option<FullTrack>,
}

#[derive(Deserialize, Debug)]
pub struct FullTrack {
    pub name: String,
    pub artists: Vec<Artist>,
}

#[derive(Deserialize, Debug)]
pub struct Artist {
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::PlaylistResponse;
    use serde_json::json;

    #[test]
    fn current_playlist_fields_are_decoded() {
        let response: PlaylistResponse = serde_json::from_value(json!({
            "items": {"items": [
                {"item": {"name": "Africa", "artists": [{"name": "Toto"}]}},
                {"item": null}
            ], "next": "https://api.spotify.com/v1/playlists/test/items?offset=2"}
        }))
        .unwrap();

        let page = response.items.unwrap();
        assert_eq!(page.items[0].item.as_ref().unwrap().name, "Africa");
        assert!(page.items[1].item.is_none());
        assert!(page.next.unwrap().contains("/items?"));
    }

    #[test]
    fn legacy_playlist_fields_remain_supported() {
        let response: PlaylistResponse = serde_json::from_value(json!({
            "tracks": {"items": [
                {"track": {"name": "Africa", "artists": [{"name": "Toto"}]}}
            ], "next": null}
        }))
        .unwrap();

        assert_eq!(
            response.tracks.unwrap().items[0]
                .track
                .as_ref()
                .unwrap()
                .name,
            "Africa"
        );
    }

    #[test]
    fn metadata_only_response_is_distinct_from_empty_playlist() {
        let metadata: PlaylistResponse =
            serde_json::from_value(json!({"name": "Restricted"})).unwrap();
        assert!(metadata.items.is_none() && metadata.tracks.is_none());

        let empty: PlaylistResponse = serde_json::from_value(json!({
            "items": {"items": [], "next": null}
        }))
        .unwrap();
        assert!(empty.items.unwrap().items.is_empty());
    }
}
