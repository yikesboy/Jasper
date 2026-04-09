use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i32,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistPage {
    pub items: Vec<PlaylistItem>,
    pub next: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PlaylistItem {
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
