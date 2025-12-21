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
struct ITunesSearchResponse {
    result_count: i32,
    results: Vec<ITunesTrack>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ITunesTrack {
    wrapper_type: String,
    kind: String,
    artist_id: i32,
    collection_id: i32,
    track_id: i32,
    artist_name: String,
    collection_name: String,
    track_name: String,
    collection_censored_name: String,
    track_censored_name: String,
    artist_view_url: String,
    collection_view_url: String,
    track_view_url: String,
    preview_url: String,
    artwork_url_30: String,
    artwork_url_60: String,
    artwork_url_100: String,
    collection_price: f32,
    track_price: f32,
    release_date: String,
    collection_explicitness: String,
    track_explicitness: String,
    disc_count: i32,
    disc_number: i32,
    track_count: i32,
    track_number: i32,
    track_time_millis: i32,
    country: String,
    currency: String,
    primary_genre_name: String,
    is_streamable: bool,
}

pub async fn search_track(
    query: &str,
) -> Result<Option<TrackInfo>, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let url = "https://itunes.apple.com/search";
    let params = [
        ("term", query),
        ("media", "music"),
        ("entity", "song"),
        ("limit", "1"),
    ];

    let response = client.get(url).query(&params).send().await?;
    let data: ITunesSearchResponse = response.json().await?;

    Ok(data.results.first().map(|track| TrackInfo {
        track_name: track.track_name.clone(),
        artist_name: track.artist_name.clone(),
        preview_url: track.preview_url.clone(),
        is_streamable: track.is_streamable.clone(),
    }))
}
