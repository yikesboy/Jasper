use super::MusicQuizHandle;
use crate::games::music_quiz::{QuizTrack, quiz::MusicQuiz};
use serenity::all::UserId;
use std::time::Duration;
use tokio::{
    sync::watch,
    time::{Instant, timeout},
};
use url::Url;

fn track() -> QuizTrack {
    QuizTrack::new(
        "Africa".into(),
        "Toto".into(),
        Url::parse("https://example.com/preview.mp3").unwrap(),
    )
}

async fn handle() -> MusicQuizHandle {
    let handle = MusicQuizHandle::new(MusicQuiz::new(2, vec![UserId::new(1)]));
    handle.start_round(track()).await;
    handle
}

async fn subscribe(handle: &MusicQuizHandle) -> watch::Receiver<bool> {
    handle
        .quiz
        .lock()
        .await
        .session()
        .current_round()
        .unwrap()
        .subscribe_completion()
}

#[tokio::test(start_paused = true)]
async fn completion_is_remembered_before_the_first_subscriber() {
    let handle = handle().await;
    handle.guess(UserId::new(1), "Toto Africa").await.unwrap();
    let started = Instant::now();
    for _ in 0..3 {
        timeout(Duration::from_millis(1), handle.wait_for_round_complete())
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(Instant::now(), started);
}

#[tokio::test(start_paused = true)]
async fn completion_between_subscription_and_first_poll_is_not_lost() {
    let handle = handle().await;
    let mut completion = subscribe(&handle).await;
    handle.guess(UserId::new(1), "Toto Africa").await.unwrap();
    let started = Instant::now();
    timeout(Duration::from_millis(1), completion.wait_for(|done| *done))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(Instant::now(), started);
}

#[tokio::test(start_paused = true)]
async fn all_current_and_late_waiters_observe_completion() {
    let handle = handle().await;
    let waits = async {
        tokio::join!(
            handle.wait_for_round_complete(),
            handle.wait_for_round_complete(),
            handle.guess(UserId::new(1), "Toto Africa")
        )
    };
    let (first, second, guess) = timeout(Duration::from_secs(1), waits).await.unwrap();
    first.unwrap();
    second.unwrap();
    guess.unwrap();
    timeout(Duration::from_millis(1), handle.wait_for_round_complete())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn partial_and_wrong_guesses_do_not_publish_completion() {
    let handle = handle().await;
    let completion = subscribe(&handle).await;
    handle.guess(UserId::new(1), "wrong").await.unwrap();
    handle.guess(UserId::new(1), "Africa").await.unwrap();
    assert!(!*completion.borrow());
    assert!(
        timeout(Duration::from_millis(1), handle.wait_for_round_complete())
            .await
            .is_err()
    );
    handle.guess(UserId::new(1), "Toto").await.unwrap();
    timeout(Duration::from_millis(1), handle.wait_for_round_complete())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn new_round_cannot_complete_a_subscription_to_an_old_unfinished_round() {
    let handle = handle().await;
    let mut old_completion = subscribe(&handle).await;
    handle.close_round().await.unwrap();
    handle.start_round(track()).await;
    handle.guess(UserId::new(1), "Toto Africa").await.unwrap();

    let old_result = timeout(
        Duration::from_millis(1),
        old_completion.wait_for(|done| *done),
    )
    .await
    .unwrap();
    assert!(old_result.is_err());
    timeout(Duration::from_millis(1), handle.wait_for_round_complete())
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn old_completed_subscription_does_not_finish_a_new_round() {
    let handle = handle().await;
    let old_completion = subscribe(&handle).await;
    handle.guess(UserId::new(1), "Toto Africa").await.unwrap();
    handle.close_round().await.unwrap();
    handle.start_round(track()).await;

    assert!(*old_completion.borrow());
    assert!(!*subscribe(&handle).await.borrow());
    assert!(
        timeout(Duration::from_millis(1), handle.wait_for_round_complete())
            .await
            .is_err()
    );
    handle.guess(UserId::new(1), "Toto Africa").await.unwrap();
    timeout(Duration::from_millis(1), handle.wait_for_round_complete())
        .await
        .unwrap()
        .unwrap();
}
