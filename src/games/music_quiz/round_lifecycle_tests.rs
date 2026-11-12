use super::MusicQuizHandle;
use crate::games::music_quiz::{
    MusicQuizError, QuizTrack,
    quiz::{GuessOutcome, MusicQuiz},
};
use serenity::all::UserId;
use std::time::Duration;
use tokio::time::{Instant, advance};
use url::Url;

fn user(id: u64) -> UserId {
    UserId::new(id)
}

fn track(name: &str, artist: &str) -> QuizTrack {
    QuizTrack::new(
        name.into(),
        artist.into(),
        Url::parse("https://example.com/preview.mp3").unwrap(),
    )
}

async fn handle(total_rounds: u32) -> MusicQuizHandle {
    let handle = MusicQuizHandle::new(MusicQuiz::new(total_rounds, vec![user(1), user(2)]));
    handle.start_round(track("Africa", "Toto")).await;
    handle
}

#[tokio::test]
async fn closing_round_freezes_unanswered_fields_and_partial_winners() {
    for first in [None, Some("Africa"), Some("Toto")] {
        let handle = handle(2).await;
        if let Some(first) = first {
            handle.guess(user(1), first).await.unwrap();
        }
        let scores = handle.leaderboard().await;
        let result = handle.close_round().await.unwrap();

        for player in [user(1), user(2)] {
            for guess in ["Toto Africa", "Africa", "Toto", "wrong"] {
                assert!(matches!(
                    handle.guess(player, guess).await,
                    Err(MusicQuizError::RoundClosed)
                ));
            }
        }

        assert_eq!(handle.leaderboard().await, scores);
        assert_eq!(handle.round_completion().await.unwrap(), result);
        assert_eq!(handle.close_round().await.unwrap(), result);
    }
}

#[tokio::test(start_paused = true)]
async fn exact_deadline_rejects_guesses_even_without_the_runtime_timer() {
    let handle = handle(2).await;
    handle
        .set_round_deadline(Instant::now() + Duration::from_secs(25))
        .await;
    advance(Duration::from_millis(24999)).await;
    assert!(matches!(
        handle.guess(user(1), "Africa").await.unwrap(),
        GuessOutcome::Track { points: 1 }
    ));

    advance(Duration::from_millis(1)).await;
    assert!(matches!(
        handle.guess(user(2), "Toto").await,
        Err(MusicQuizError::RoundClosed)
    ));
    let result = handle.close_round().await.unwrap();
    assert_eq!(result.track_guessed_by, Some(user(1)));
    assert_eq!(result.artist_guessed_by, None);
    assert_eq!(handle.leaderboard().await, vec![(user(1), 1), (user(2), 0)]);
}

#[tokio::test(start_paused = true)]
async fn deadline_cannot_be_extended_by_rearming_the_round() {
    let handle = handle(2).await;
    handle
        .set_round_deadline(Instant::now() + Duration::from_secs(1))
        .await;
    handle
        .set_round_deadline(Instant::now() + Duration::from_secs(60))
        .await;
    advance(Duration::from_secs(1)).await;
    assert!(matches!(
        handle.guess(user(1), "Toto Africa").await,
        Err(MusicQuizError::RoundClosed)
    ));
    assert_eq!(handle.leaderboard().await, vec![(user(1), 0), (user(2), 0)]);
}

#[tokio::test]
async fn setting_deadline_does_not_reopen_an_explicitly_closed_round() {
    let handle = handle(2).await;
    let result = handle.close_round().await.unwrap();
    handle
        .set_round_deadline(Instant::now() + Duration::from_secs(60))
        .await;
    assert!(matches!(
        handle.guess(user(1), "Toto Africa").await,
        Err(MusicQuizError::RoundClosed)
    ));
    assert_eq!(handle.round_completion().await.unwrap(), result);
}

#[tokio::test]
async fn new_round_accepts_guesses_without_changing_the_previous_result() {
    let handle = handle(2).await;
    handle.guess(user(1), "Africa").await.unwrap();
    let previous = handle.close_round().await.unwrap();
    handle.start_round(track("Numb", "Linkin Park")).await;
    assert!(matches!(
        handle.guess(user(2), "Linkin Park Numb").await.unwrap(),
        GuessOutcome::Both { points: 3 }
    ));

    assert_eq!(previous.track_name, "Africa");
    assert_eq!(previous.track_guessed_by, Some(user(1)));
    assert_eq!(previous.artist_guessed_by, None);
    assert_eq!(handle.round_completion().await.unwrap().track_name, "Numb");
    assert_eq!(handle.leaderboard().await, vec![(user(2), 3), (user(1), 1)]);
}

#[tokio::test(start_paused = true)]
async fn final_scores_stay_frozen_while_cleanup_is_pending() {
    let handle = handle(1).await;
    assert!(handle.is_finished().await);
    handle.guess(user(1), "Africa").await.unwrap();
    handle.close_round().await.unwrap();
    let final_scores = handle.leaderboard().await;
    advance(Duration::from_secs(3)).await;
    assert!(matches!(
        handle.guess(user(2), "Toto Africa").await,
        Err(MusicQuizError::RoundClosed)
    ));
    assert_eq!(handle.leaderboard().await, final_scores);
}

#[tokio::test(start_paused = true)]
async fn queued_guess_checks_deadline_after_acquiring_the_scoring_mutex() {
    let handle = handle(2).await;
    handle
        .set_round_deadline(Instant::now() + Duration::from_secs(1))
        .await;
    let guard = handle.quiz.lock().await;
    let clone = handle.clone();
    let (started, waiting) = tokio::sync::oneshot::channel();
    let guess = tokio::spawn(async move {
        started.send(()).unwrap();
        clone.guess(user(1), "Toto Africa").await
    });
    waiting.await.unwrap();
    advance(Duration::from_secs(1)).await;
    drop(guard);

    assert!(matches!(
        guess.await.unwrap(),
        Err(MusicQuizError::RoundClosed)
    ));
    assert_eq!(handle.leaderboard().await, vec![(user(1), 0), (user(2), 0)]);
}

#[tokio::test]
async fn close_and_guess_ordering_produces_a_consistent_score_and_result() {
    for guess_first in [false, true] {
        let handle = handle(2).await;
        handle.guess(user(1), "Africa").await.unwrap();
        let (guess, result) = if guess_first {
            tokio::join!(handle.guess(user(2), "Toto"), handle.close_round())
        } else {
            let (result, guess) = tokio::join!(handle.close_round(), handle.guess(user(2), "Toto"));
            (guess, result)
        };
        let result = result.unwrap();
        let expected_artist = match guess {
            Ok(GuessOutcome::Artist { points: 1 }) => Some(user(2)),
            Err(MusicQuizError::RoundClosed) => None,
            _ => panic!("guess must either win before close or be rejected after it"),
        };
        assert_eq!(result.track_guessed_by, Some(user(1)));
        assert_eq!(result.artist_guessed_by, expected_artist);
        let second_score = u32::from(expected_artist.is_some());
        assert_eq!(
            handle.leaderboard().await,
            vec![(user(1), 1), (user(2), second_score)]
        );
        assert_eq!(handle.round_completion().await.unwrap(), result);
    }
}
