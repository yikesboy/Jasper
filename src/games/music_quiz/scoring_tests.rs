use super::{GuessOutcome, MusicQuiz};
use crate::games::music_quiz::{MusicQuizHandle, QuizTrack};
use serenity::all::UserId;
use url::Url;

fn user(id: u64) -> UserId {
    UserId::new(id)
}

fn quiz() -> MusicQuiz {
    let mut quiz = MusicQuiz::new(1, vec![user(1), user(2), user(3)]);
    quiz.session_mut().start_round(QuizTrack::new(
        "Africa".into(),
        "Toto".into(),
        Url::parse("https://example.com/preview.mp3").unwrap(),
    ));
    quiz
}

#[test]
fn partial_round_completion_scores_consistently_for_both_orders_and_guess_formats() {
    for track_first in [false, true] {
        for same_user in [false, true] {
            for combined in [false, true] {
                let mut quiz = quiz();
                let (first, remaining) = if track_first {
                    ("Africa", "Toto")
                } else {
                    ("Toto", "Africa")
                };
                let second_user = if same_user { user(1) } else { user(2) };
                let first_outcome = quiz.make_guess(user(1), first).unwrap();
                assert!(match first_outcome {
                    GuessOutcome::Track { points: 1 } => track_first,
                    GuessOutcome::Artist { points: 1 } => !track_first,
                    _ => false,
                });
                assert!(!quiz.is_round_complete());

                let second = if combined { "Toto Africa" } else { remaining };
                let outcome = quiz.make_guess(second_user, second).unwrap();
                let expected_points = if same_user { 2 } else { 1 };
                assert!(match outcome {
                    GuessOutcome::Artist { points } => track_first && points == expected_points,
                    GuessOutcome::Track { points } => !track_first && points == expected_points,
                    _ => false,
                });

                let round = quiz.session().current_round().unwrap();
                assert_eq!(
                    round.track_guessed_by(),
                    Some(if track_first { user(1) } else { second_user })
                );
                assert_eq!(
                    round.artist_guessed_by(),
                    Some(if track_first { second_user } else { user(1) })
                );
                assert!(quiz.is_round_complete());
                let expected = if same_user {
                    vec![(user(1), 3), (user(2), 0), (user(3), 0)]
                } else {
                    vec![(user(1), 1), (user(2), 1), (user(3), 0)]
                };
                assert_eq!(quiz.session().get_leaderboard(), expected);
            }
        }
    }
}

#[test]
fn repeated_combined_guesses_cannot_farm_points_or_replace_either_winner() {
    for first in ["Africa", "Toto"] {
        let mut quiz = quiz();
        quiz.make_guess(user(1), first).unwrap();
        quiz.make_guess(user(2), "Toto Africa").unwrap();
        let expected_scores = quiz.session().get_leaderboard();
        let round = quiz.session().current_round().unwrap();
        let expected_winners = (round.track_guessed_by(), round.artist_guessed_by());

        for _ in 0..100 {
            for user in [user(1), user(2), user(3)] {
                for guess in ["Toto Africa", "Africa Toto", "Africa", "Toto"] {
                    assert!(matches!(
                        quiz.make_guess(user, guess).unwrap(),
                        GuessOutcome::AlreadyGuessed
                    ));
                }
            }
        }

        assert!(quiz.is_round_complete());
        assert_eq!(quiz.session().get_leaderboard(), expected_scores);
        assert_eq!(
            expected_scores.iter().map(|(_, score)| score).sum::<u32>(),
            2
        );
        let round = quiz.session().current_round().unwrap();
        assert_eq!(
            (round.track_guessed_by(), round.artist_guessed_by()),
            expected_winners
        );
    }
}

#[test]
fn repeated_partial_answers_do_not_award_points_before_completion() {
    for first in ["Africa", "Toto"] {
        let mut quiz = quiz();
        quiz.make_guess(user(1), first).unwrap();
        for _ in 0..100 {
            for user in [user(1), user(2), user(3)] {
                assert!(matches!(
                    quiz.make_guess(user, first).unwrap(),
                    GuessOutcome::AlreadyGuessed
                ));
            }
        }
        assert!(!quiz.is_round_complete());
        assert_eq!(
            quiz.session().get_leaderboard(),
            vec![(user(1), 1), (user(2), 0), (user(3), 0)]
        );
        quiz.make_guess(user(1), "Toto Africa").unwrap();
        assert!(quiz.is_round_complete());
        assert_eq!(
            quiz.session().get_leaderboard(),
            vec![(user(1), 3), (user(2), 0), (user(3), 0)]
        );
    }
}

#[tokio::test]
async fn competing_combined_guesses_award_only_the_remaining_answer() {
    for track_first in [false, true] {
        let handle = MusicQuizHandle::new(quiz());
        handle
            .guess(user(1), if track_first { "Africa" } else { "Toto" })
            .await
            .unwrap();
        let other = handle.clone();

        let (second, third) = tokio::join!(
            handle.guess(user(2), "Toto Africa"),
            other.guess(user(3), "Toto Africa")
        );
        let mut winner = None;
        for (user, outcome) in [(user(2), second.unwrap()), (user(3), third.unwrap())] {
            match outcome {
                GuessOutcome::Artist { points: 1 } if track_first => {
                    assert!(winner.replace(user).is_none());
                }
                GuessOutcome::Track { points: 1 } if !track_first => {
                    assert!(winner.replace(user).is_none());
                }
                GuessOutcome::AlreadyGuessed => {}
                _ => panic!("only the previously unanswered field should earn a point"),
            }
        }
        let winner = winner.expect("one competing guess must win the remaining answer");
        let round = handle.round_completion().await.unwrap();
        assert_eq!(
            round.track_guessed_by,
            Some(if track_first { user(1) } else { winner })
        );
        assert_eq!(
            round.artist_guessed_by,
            Some(if track_first { winner } else { user(1) })
        );
        let scores = handle.leaderboard().await;
        assert_eq!(scores.iter().map(|(_, score)| score).sum::<u32>(), 2);
        assert_eq!(scores.iter().find(|(id, _)| *id == user(1)).unwrap().1, 1);
        assert_eq!(scores.iter().find(|(id, _)| *id == winner).unwrap().1, 1);
    }
}
