use crate::commands::MusicQuizCommandError;
use crate::games::music_quiz::QuizTrack;
use crate::services::spotify::models::FullTrack;
use crate::Data;
use rand::seq::SliceRandom;
use url::Url;

pub struct MusicQuizPreparationService;

impl MusicQuizPreparationService {
    pub async fn fetch_quiz_tracks(
        data: &Data,
        playlist_link: Url,
        count: u32,
    ) -> Result<Vec<QuizTrack>, MusicQuizCommandError> {
        let mut track_list = data
            .spotify
            .get_playlist_tracks(playlist_link)
            .await
            .map_err(MusicQuizCommandError::Spotify)?;

        if track_list.len() < count as usize {
            return Err(MusicQuizCommandError::PlaylistContainsNotEnoughSongs {
                expected: count,
                actual: track_list.len() as u32,
            });
        }

        track_list.shuffle(&mut rand::rng());

        let mut result = Vec::with_capacity(count as usize);

        for track in track_list {
            if result.len() >= count as usize {
                break;
            }

            let search_query = Self::build_search_query(&track);
            let track_info = data
                .itunes
                .search_track(&search_query)
                .await
                .map_err(MusicQuizCommandError::ITunes)?;

            let Some(track_info) = track_info else {
                continue;
            };

            let Some(preview_url) = track_info.preview_url else {
                continue;
            };

            result.push(QuizTrack::new(
                track_info.track_name,
                track_info.artist_name,
                preview_url,
            ));
        }

        if result.len() < count as usize {
            return Err(MusicQuizCommandError::PlaylistContainsNotEnoughPreviewableSongs {
                expected: count,
                actual: result.len() as u32,
            });
        }

        Ok(result)
    }

    fn build_search_query(track: &FullTrack) -> String {
        let artists = track
            .artists
            .iter()
            .map(|artist| artist.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        format!("{} - {}", track.name, artists)
    }
}
