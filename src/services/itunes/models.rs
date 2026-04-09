use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub track_name: String,
    pub artist_name: String,
    pub preview_url: String,
    pub is_streamable: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ITunesSearchResponse {
    pub results: Vec<ITunesTrack>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ITunesTrack {
    pub artist_name: String,
    pub track_name: String,
    pub preview_url: String,
    pub is_streamable: bool,
}
