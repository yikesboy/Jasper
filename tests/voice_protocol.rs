//! Offline compatibility checks for the voice model re-exported by Songbird.
//! These guard against returning to a pre-DAVE dependency, but do not prove a
//! live Discord handshake or encrypted audio delivery.

use serde_json::json;
use songbird::model::{
    Event,
    id::{GuildId, UserId},
    payload::Identify,
};

#[test]
fn voice_identify_can_advertise_dave_support() {
    let identify = Event::Identify(Identify {
        server_id: GuildId(1),
        user_id: UserId(2),
        session_id: "test-session".into(),
        token: "test-token".into(),
        max_dave_protocol_version: Some(1),
    });

    let payload = serde_json::to_value(identify).unwrap();

    assert_eq!(payload["op"], 0);
    assert_eq!(payload["d"]["max_dave_protocol_version"], 1);
}

#[test]
fn voice_session_description_preserves_negotiated_dave_version() {
    let payload = json!({
        "op": 4,
        "d": {
            "mode": "aead_aes256_gcm_rtpsize",
            "secret_key": vec![0_u8; 32],
            "dave_protocol_version": 1
        }
    });

    // The voice model borrows JSON keys while decoding websocket text frames.
    let event: Event = serde_json::from_str(&payload.to_string()).unwrap();

    let Event::SessionDescription(description) = event else {
        panic!("expected a voice session description");
    };
    assert_eq!(description.dave_protocol_version, 1);
    assert_eq!(description.secret_key.len(), 32);
}
