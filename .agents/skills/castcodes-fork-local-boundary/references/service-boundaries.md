# Hosted Service Boundary Map

## Core Channel State

- `crates/warp_core/src/channel/state.rs`: public OSS defaults, telemetry/crash availability, cloud availability.
- `crates/warp_core/src/channel/config.rs`: server, Oz, telemetry, crash, autoupdate, and MCP config shapes.
- `app/src/bin/oss.rs`: public wrapper that installs unavailable server and Oz configs.
- `app/src/bin/{local,dev,preview,stable}.rs`: channel-config backed wrappers for internal channels.

## Common Hosted Surfaces

- Auth: `app/src/auth/**`
- Server API and GraphQL: `app/src/server/**`, `crates/graphql/**`
- Cloud objects and Drive: `app/src/cloud_object/**`, `app/src/drive/**`
- Oz and agent cloud mode: `app/src/ai/**`, `crates/ai/**`, `crates/warp_cli/src/agent.rs`
- Shared sessions: `app/src/terminal/shared_session/**`, `crates/warp_terminal/src/shared_session*`
- Telemetry: `app/src/server/telemetry/**`, `crates/warp_core/src/telemetry.rs`, `crates/warpui/**`
- Crash reporting: `app/src/crash_reporting/**`, `app/build.rs`, Sentry scripts
- Autoupdate and release feeds: `app/src/autoupdate/**`, channel version crates, platform bundle scripts
- Billing and pricing: `app/src/billing/**`, `app/src/pricing/**`, onboarding billing state

## Search Patterns

```bash
rg -n "app\\.warp\\.dev|warp\\.dev|warpdotdev|Sentry|RudderStack|billing|telemetry|crash|cloud|shared_session|run-cloud|Oz|autoupdate|release" \
  app/src crates script app/Cargo.toml Cargo.toml
```

After finding a candidate, inspect the caller chain. The bug is often not the string itself; it is exposing a public entrypoint that reaches a configured upstream service.

## OSS Behavior Rules

- Hosted auth or account flows: hide or mark unavailable.
- Hosted telemetry/crash controls: hide when config is absent.
- Hosted release feeds: disable update checks unless CastCodes release infrastructure is configured.
- Shared sessions and cloud agents: hide entrypoints in public OSS unless a local-only implementation exists.
- Billing/pricing: remove from public workflows or clearly show unavailable state.
- Tests should assert no service call is attempted from `Channel::Oss` when practical.
