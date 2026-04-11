use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub track_name: String,
    pub artist_name: String,
    pub preview_url: Option<Url>,
    pub is_streamable: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItunesSearchResponse {
    pub results: Vec<ItunesTrack>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItunesTrack {
    pub artist_name: String,
    pub track_name: String,
    pub preview_url: Option<String>,
    pub is_streamable: bool,
}
