use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i32,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistTracks {
    pub items: Vec<PlaylistItem>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistItem {
    pub track: FullTrack,
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

#[derive(Deserialize)]
pub struct PlaylistResponse {
    pub tracks: PlaylistTracks,
}
