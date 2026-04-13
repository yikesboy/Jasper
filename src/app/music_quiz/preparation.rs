use crate::commands::MusicQuizCommandError;
use crate::games::music_quiz::QuizTrack;
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

            let preview_track = data
                .itunes
                .search_preview_track(&track.search_query())
                .await
                .map_err(MusicQuizCommandError::Itunes)?;

            let Some(preview_track) = preview_track else {
                continue;
            };

            result.push(QuizTrack::new(
                preview_track.title().to_string(),
                preview_track.artist().to_string(),
                preview_track.preview_url().clone(),
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
}
