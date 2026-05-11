# CastCodes UI Map

## Shared UI Primitives

- `app/src/view_components/**`: app-level reusable controls.
- `crates/ui_components/**`: lower-level shared UI controls and themes.
- `app/src/themes/**`: themes and theme editor surfaces.
- `app/src/menu.rs`, `app/src/palette.rs`, `app/src/search/**`: menus, palette, and search surfaces.

## High-Risk CastCodes Surfaces

- Onboarding and auth: `app/src/auth/**`, `crates/onboarding/**`, `app/src/workspace/one_time_modal_model.rs`
- Settings: `app/src/settings_view/**`, especially privacy, billing, MCP servers, environments, and feature pages.
- Resource center and changelog: `app/src/resource_center/**`, `app/src/changelog_model.rs`, `app/src/autoupdate/**`
- Terminal commands and slash commands: `app/src/terminal/input/**`, `app/src/search/slash_command_menu/**`
- Shared sessions: `app/src/terminal/shared_session/**`, `app/src/terminal/view/shared_session/**`
- Agent and cloud surfaces: `app/src/ai/**`, `app/src/workspace/view/**`, `app/src/terminal/view/ambient_agent/**`

## Review Questions

- Is this user-facing text public CastCodes text or an inherited internal identifier?
- Does the action work in public OSS without upstream infrastructure?
- If the feature is unavailable, is it hidden, disabled, or clearly marked?
- Does the UI reuse existing components and themes?
- Is the command/menu/search entry consistent with channel availability?

## Useful Checks

```bash
rg -n "ActionButtonTheme|PrimaryTheme|SecondaryTheme|NakedTheme|DangerPrimaryTheme" app/src crates/ui_components
rg -n "ChannelState::cloud_services_available|is_telemetry_available|is_crash_reporting_available" app/src crates
rg -n "Warp|CastCodes|sign in|billing|cloud|telemetry|crash|shared session" app/src crates/onboarding
```
