use super::{RoundEndReason, wait_for_round_completion_or_timeout};
use crate::games::music_quiz::{
    MusicQuizError, MusicQuizHandle, QuizTrack,
    quiz::{GuessOutcome, MusicQuiz},
};
use serenity::all::UserId;
use std::time::Duration;
use tokio::time::{Instant, advance};
use url::Url;

async fn handle() -> MusicQuizHandle {
    let handle = MusicQuizHandle::new(MusicQuiz::new(1, vec![UserId::new(1)]));
    handle
        .start_round(QuizTrack::new(
            "Africa".into(),
            "Toto".into(),
            Url::parse("https://example.com/preview.mp3").unwrap(),
        ))
        .await;
    handle
}

#[tokio::test(start_paused = true)]
async fn timeout_returns_a_closed_result_before_reveal_or_intermission() {
    let handle = handle().await;
    let (reason, result) = wait_for_round_completion_or_timeout(&handle).await;
    assert!(matches!(reason, RoundEndReason::TimedOut));
    let result = result.unwrap();
    assert_eq!(result.track_name, "Africa");
    assert_eq!(result.artist_name, "Toto");
    assert_eq!(result.track_guessed_by, None);
    assert_eq!(result.artist_guessed_by, None);

    for elapsed in [Duration::ZERO, Duration::from_secs(3)] {
        advance(elapsed).await;
        assert!(matches!(
            handle.guess(UserId::new(1), "Toto Africa").await,
            Err(MusicQuizError::RoundClosed)
        ));
    }
    assert_eq!(handle.leaderboard().await, vec![(UserId::new(1), 0)]);
    assert_eq!(handle.round_completion().await.unwrap(), result);
}

#[tokio::test(start_paused = true)]
async fn timeout_preserves_partial_scores_and_unanswered_fields() {
    let handle = handle().await;
    handle.guess(UserId::new(1), "Africa").await.unwrap();
    let (reason, result) = wait_for_round_completion_or_timeout(&handle).await;
    assert!(matches!(reason, RoundEndReason::TimedOut));
    let result = result.unwrap();
    assert_eq!(result.track_guessed_by, Some(UserId::new(1)));
    assert_eq!(result.artist_guessed_by, None);
    assert!(matches!(
        handle.guess(UserId::new(1), "Toto Africa").await,
        Err(MusicQuizError::RoundClosed)
    ));
    assert_eq!(handle.leaderboard().await, vec![(UserId::new(1), 1)]);
}

#[tokio::test(start_paused = true)]
async fn early_completion_also_closes_and_captures_the_winning_result() {
    let handle = handle().await;
    let started_at = Instant::now();
    let complete = async {
        tokio::task::yield_now().await;
        handle.guess(UserId::new(1), "Toto Africa").await.unwrap()
    };
    let ((reason, result), outcome) =
        tokio::join!(wait_for_round_completion_or_timeout(&handle), complete);
    assert!(matches!(outcome, GuessOutcome::Both { points: 3 }));
    assert!(matches!(reason, RoundEndReason::GuessedEarly));
    assert!(Instant::now() - started_at < Duration::from_secs(25));
    let result = result.unwrap();
    assert_eq!(result.track_guessed_by, Some(UserId::new(1)));
    assert_eq!(result.artist_guessed_by, Some(UserId::new(1)));
    assert!(matches!(
        handle.guess(UserId::new(1), "Toto Africa").await,
        Err(MusicQuizError::RoundClosed)
    ));
    assert_eq!(handle.leaderboard().await, vec![(UserId::new(1), 3)]);
}

#[tokio::test(start_paused = true)]
async fn completed_before_runtime_wait_does_not_consume_the_timeout() {
    let handle = handle().await;
    handle.guess(UserId::new(1), "Toto Africa").await.unwrap();
    let started = Instant::now();
    let (reason, result) = wait_for_round_completion_or_timeout(&handle).await;

    assert!(matches!(reason, RoundEndReason::GuessedEarly));
    assert_eq!(Instant::now(), started);
    let result = result.unwrap();
    assert_eq!(result.track_guessed_by, Some(UserId::new(1)));
    assert_eq!(result.artist_guessed_by, Some(UserId::new(1)));
    assert!(matches!(
        handle.guess(UserId::new(1), "Toto Africa").await,
        Err(MusicQuizError::RoundClosed)
    ));
}

#[tokio::test(start_paused = true)]
async fn separate_answers_completed_before_wait_are_also_remembered() {
    let handle = handle().await;
    handle.guess(UserId::new(1), "Africa").await.unwrap();
    handle.guess(UserId::new(1), "Toto").await.unwrap();
    let started = Instant::now();
    let (reason, _) = wait_for_round_completion_or_timeout(&handle).await;

    assert!(matches!(reason, RoundEndReason::GuessedEarly));
    assert_eq!(Instant::now(), started);
    assert_eq!(handle.leaderboard().await, vec![(UserId::new(1), 3)]);
}

#[tokio::test(start_paused = true)]
async fn previous_round_completion_cannot_short_circuit_the_next_round() {
    let handle = handle().await;
    handle.guess(UserId::new(1), "Toto Africa").await.unwrap();
    wait_for_round_completion_or_timeout(&handle).await;
    handle
        .start_round(QuizTrack::new(
            "Africa".into(),
            "Toto".into(),
            Url::parse("https://example.com/preview.mp3").unwrap(),
        ))
        .await;

    let started = Instant::now();
    let (reason, result) = wait_for_round_completion_or_timeout(&handle).await;
    assert!(matches!(reason, RoundEndReason::TimedOut));
    assert_eq!(Instant::now() - started, Duration::from_secs(25));
    let result = result.unwrap();
    assert_eq!(result.track_guessed_by, None);
    assert_eq!(result.artist_guessed_by, None);
    assert_eq!(handle.leaderboard().await, vec![(UserId::new(1), 3)]);
}

#[tokio::test(start_paused = true)]
async fn guess_at_timeout_cannot_publish_a_false_early_completion() {
    let handle = handle().await;
    let guess = async {
        tokio::time::sleep(Duration::from_secs(25)).await;
        handle.guess(UserId::new(1), "Toto Africa").await
    };
    let ((reason, result), guess) =
        tokio::join!(wait_for_round_completion_or_timeout(&handle), guess);
    assert!(matches!(reason, RoundEndReason::TimedOut));
    assert!(matches!(guess, Err(MusicQuizError::RoundClosed)));
    let result = result.unwrap();
    assert_eq!(result.track_guessed_by, None);
    assert_eq!(result.artist_guessed_by, None);
    assert_eq!(handle.leaderboard().await, vec![(UserId::new(1), 0)]);
}

#[tokio::test(start_paused = true)]
async fn missing_round_is_not_mistaken_for_a_successful_completion() {
    let handle = MusicQuizHandle::new(MusicQuiz::new(1, vec![UserId::new(1)]));
    let (reason, result) = wait_for_round_completion_or_timeout(&handle).await;
    assert!(matches!(reason, RoundEndReason::TimedOut));
    assert!(result.is_none());
}
