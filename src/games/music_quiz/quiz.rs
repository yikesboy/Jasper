use crate::games::music_quiz::error::MusicQuizError;
use crate::games::music_quiz::session::MusicQuizSession;
use crate::games::music_quiz::QuizTrack;
use serenity::all::UserId;
use strsim::normalized_levenshtein;
use tokio::sync::Notify;

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
    round_complete_notify: std::sync::Arc<Notify>,
}

impl MusicQuiz {
    const SIMILARITY_THRESHOLD_BOTH: f64 = 0.88;
    const SIMILARITY_THRESHOLD_TRACK: f64 = 0.90;
    const SIMILARITY_THRESHOLD_ARTIST: f64 = 0.85;

    pub fn new(
        total_rounds: u32,
        participants: Vec<UserId>,
    ) -> Self {
        Self {
            session: MusicQuizSession::new(total_rounds, participants),
            round_complete_notify: std::sync::Arc::new(Notify::new()),
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
        let round = match self.session.current_round_mut() {
            Some(round) => round,
            None => return Err(MusicQuizError::NoRoundInProgress),
        };
        let guess_result = Self::evaluate_user_guess(guess, round.track());

        let outcome = match guess_result {
            GuessResult::Both => match (round.track_guessed_by(), round.artist_guessed_by()) {
                (None, None) => {
                    round.set_artist_guessed_by(user_id);
                    round.set_track_guessed_by(user_id);
                    self.session.add_score(user_id, 3);
                    GuessOutcome::Both { points: 3 }
                }
                (Some(_), None) => {
                    round.set_track_guessed_by(user_id);
                    self.session.add_score(user_id, 1);
                    GuessOutcome::Track { points: 1 }
                }
                (None, Some(_)) => {
                    round.set_artist_guessed_by(user_id);
                    self.session.add_score(user_id, 1);
                    GuessOutcome::Artist { points: 1 }
                }
                (Some(_), Some(_)) => GuessOutcome::AlreadyGuessed,
            },
            GuessResult::TrackOnly => match round.track_guessed_by() {
                Some(_) => GuessOutcome::AlreadyGuessed,
                None => {
                    let points = if round.artist_guessed_by() == Some(user_id) {
                        round.set_track_guessed_by(user_id);
                        self.session.add_score(user_id, 2);
                        3
                    } else {
                        round.set_track_guessed_by(user_id);
                        self.session.add_score(user_id, 1);
                        1
                    };
                    GuessOutcome::Track { points }
                }
            },
            GuessResult::ArtistOnly => match round.artist_guessed_by() {
                Some(_) => GuessOutcome::AlreadyGuessed,
                None => {
                    let points = if round.track_guessed_by() == Some(user_id) {
                        round.set_artist_guessed_by(user_id);
                        self.session.add_score(user_id, 2);
                        3
                    } else {
                        round.set_artist_guessed_by(user_id);
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

    pub fn notify_round_complete(&self) -> std::sync::Arc<Notify> {
        self.round_complete_notify.clone()
    }

    fn evaluate_user_guess(guess: &str, current_track: &QuizTrack) -> GuessResult {
        let normalized_track_name = Self::normalize_string(current_track.name());
        let normalized_track_artist = Self::normalize_string(current_track.artist());
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

    pub fn is_round_complete(&self) -> bool {
        if let Some(round) = self.session.current_round() {
            round.is_complete()
        } else {
            false
        }
    }
}
