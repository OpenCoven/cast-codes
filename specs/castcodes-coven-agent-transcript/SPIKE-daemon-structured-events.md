# Spike: structured-event source for coven-code (Phase 1 top risk)

- **Date:** 2026-07-17
- **Question:** Can `cast_agent` obtain structured events (tool calls, permissions, assistant messages) from a `coven-code` daemon session for rich rendering — and does it require an upstream Coven daemon/harness change?
- **Verdict:** ✅ **Resolved favorably. No upstream change needed.**

## Method

Investigated the live daemon (`~/.coven/coven.sock`), the `coven` CLI, its native binary strings, and the on-disk adapter manifests — all read-only. Did **not** launch a live agent (harnesses run in `danger-full-access` sandboxes; launching blindly is unsafe).

## Findings

1. **The daemon records only raw PTY `output` today.** A completed `codex` session had 400/400 events of `kind: "output"` carrying human-readable PTY text (`{"data":"OpenAI Codex v0.144.4\r\n…"}`). Of 817 sessions, **0** had a populated `transcript_path`; no `.jsonl` transcripts on disk. So with the current `launchMode: "nonInteractive"` there are no structured events — this is exactly why coven-code renders as plain text.

2. **The daemon supports a third launch mode: `stream`.** The native binary enforces `launchMode` ∈ {`interactive`, `nonInteractive`, `stream`}. In `stream` mode it runs the harness with its adapter `stream_args` and forwards the harness's JSONL — evidenced by strings such as *"write stream-json message to live session"*, *"forwarding claude stdout"*, *"failed to encode stream-json user envelope"*.

3. **`coven-code` natively supports stream mode** (`~/.coven/adapters/coven-code.json`):
   ```json
   "capabilities": { "stream": true, "preassigned_session_id": true, … },
   "stream_args": {
     "prefix_args": ["--print","--input-format","stream-json","--output-format","stream-json"],
     "session_id_flag": "--session-id",
     "resume_flag": "--resume"
   }
   ```
   `codex`, `claude`, and `copilot` are also `stream: true`; `hermes` and `opencode` are `stream: false`.

4. **Stream-json wire shape:** `system.init / user / assistant / tool_result / result` JSONL (per `coven run --stream-json` help). This maps cleanly onto `ChatEntryKind`: `assistant → AssistantResponse`, `tool_result → ToolCall/ToolResult card`, permission → `PermissionRequest`, `result → Stop`.

5. **Stream mode is long-lived and JSONL-duplex.** Per the coven-code manifest description: *"one turn per stdin user frame, exits on stdin EOF; the positional prompt is ignored in this mode."* So the prompt is delivered as a **stream-json user frame on stdin**, not the positional `prompt` field — and follow-up turns are additional user frames (no relaunch).

## Implications for the Phase 1 plan

- **Mechanism (confirmed):** `cast_agent` launches the coven-code session with `launchMode: "stream"` (not `nonInteractive`) and parses the JSONL `output` events into `CovenAgentEvent`s. No daemon or harness change required.
- **Open implementation detail (resolve early in the plan):** how the user prompt is delivered in stream mode. The positional `prompt` is ignored; the harness expects a stream-json **user frame on stdin**. Determine the daemon API for pushing a user frame to a live stream session (the *"write stream-json message to live session"* path) vs. whether `POST /api/v1/sessions` injects an initial frame from the `prompt` field. This is a small, well-bounded follow-up — a controlled single `coven run coven-code --stream-json` with a trivial prompt will confirm the exact wire behavior when needed.
- **Bonus:** the JSONL-duplex, long-lived session model directly enables **Phase 2's in-panel composer** (send follow-up user frames without relaunching) and generalizes to *any* stream-capable harness (codex/claude/copilot) — de-risking **Phase 3** too.
- **Fallback unchanged:** if a session is launched `nonInteractive` (or a harness is `stream: false`), the existing plain-text path renders as today. The rich path is strictly additive.

## Residual risk

Low. The only unknown is the exact prompt-delivery API in stream mode (item above), which is a wire detail, not a feasibility question. Rich rendering for coven-code is feasible with a `cast_agent`-only change.
