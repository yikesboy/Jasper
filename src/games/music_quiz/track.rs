use url::Url;

#[derive(Debug, Clone)]
pub struct QuizTrack {
    name: String,
    artist: String,
    preview_url: Url,
}

impl QuizTrack {
    pub fn new(name: String, artist: String, preview_url: Url) -> Self {
        Self {
            name,
            artist,
            preview_url,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn artist(&self) -> &str {
        &self.artist
    }

    pub fn preview_url(&self) -> &Url {
        &self.preview_url
    }
}
