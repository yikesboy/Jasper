use super::{GameState, GameStateError};
use crate::games::music_quiz::{MusicQuizHandle, QuizTrack, quiz::MusicQuiz};
use dashmap::DashMap;
use serenity::{
    all::{ChannelId, GuildId, Message, MessageId, UserId},
    http::HttpBuilder,
};
use std::{
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    time::timeout,
};
use url::Url;

const CHILD_TEST: &str = "JASPER_REGISTRY_LOCK_CHILD_TEST";

// Tokio timeouts cannot fire if a blocking map write freezes the only runtime
// thread. Run each regression case in a subprocess with an external watchdog.
fn bounded_single_thread_test(name: &str, correct_guess: bool, http_failure: bool) {
    if std::env::var(CHILD_TEST).as_deref() == Ok(name) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(pending_http_allows_registry_writes(
                correct_guess,
                http_failure,
            ));
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!("games::state::tests::{name}"),
            "--nocapture",
        ])
        .env(CHILD_TEST, name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let output = child.wait_with_output().unwrap();
            assert!(
                status.success(),
                "child failed:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let output = child.wait_with_output().unwrap();
            panic!(
                "registry blocked the single-thread runtime:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

async fn quiz(title: &str) -> MusicQuizHandle {
    let quiz = MusicQuizHandle::new(MusicQuiz::new(1, vec![UserId::new(1)]));
    quiz.start_round(QuizTrack::new(
        title.into(),
        "Toto".into(),
        Url::parse("https://example.com/preview.mp3").unwrap(),
    ))
    .await;
    quiz
}

fn message(guild_id: GuildId, content: &str) -> Message {
    let mut message = Message::default();
    message.id = MessageId::new(3);
    message.channel_id = ChannelId::new(2);
    message.guild_id = Some(guild_id);
    message.author.id = UserId::new(1);
    message.content = content.into();
    message
}

fn colliding_guild(state: &GameState, guild_id: GuildId) -> GuildId {
    let guard = state.games.get(&guild_id).unwrap();
    // The fixture has just two shards. Find a second guild in the same shard
    // using nonblocking probes, without enabling DashMap's raw API feature.
    let collision = (2..1024)
        .map(GuildId::new)
        .find(|candidate| state.games.try_get_mut(candidate).is_locked())
        .expect("expected a colliding guild in the two-shard test map");
    drop(guard);
    collision
}

async fn pending_http_allows_registry_writes(correct_guess: bool, http_failure: bool) {
    let state = Arc::new(GameState {
        games: DashMap::with_shard_amount(2),
    });
    let guild_id = GuildId::new(1);
    let original = quiz("Africa").await;
    state.start_quiz(guild_id, original.clone()).unwrap();
    let collision = colliding_guild(&state, guild_id);
    let message = message(
        guild_id,
        if correct_guess {
            "Toto Africa"
        } else {
            "wrong"
        },
    );
    let response_body = if http_failure {
        r#"{"code":50013,"message":"Missing Permissions"}"#.to_string()
    } else if correct_guess {
        serde_json::to_string(&message).unwrap()
    } else {
        String::new()
    };
    let status = if http_failure {
        403
    } else if correct_guess {
        200
    } else {
        204
    };

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    let http = HttpBuilder::new("test-token")
        .client(
            reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
        )
        .proxy(proxy)
        .ratelimiter_disabled(true)
        .build();
    let (received, pending) = oneshot::channel();
    let (release, released) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut buffer = [0_u8; 4096];
            let count = stream.read(&mut buffer).await.unwrap();
            assert!(count > 0, "request ended before its body was complete");
            bytes.extend_from_slice(&buffer[..count]);
            assert!(bytes.len() < 65536);
            if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
                let length = headers
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .map(|value| value.trim().parse::<usize>().unwrap())
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + length {
                    break;
                }
            }
        }
        received.send(String::from_utf8(bytes).unwrap()).unwrap();
        released.await.unwrap();
        let response = format!(
            "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });
    let handler_state = Arc::clone(&state);
    let handler = tokio::spawn(async move { handler_state.handle_message(&http, &message).await });
    let request = timeout(Duration::from_secs(5), pending)
        .await
        .unwrap()
        .unwrap();
    if correct_guess {
        assert!(request.starts_with("POST "));
        assert!(request.contains("/channels/2/messages"));
    } else {
        assert!(request.starts_with("PUT "));
        assert!(request.contains("/reactions/"));
    }
    assert!(
        !handler.is_finished(),
        "HTTP response must still be pending"
    );

    // Each operation needs the shard's write lock. A map read guard retained by
    // the message task would deadlock this runtime before HTTP can be released.
    state
        .start_quiz(collision, quiz("Other guild").await)
        .unwrap();
    state.end_game(collision);
    assert!(matches!(
        state.start_quiz(guild_id, quiz("Duplicate").await),
        Err(GameStateError::GameAlreadyActiveInServer)
    ));
    state.end_game(guild_id);
    let replacement = quiz("Replacement").await;
    state.start_quiz(guild_id, replacement).unwrap();

    release.send(()).unwrap();
    timeout(Duration::from_secs(5), handler)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    server.await.unwrap();

    let active = state.games.get(&guild_id).unwrap().value().clone();
    assert_eq!(
        active.round_completion().await.unwrap().track_name,
        "Replacement"
    );
    assert_eq!(active.leaderboard().await, vec![(UserId::new(1), 0)]);
    assert_eq!(
        original.leaderboard().await,
        vec![(UserId::new(1), if correct_guess { 3 } else { 0 })]
    );
}

#[test]
fn pending_reaction_does_not_hold_registry_lock() {
    bounded_single_thread_test("pending_reaction_does_not_hold_registry_lock", false, false);
}

#[test]
fn pending_reply_does_not_hold_registry_lock() {
    bounded_single_thread_test("pending_reply_does_not_hold_registry_lock", true, false);
}

#[test]
fn failed_reaction_does_not_block_cleanup_or_replacement() {
    bounded_single_thread_test(
        "failed_reaction_does_not_block_cleanup_or_replacement",
        false,
        true,
    );
}

#[test]
fn failed_reply_does_not_block_cleanup_or_replacement() {
    bounded_single_thread_test(
        "failed_reply_does_not_block_cleanup_or_replacement",
        true,
        true,
    );
}
