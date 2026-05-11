---
name: castcodes-fork-local-boundary
description: Preserve CastCodes fork-local OSS behavior. Use when touching auth, cloud or Oz flows, telemetry, crash reporting, billing, autoupdate, release feeds, shared sessions, feedback, upstream URLs, hosted API clients, or any feature that could call Warp-owned infrastructure from the public CastCodes build.
---

# castcodes-fork-local-boundary

## Workflow

Treat the public CastCodes OSS build as fork-local by default. It must not call upstream hosted services unless CastCodes-owned infrastructure is intentionally added later.

Before editing a hosted-service seam:

1. Read `CASTCODES.md`.
2. Inspect `references/service-boundaries.md` for the likely code path.
3. Confirm whether the behavior is gated by `ChannelState`, a Cargo feature, or channel config.
4. Make the OSS behavior explicit: hide the UI, disable the command, or mark the service unavailable.

Do not make public OSS builds silently fall through to upstream auth, cloud APIs, release feeds, Sentry, RudderStack, billing, shared sessions, hosted telemetry, or hosted crash reporting.

## Preferred Seams

Use existing channel state and config seams before adding new global flags:

- `ChannelState::cloud_services_available()` for hosted cloud/service availability.
- `ChannelState::is_telemetry_available()` for telemetry UI and event-sending surfaces.
- `ChannelState::is_crash_reporting_available()` for crash reporting UI and reporting surfaces.
- `WarpServerConfig::unavailable()` and `OzConfig::unavailable()` for OSS channel config.
- `Channel::Oss` and the `cast-codes` binary wrapper for public build behavior.

If a feature is unavailable in OSS, keep local terminal and code workflows working. Remove dead affordances rather than leaving clickable UI that fails after network or auth work begins.

## Verification

For boundary changes, run the smallest relevant test plus:

```bash
./script/check_rebrand
cargo test -p warp_core --features local_fs castcodes
cargo check -p warp --bin cast-codes --features gui
```

If the change touches UI or command exposure, add or update a focused test that proves the OSS channel hides, disables, or reports unavailable state without attempting the upstream call.

## Related Skills

- Use `castcodes-rebrand-surface` for public URLs and copy.
- Use `castcodes-ui-work` for user-facing unavailable states.
- Use `castcodes-dev-loop` for command selection and verification.
