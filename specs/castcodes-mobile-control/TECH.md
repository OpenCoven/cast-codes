# CastCodes Mobile Control - Technical Spec

Companion to PRODUCT.md. This spec chooses an iOS/TestFlight-first app backed by a Tailscale-only host gateway.

## Recommendation

Build v1 as:

- Native iOS SwiftUI app for speed to TestFlight, strong Keychain/QR/Tailscale ergonomics, and a polished iPhone-first UI.
- Host-side CastCodes Mobile Control service running on the desktop machine.
- Tailscale-only network path, matching the Control UI pattern.
- Adapter boundary per runner so Codex, Claude Code, and OpenClaw can ship together without forcing one internal session model.

Avoid React Native/Expo for v1. It becomes attractive once Android is real, but the first problem is the host protocol and private-control trust boundary. SwiftUI keeps the mobile side thinner and makes TestFlight iteration straightforward.

## System Shape

Phone CastCodes app -> HTTPS over Tailscale -> CastCodes Mobile Control service on desktop.

The desktop service connects to:

- CastCodes local session/transcript store.
- Codex runner adapter.
- Claude Code runner adapter.
- OpenClaw Gateway/session adapter.
- Project/workspace discovery.

The phone never shells out and never talks directly to vendor CLIs. It calls a narrow host API. The host owns process lifecycle, transcript persistence, permission policy, and local filesystem access.

## Host Service

Add a host-side service to CastCodes, behind an explicit setting:

- app/src/mobile_control/mod.rs
- app/src/mobile_control/server.rs for local HTTPS server lifecycle.
- app/src/mobile_control/auth.rs for pairing token and credential validation.
- app/src/mobile_control/tailscale.rs for tailnet address and MagicDNS detection helpers.
- app/src/mobile_control/api.rs for route definitions and DTOs.
- app/src/mobile_control/sessions.rs for the normalized session facade.
- app/src/mobile_control/runners/codex.rs
- app/src/mobile_control/runners/claude.rs
- app/src/mobile_control/runners/openclaw.rs
- app/src/mobile_control/pairing.rs for QR payloads and revocation.

The service is off by default. Enabling it starts a listener reachable only through the configured tailnet path.

## API v1

Use JSON over HTTPS plus a streaming endpoint for live events.

- GET /v1/health
- GET /v1/host
- POST /v1/pair/claim
- GET /v1/projects
- GET /v1/runners
- GET /v1/models?runner=codex
- GET /v1/sessions
- POST /v1/sessions
- GET /v1/sessions/{id}
- POST /v1/sessions/{id}/messages
- GET /v1/sessions/{id}/events
- POST /v1/sessions/{id}/stop

The events endpoint can start as Server-Sent Events. WebSocket is acceptable later if bidirectional streaming becomes necessary, but SSE is simpler for mobile transcript updates.

## Normalized Session Model

Each runner adapter maps native state into one facade:

- runner: Codex, ClaudeCode, or OpenClaw.
- status: Running, Idle, WaitingForInput, WaitingForPermission, Complete, Failed, or Stopped.
- summary fields: id, title, project, cwd, message_count, updated_at_ms.
- transcript entries: user message, assistant message, tool call, file edit, command, permission request, status event, error.

The mobile app should not need to know whether a transcript came from OSC 777 CLI-agent events, Codex rollout history, Claude session metadata, or OpenClaw session APIs.

## Runner Adapters

### Codex

- Discover installed codex binary and version.
- List supported models from local config or a curated table.
- Start sessions by opening/running Codex on the host side.
- Capture transcript/status through the existing CastCodes CLI chat event path where possible.

### Claude Code

- Discover installed claude binary and version.
- Respect Claude's own auth and model handling.
- Use existing plugin/event integration where available.
- Resume sessions only through Claude-supported resume flags.

### OpenClaw

- Treat OpenClaw as a separate adapter, not a hard dependency of CastCodes desktop UI.
- Prefer connecting to a local Coven/OpenClaw Gateway already running on the host.
- Normalize visible sessions, messages, and status into the same mobile facade.
- Keep this adapter configurable and disable it cleanly when the gateway is absent.

## iOS App Structure

- CastCodesMobile/App/CastCodesMobileApp.swift
- CastCodesMobile/Core/APIClient.swift
- CastCodesMobile/Core/EventStreamClient.swift
- CastCodesMobile/Core/KeychainStore.swift
- CastCodesMobile/Core/PairingStore.swift
- CastCodesMobile/Core/Models.swift
- CastCodesMobile/Features/Pairing/
- CastCodesMobile/Features/Home/
- CastCodesMobile/Features/Sessions/
- CastCodesMobile/Features/NewSession/
- CastCodesMobile/Features/Models/
- CastCodesMobile/Features/Settings/
- CastCodesMobile/Design/Theme.swift
- CastCodesMobile/Design/Components/

Use Swift concurrency for API calls and URLSession.bytes for SSE. Store pairing credentials in Keychain. Cache session summaries locally enough to render the last known state while offline.

## Tailscale Setup

Preferred v1 path:

1. Desktop CastCodes detects tailnet hostname/address.
2. CastCodes starts the mobile-control HTTPS service.
3. CastCodes offers a QR payload with version, host, baseUrl, single-use pairingToken, and expiresAt.
4. iOS scans the QR, calls /v1/pair/claim, and stores the returned mobile credential.

If Tailscale Serve or certificate constraints make host certificates awkward, use a CastCodes-generated local CA/cert pinned by the pairing flow. Do not fall back to a public relay.

## Implementation Phases

### Phase 1 - Host Protocol Skeleton

- Add mobile-control module and feature flag.
- Implement health, host metadata, pairing, and static fake session list.
- Verify service binds only to the intended Tailscale/local path.

### Phase 2 - iOS Shell

- Create SwiftUI app.
- Implement pairing, offline state, home/session list, and session detail against fake host data.
- Ship internal TestFlight build.

### Phase 3 - Real Sessions

- Wire Codex and Claude Code adapters to host session discovery and transcript streaming.
- Implement send-message and start-session flows.
- Add model picker from adapter-provided capabilities.

### Phase 4 - OpenClaw Adapter

- Add configurable OpenClaw Gateway adapter.
- List sessions and stream messages through the normalized API.
- Start OpenClaw-compatible sessions if the local gateway exposes a safe API for that.

### Phase 5 - Hardening

- Pairing revocation UI.
- Permission prompt policy.
- Host logs with redaction.
- SSE reconnect and mobile cache cleanup.
- Manual security checklist for not-public-internet reachability.

## Verification

- Desktop: unit tests for auth token expiry, session facade mapping, and runner availability parsing.
- Desktop: integration smoke that service refuses unauthenticated requests.
- Desktop: manual network check that listener is not exposed outside Tailscale/local path.
- iOS: pairing flow tests, API client tests with fixture responses, and TestFlight smoke on real tailnet.
- End-to-end: start Codex and Claude Code sessions from mobile, send follow-up prompts, disconnect Tailscale, reconnect, and verify transcript recovery.
