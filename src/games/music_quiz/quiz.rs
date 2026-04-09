use crate::games::music_quiz::error::MusicQuizError;
use crate::games::music_quiz::session::MusicQuizSession;
use crate::services::itunes::itunes::ItunesAPI;
use crate::services::itunes::models::TrackInfo;
use crate::services::spotify::SpotifyAPI;
use rand::seq::SliceRandom;
use serenity::all::UserId;
use std::sync::Arc;
use strsim::normalized_levenshtein;
use tokio::sync::Notify;
use url::Url;

enum GuessResult {
    ArtistOnly,
    TrackOnly,
    Both,
    Neither,
}

pub enum GuessOutcome {
    Both { points: u32 },
    Artist { points: u32 },
    Track { points: u32 },
    AlreadyGuessed,
    Wrong,
}
pub struct MusicQuiz {
    session: MusicQuizSession,
    round_complete_notify: Arc<Notify>,
    itunes: Arc<ItunesAPI>,
    spotify: Arc<SpotifyAPI>,
}

impl MusicQuiz {
    const SIMILARITY_THRESHOLD_BOTH: f64 = 0.88;
    const SIMILARITY_THRESHOLD_TRACK: f64 = 0.90;
    const SIMILARITY_THRESHOLD_ARTIST: f64 = 0.85;

    pub fn new(
        total_rounds: u32,
        participants: Vec<UserId>,
        itunes: Arc<ItunesAPI>,
        spotify: Arc<SpotifyAPI>,
    ) -> Self {
        Self {
            session: MusicQuizSession::new(total_rounds, participants),
            round_complete_notify: Arc::new(Notify::new()),
            itunes,
            spotify,
        }
    }

    pub fn session(&self) -> &MusicQuizSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut MusicQuizSession {
        &mut self.session
    }

    pub fn make_guess(
        &mut self,
        user_id: UserId,
        guess: &str,
    ) -> Result<GuessOutcome, MusicQuizError> {
        let round = match self.session.current_round.as_mut() {
            Some(round) => round,
            None => return Err(MusicQuizError::NoRoundInProgress),
        };
        let guess_result = Self::evaluate_user_guess(guess, &round.track);

        let outcome = match guess_result {
            GuessResult::Both => match (round.track_guessed_by, round.artist_guessed_by) {
                (None, None) => {
                    round.artist_guessed_by = Some(user_id);
                    round.track_guessed_by = Some(user_id);
                    self.session.add_score(user_id, 3);
                    GuessOutcome::Both { points: 3 }
                }
                (Some(_), None) => {
                    round.track_guessed_by = Some(user_id);
                    self.session.add_score(user_id, 1);
                    GuessOutcome::Track { points: 1 }
                }
                (None, Some(_)) => {
                    round.artist_guessed_by = Some(user_id);
                    self.session.add_score(user_id, 1);
                    GuessOutcome::Artist { points: 1 }
                }
                (Some(_), Some(_)) => GuessOutcome::AlreadyGuessed,
            },
            GuessResult::TrackOnly => match round.track_guessed_by {
                Some(_) => GuessOutcome::AlreadyGuessed,
                None => {
                    let points = if round.artist_guessed_by == Some(user_id) {
                        round.track_guessed_by = Some(user_id);
                        self.session.add_score(user_id, 2);
                        3
                    } else {
                        round.track_guessed_by = Some(user_id);
                        self.session.add_score(user_id, 1);
                        1
                    };
                    GuessOutcome::Track { points }
                }
            },
            GuessResult::ArtistOnly => match round.artist_guessed_by {
                Some(_) => GuessOutcome::AlreadyGuessed,
                None => {
                    let points = if round.track_guessed_by == Some(user_id) {
                        round.artist_guessed_by = Some(user_id);
                        self.session.add_score(user_id, 2);
                        3
                    } else {
                        round.artist_guessed_by = Some(user_id);
                        self.session.add_score(user_id, 1);
                        1
                    };
                    GuessOutcome::Artist { points }
                }
            },
            GuessResult::Neither => GuessOutcome::Wrong,
        };

        if self.is_round_complete() {
            self.round_complete_notify.notify_waiters();
        }

        Ok(outcome)
    }

    pub fn notify_round_complete(&self) -> Arc<Notify> {
        self.round_complete_notify.clone()
    }

    fn evaluate_user_guess(guess: &str, current_track: &TrackInfo) -> GuessResult {
        let normalized_track_name = Self::normalize_string(&current_track.track_name);
        let normalized_track_artist = Self::normalize_string(&current_track.artist_name);
        let normalized_guess = Self::normalize_string(guess);

        let both = format!("{}{}", normalized_track_name, normalized_track_artist);
        let both_reverse = format!("{}{}", normalized_track_artist, normalized_track_name);
        let both_match = normalized_levenshtein(&normalized_guess, &both)
            > Self::SIMILARITY_THRESHOLD_BOTH
            || normalized_levenshtein(&normalized_guess, &both_reverse)
                > Self::SIMILARITY_THRESHOLD_BOTH;

        if both_match {
            return GuessResult::Both;
        }

        let track_matches = normalized_levenshtein(&normalized_guess, &normalized_track_name)
            > Self::SIMILARITY_THRESHOLD_TRACK;
        let artist_matches = normalized_levenshtein(&normalized_guess, &normalized_track_artist)
            > Self::SIMILARITY_THRESHOLD_ARTIST;

        match (track_matches, artist_matches) {
            (true, _) => GuessResult::TrackOnly,
            (_, true) => GuessResult::ArtistOnly,
            _ => GuessResult::Neither,
        }
    }

    fn normalize_string(input: &str) -> String {
        input
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect()
    }

    pub async fn fetch_random_tracks_from_playlist(
        &self,
        playlist_link: Url,
        count: u32,
    ) -> Result<Vec<TrackInfo>, MusicQuizError> {
        let mut track_list = self
            .spotify
            .get_playlist_tracks(playlist_link)
            .await
            .map_err(|e| MusicQuizError::FailedToFetchTracksFromPlaylist(e.to_string()))?;

        if track_list.is_empty() || track_list.len() < count as usize {
            return Err(MusicQuizError::PlaylistContainsNotEnoughSongs {
                expected: count,
                actual: track_list.len() as u32,
            });
        }

        track_list.shuffle(&mut rand::rng());

        let mut result = Vec::with_capacity(count as usize);

        for ft in track_list {
            if result.len() >= count as usize {
                break;
            }

            let artists = ft
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<&str>>()
                .join(", ");
            let search_query = format! {"{} - {}", ft.name, artists};

            match self.itunes.search_track(&search_query).await {
                Ok(Some(track_info)) if !track_info.preview_url.is_empty() => {
                    result.push(track_info)
                }
                Ok(None) | Ok(Some(_)) => continue,
                Err(e) => return Err(MusicQuizError::FetchError(e.to_string())),
            }
        }

        if result.len() < count as usize {
            return Err(MusicQuizError::PlaylistContainsNotEnoughPreviewableSongs {
                expected: count,
                actual: result.len() as u32,
            });
        }

        Ok(result)
    }

    pub fn is_round_complete(&self) -> bool {
        if let Some(round) = &self.session.current_round {
            round.artist_guessed_by.is_some() && round.track_guessed_by.is_some()
        } else {
            false
        }
    }
}
