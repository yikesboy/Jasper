use crate::games::music_quiz::quiz::{GuessOutcome, MusicQuiz};
use crate::games::music_quiz::{MusicQuizError, QuizTrack};
use serenity::all::UserId;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

#[derive(Clone)]
pub struct MusicQuizHandle {
    quiz: Arc<Mutex<MusicQuiz>>,
}

pub struct RoundProgress {
    pub round_number: u32,
    pub total_rounds: u32,
}

pub struct RoundCompletion {
    pub track_name: String,
    pub artist_name: String,
    pub artist_guessed_by: Option<UserId>,
    pub track_guessed_by: Option<UserId>,
}

impl MusicQuizHandle {
    pub fn new(quiz: MusicQuiz) -> Self {
        Self {
            quiz: Arc::new(Mutex::new(quiz)),
        }
    }

    pub async fn start_round(&self, track: QuizTrack) {
        let mut quiz = self.quiz.lock().await;
        quiz.session_mut().start_round(track);
    }

    pub async fn round_progress(&self) -> RoundProgress {
        let quiz = self.quiz.lock().await;

        RoundProgress {
            round_number: quiz.session().round_number(),
            total_rounds: quiz.session().total_rounds(),
        }
    }

    pub async fn round_completion(&self) -> Option<RoundCompletion> {
        let quiz = self.quiz.lock().await;
        let round = quiz.session().current_round()?;

        Some(RoundCompletion {
            track_name: round.track().name().to_string(),
            artist_name: round.track().artist().to_string(),
            artist_guessed_by: round.artist_guessed_by(),
            track_guessed_by: round.track_guessed_by(),
        })
    }

    pub async fn leaderboard(&self) -> Vec<(UserId, u32)> {
        let quiz = self.quiz.lock().await;
        quiz.session().get_leaderboard()
    }

    pub async fn guess(
        &self,
        user_id: UserId,
        guess: &str,
    ) -> Result<GuessOutcome, MusicQuizError> {
        let mut quiz = self.quiz.lock().await;
        quiz.make_guess(user_id, guess)
    }

    pub async fn is_finished(&self) -> bool {
        let quiz = self.quiz.lock().await;
        quiz.session().is_finished()
    }

    pub async fn notify_round_complete(&self) -> Arc<Notify> {
        let quiz = self.quiz.lock().await;
        quiz.notify_round_complete()
    }
}
