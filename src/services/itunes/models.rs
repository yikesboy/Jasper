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
    pub result_count: i32,
    pub results: Vec<ITunesTrack>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ITunesTrack {
    pub wrapper_type: String,
    pub kind: String,
    pub artist_id: i32,
    pub collection_id: i32,
    pub track_id: i32,
    pub artist_name: String,
    pub collection_name: String,
    pub track_name: String,
    pub collection_censored_name: String,
    pub track_censored_name: String,
    pub artist_view_url: String,
    pub collection_view_url: String,
    pub track_view_url: String,
    pub preview_url: String,
    pub artwork_url_30: String,
    pub artwork_url_60: String,
    pub artwork_url_100: String,
    pub collection_price: f32,
    pub track_price: f32,
    pub release_date: String,
    pub collection_explicitness: String,
    pub track_explicitness: String,
    pub disc_count: i32,
    pub disc_number: i32,
    pub track_count: i32,
    pub track_number: i32,
    pub track_time_millis: i32,
    pub country: String,
    pub currency: String,
    pub primary_genre_name: String,
    pub is_streamable: bool,
}
