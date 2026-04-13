use serde::Deserialize;

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
}
