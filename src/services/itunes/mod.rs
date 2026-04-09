mod error;
pub mod itunes;
pub mod models;

pub use error::ITunesAPIError;
pub use itunes::ItunesAPI;
pub use models::TrackInfo;
