# Release Configurations

This README file documents the format of the `release_configurations.json` file located in this directory. The file defines CastCodes release channels and provides values used by the `create_release.yml` GitHub workflow.

The public OSS release lane is GitHub-only. It should not depend on upstream Warp release services, Sentry, Slack, Azure signing, or private channel configuration. Publishing a Gatekeeper-valid macOS DMG also requires the Apple Developer ID signing and notarization secrets consumed by `create_release.yml`; dry runs and non-macOS release assets require only `GITHUB_TOKEN`.

The standard OSS release asset set is intentionally small: macOS arm64 DMG and CLI tarball, Linux x86_64 app and CLI packages, Windows x64 installer, and web bundle. Intel macOS, universal macOS, Linux ARM64, and Windows ARM64 assets are excluded from the default release lane.

At some point, we may want to replace this document with a JSON schema file (which could be used to validate the correctness of the configuration as part of PR presubmit).

## Fields

* **channel**: The channel's unique identifier
* **type**: The release cadence. At present, the valid values are "nightly", "weekly", or "manual".
* **is_prerelease**: If true, the GitHub release for this channel will be marked as prerelease.
* **is_autopush**: If true, this channel uses the "latest" keyword in `channel_versions.json` to automatically deploy new release candidates.  Non-autopush channels require a manual change in order to deploy them.
* **release_base_name**: The base name of GitHub releases created for this channel.
* **release_body_text**: The body text for GitHub releases created for this channel.
* **sentry_project**: Which Sentry project should receive crash and error reports for this channel.
* **sentry_environment**: The Sentry environment that corresponds to this channel.
* **changelog_slack_channel**: The Slack channel where new changelogs will be posted whenever a new release candidates is cut.
