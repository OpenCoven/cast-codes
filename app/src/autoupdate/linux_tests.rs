use super::*;

#[test]
fn test_repo_name() {
    assert_eq!(repo_name(Channel::Dev), "warpdotdev-dev");
    assert_eq!(repo_name(Channel::Stable), "warpdotdev");
    assert_eq!(repo_name(Channel::Oss), "castcodes");
}

#[test]
fn test_package_name() {
    assert_eq!(package_name(Channel::Stable), "warp-terminal");
    assert_eq!(package_name(Channel::Oss), "cast-codes");
}

#[test]
fn oss_pacman_update_command_does_not_configure_warp_infrastructure() {
    let _guard = ChannelState::override_channel_for_test(Channel::Oss);
    let command = PackageManager::Pacman {
        is_repo_configured: false,
        is_signing_key_configured: false,
    }
    .update_command(&ShellType::Bash, "update_id");

    assert!(command.contains("sudo pacman -Sy cast-codes"));
    assert!(!command.contains("releases.warp.dev"));
    assert!(!command.contains("linux-maintainers@warp.dev"));
    assert!(!command.contains("warpdotdev"));
}
