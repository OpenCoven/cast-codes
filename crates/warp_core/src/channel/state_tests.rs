use super::{derive_http_origin_from_ws_url, ChannelState};
use crate::brand;
use crate::channel::Channel;

#[test]
fn wss_becomes_https_and_strips_path() {
    let got = derive_http_origin_from_ws_url("wss://rtc.app.warp.dev/graphql/v2");
    assert_eq!(got.as_deref(), Some("https://rtc.app.warp.dev"));
}

#[test]
fn ws_becomes_http_and_preserves_port() {
    let got = derive_http_origin_from_ws_url("ws://localhost:8080/graphql/v2");
    assert_eq!(got.as_deref(), Some("http://localhost:8080"));
}

#[test]
fn unparseable_input_returns_none() {
    assert!(derive_http_origin_from_ws_url("not a url").is_none());
    assert!(derive_http_origin_from_ws_url("https://app.warp.dev").is_none());
}

#[test]
fn oss_channel_uses_castcodes_public_identity() {
    assert_eq!(Channel::Oss.cli_command_name(), "cast-codes");
    assert_eq!(Channel::Oss.to_string(), "cast-codes");
    assert_eq!(
        ChannelState::app_id().to_string(),
        "dev.castcodes.CastCodes"
    );
    assert_eq!(ChannelState::url_scheme(), "castcodes");
    assert!(!ChannelState::cloud_services_available());
    assert!(!ChannelState::is_telemetry_available());
    assert!(!ChannelState::is_crash_reporting_available());
    assert_eq!(ChannelState::releases_base_url(), "");
    assert_eq!(
        ChannelState::server_root_url(),
        brand::UNAVAILABLE_LOCALHOST_HTTP_URL
    );
    assert_eq!(
        ChannelState::oz_root_url(),
        brand::UNAVAILABLE_LOCALHOST_HTTP_URL
    );
    assert_eq!(ChannelState::firebase_api_key(), "");
    assert!(ChannelState::session_sharing_server_url().is_none());
}
