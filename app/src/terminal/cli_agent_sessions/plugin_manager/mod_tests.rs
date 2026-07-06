use std::cmp::Ordering;

use super::{compare_versions, plugin_manager_for};
use crate::terminal::CLIAgent;
use warp_core::channel::{Channel, ChannelState};

#[test]
#[serial_test::serial]
fn returns_manager_for_claude() {
    let _channel_guard = ChannelState::override_channel_for_test(Channel::Stable);
    assert!(plugin_manager_for(CLIAgent::Claude).is_some());
}

#[test]
#[serial_test::serial]
fn returns_manager_for_opencode() {
    let _channel_guard = ChannelState::override_channel_for_test(Channel::Stable);
    let _oc_guard = crate::features::FeatureFlag::OpenCodeNotifications.override_enabled(true);
    let _hoa_guard = crate::features::FeatureFlag::HOANotifications.override_enabled(true);
    assert!(plugin_manager_for(CLIAgent::OpenCode).is_some());
}

#[test]
fn returns_manager_for_codex() {
    let _codex_guard = crate::features::FeatureFlag::CodexNotifications.override_enabled(true);
    let _hoa_guard = crate::features::FeatureFlag::HOANotifications.override_enabled(true);
    assert!(plugin_manager_for(CLIAgent::Codex).is_some());
}

#[test]
#[serial_test::serial]
fn returns_manager_for_gemini() {
    let _channel_guard = ChannelState::override_channel_for_test(Channel::Stable);
    let _gemini_guard = crate::features::FeatureFlag::GeminiNotifications.override_enabled(true);
    let _hoa_guard = crate::features::FeatureFlag::HOANotifications.override_enabled(true);
    assert!(plugin_manager_for(CLIAgent::Gemini).is_some());
}

#[test]
#[serial_test::serial]
fn local_only_returns_none_for_gemini_upstream_plugin() {
    let _channel_guard = ChannelState::override_channel_for_test(Channel::Oss);
    let _gemini_guard = crate::features::FeatureFlag::GeminiNotifications.override_enabled(true);
    let _hoa_guard = crate::features::FeatureFlag::HOANotifications.override_enabled(true);
    assert!(plugin_manager_for(CLIAgent::Gemini).is_none());
}

#[test]
#[serial_test::serial]
fn local_only_returns_none_for_claude_upstream_plugin() {
    let _channel_guard = ChannelState::override_channel_for_test(Channel::Oss);
    assert!(plugin_manager_for(CLIAgent::Claude).is_none());
}

#[test]
#[serial_test::serial]
fn local_only_returns_none_for_opencode_upstream_plugin() {
    let _channel_guard = ChannelState::override_channel_for_test(Channel::Oss);
    let _oc_guard = crate::features::FeatureFlag::OpenCodeNotifications.override_enabled(true);
    let _hoa_guard = crate::features::FeatureFlag::HOANotifications.override_enabled(true);
    assert!(plugin_manager_for(CLIAgent::OpenCode).is_none());
}

#[test]
fn returns_none_for_unsupported_agents() {
    assert!(plugin_manager_for(CLIAgent::Amp).is_none());
    assert!(plugin_manager_for(CLIAgent::Droid).is_none());
    assert!(plugin_manager_for(CLIAgent::Copilot).is_none());
    assert!(plugin_manager_for(CLIAgent::Unknown).is_none());
}

#[test]
fn compare_versions_equal() {
    assert_eq!(compare_versions("1.2.3", "1.2.3"), Ordering::Equal);
}

#[test]
fn compare_versions_less_than_major() {
    assert_eq!(compare_versions("1.0.0", "2.0.0"), Ordering::Less);
}

#[test]
fn compare_versions_less_than_minor() {
    assert_eq!(compare_versions("1.1.0", "1.2.0"), Ordering::Less);
}

#[test]
fn compare_versions_less_than_patch() {
    assert_eq!(compare_versions("1.1.0", "1.1.1"), Ordering::Less);
}

#[test]
fn compare_versions_greater_than() {
    assert_eq!(compare_versions("3.0.0", "2.0.0"), Ordering::Greater);
}

#[test]
fn compare_versions_unparseable_treated_as_zero() {
    assert_eq!(compare_versions("abc", "0.0.0"), Ordering::Equal);
    assert_eq!(compare_versions("abc", "1.0.0"), Ordering::Less);
}

#[test]
fn compare_versions_partial_version_string() {
    assert_eq!(compare_versions("2", "2.0.0"), Ordering::Equal);
    assert_eq!(compare_versions("2.1", "2.1.0"), Ordering::Equal);
}

#[test]
fn compare_versions_empty_string() {
    assert_eq!(compare_versions("", "2.0.0"), Ordering::Less);
}
