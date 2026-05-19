# CastCodes Mobile Control - Product Spec

## Summary

Build a Tailscale-only mobile companion for CastCodes that lets Val control and review coding-agent sessions from an iPhone while the actual work continues on a trusted desktop machine. The app is not a hosted chat product. It is a private tailnet client for CastCodes, Codex, Claude Code, and OpenClaw-compatible sessions exposed by a local gateway on the user's machine.

The first target is iOS/TestFlight. The architecture should leave room for Android later, but v1 should optimize for getting the iPhone workflow right.

## Product Position

The reference app proves the shape: a dark mobile workspace with session history, agent/task surfaces, project grouping, model selection, and a chat composer. CastCodes should use that pattern, but the trust model is different:
- No public relay.
- No hosted session sync.
- No remote agent execution on OpenCoven infrastructure.
- No broad internet exposure.
- The phone talks to the user's machine over Tailscale, the same boundary as the Control UI.

## Goals

- Show active and past coding sessions from the user's CastCodes host.
- Send prompts into active sessions.
- Start new sessions for supported runners: Codex, Claude Code, and OpenClaw.
- Surface agent status clearly: running, idle, blocked on permission, waiting for input, failed, complete.
- Support model/provider selection at session start where the underlying runner supports it.
- Preserve transcript history locally on the host and render it on mobile.
- Use Tailscale identity and local pairing as the security boundary.
- Feel like a focused CastCodes control surface, not a generic mobile chat app.

## Non-goals for v1

- Hosted relay, cloud sync, web login, or push notifications through OpenCoven servers.
- Editing files directly from mobile.
- Full terminal emulation.
- Mobile-side agent execution.
- Rebuilding Codex, Claude Code, or OpenClaw internals.
- Public sharing links.
- Cross-platform Android release on day one.

## Core Screens

### Home

Dark first screen modeled after the reference:

- CastCodes brand header.
- Search button.
- Host status chip showing the paired machine and Tailscale reachability.
- Navigation rows for Sessions, Tasks, Skills, Memory, Insights, and Projects.
- Project folders with counts.
- Recent sessions list with title, runner, project/workspace, message count, status, and last activity.
- Floating Chat / New Session button.

### Session Detail

Chat-style transcript view for one coding session:
- Header with back, session title, runner badge, project/folder action.
- Transcript bubbles for user prompts and agent responses.
- Compact cards for tool calls, file edits, command execution, permission prompts, and errors.
- Composer with attachments/context button, model/run-mode indicator, voice dictation if the OS provides it, and send.
- Blocked state controls that make it obvious when the desktop host needs approval.

The app should not pretend it can approve dangerous desktop actions silently. v1 can display permission requests and allow simple textual replies. Actual command/file approvals should remain constrained by the host gateway's policy.

### New Session

Start a session on the paired host:
- Choose runner: Codex, Claude Code, OpenClaw.
- Choose project/workspace from host-discovered roots.
- Choose model/profile from runner-discovered capabilities.
- Optional initial prompt.
- Start creates the process/session on the host and immediately opens the transcript.

### Models

Bottom sheet similar to the reference:
- Group by runner/provider.
- Search models.
- Favorite models.
- Show exact runner model id.
- Hide unsupported options for the selected runner.

### Host Pairing

Pairing is local-first:
- User enables Mobile Control in CastCodes desktop.
- Desktop shows a QR code containing the Tailscale MagicDNS/base URL plus a short-lived pairing token.
- Mobile scans it, verifies host name/device identity, and stores a scoped credential in Keychain.
- The credential can be revoked from desktop settings.

## Security Model
- Transport is HTTPS over Tailscale only.
- Host binds the mobile-control server to the tailnet address or localhost plus Tailscale serve, never 0.0.0.0 on public networks.
- Pairing tokens are short-lived and single-use.
- Mobile credentials are scoped to mobile-control APIs, not shell access.
- Host validates both the pairing credential and Tailscale peer identity.
- Sensitive payloads in logs are redacted.
- No analytics or telemetry in v1.

## Success Criteria
- From iPhone on the same tailnet, Val can pair to a CastCodes host by QR code.
- She can see active Codex, Claude Code, and OpenClaw-compatible sessions in one list.
- She can open a session, read the transcript, and send a follow-up prompt.
- She can start a new Codex or Claude Code session against a chosen project and model.
- If Tailscale is disconnected, the app shows a clear offline state without data loss.
- No API is reachable from the public internet in the default setup.

## Open Questions

- Whether OpenClaw sessions should be proxied through the existing Coven Gateway API shape or exposed through a CastCodes-normalized adapter first.
- Whether mobile can approve low-risk permission prompts in v1, or only display them and ask the user to approve on desktop.
- Whether transcript persistence should live in CastCodes' existing CLI chat sqlite store or a new mobile-control store with a stable API facade.
