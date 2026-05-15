# CastCodes Chat Panel — Product Spec

## Summary

Add a chat panel to CastCodes that lets users converse with locally-installed AI coding CLIs (`claude` and `codex` in v1) from inside the app. The panel renders streaming responses, surfaces tool calls and file edits, persists conversations locally, and supports switching CLIs and models. Authentication is delegated to each CLI's existing login flow — CastCodes never sees an API key or OAuth token.

## Why

CastCodes inherits a substantial AI chat surface from upstream Warp, but the entire surface routes through Warp's hosted multi-agent gateway. The fork-local boundary forbids calling upstream-owned infrastructure from the public OSS build, so the inherited chat is dead weight in OSS today. Users who want AI assistance must alt-tab to a separate Claude Code or Codex window.

A native panel that drives the user's existing CLI installs:

- Avoids the hosted-service dependency entirely (fork-local boundary respected by construction).
- Avoids registering an Anthropic/OpenAI OAuth client, hosting an inference proxy, or maintaining a model catalog — the CLI vendors do all of that.
- Gives CastCodes a credible, differentiated AI story: "local terminal plus local orchestration of CLIs you already trust."
- Sidesteps the messy "rip out the hosted agent" path — the inherited surface stays dormant; new panel is additive.

## Goals

- Real-time chat with `claude` and `codex` CLIs from a panel inside CastCodes.
- Switch between CLIs and between models within a CLI.
- Surface tool calls, file edits, and shell commands the CLI runs, in-line in the transcript.
- Persist conversations locally and resume them across app restarts.
- Honest empty / error states when the required CLI is not installed or not authenticated.
- Public-surface naming and copy that passes the rebrand guard.
- Zero calls to Warp-owned infrastructure from this feature.

## Non-goals (v1)

- Bring-your-own-API-key direct provider HTTP paths (Anthropic / OpenAI / Google / OpenRouter SDKs). Tracked as a Shape C follow-up; the architecture leaves room for it via a `ChatBackend` abstraction, but only the CLI backend ships in v1.
- OAuth with Anthropic / OpenAI directly. Auth lives in the CLIs; CastCodes does not register an OAuth client.
- Modifying or re-enabling the inherited Warp agent UI. It stays dormant in OSS. A separate cleanup pass may compile-gate it out of OSS builds later.
- Inline-in-block chat (chat exchanges rendered as terminal blocks). Different paradigm; out of scope.
- Voice / audio input. The inherited `app/src/ai/voice/` surface stays dormant.
- Sharing terminal block context into the chat as attachments. Worth a follow-up; v1 is text-only input.
- Multi-tab parallel sessions in the same panel. One active session per panel in v1.
- Gemini CLI, aider, opencode, or other backends beyond claude and codex.

## User flows

### Discovery

User opens the chat panel for the first time:

- If at least one supported CLI is detected on `PATH`, the panel opens to a blank composer with a model picker primed to the default (claude if available, else codex).
- If no supported CLI is detected, the panel renders an empty state: each supported CLI listed with its install command, a "Re-check installations" button, and a link to the CLI vendor's install docs. The composer is disabled.
- If a CLI is detected but reports not-logged-in (claude returns an auth error on first request), the panel surfaces the CLI's auth-failure message verbatim plus a one-line hint pointing to the CLI's login command (`claude /login`, `codex auth login`, etc.). The composer remains enabled so the user can retry after logging in externally.

### New chat

1. User selects CLI and model from the model picker (top of the panel).
2. User types into the composer and presses send.
3. Subprocess spawns. The transcript shows a "Starting <cli> with <model>…" placeholder for the first send only.
4. Stream-JSON events from the CLI render in real time:
   - **Assistant text** streams into a growing assistant message bubble.
   - **Tool calls** appear as collapsible cards (e.g., "Edit `src/foo.rs`", "Run `cargo check`"). Collapsed by default; expand to see arguments.
   - **Tool results** attach to the corresponding tool-call card. File-edit cards show a diff preview. Shell-command cards show captured output.
   - **Errors** show inline in the transcript with the CLI's error text and a "Restart session" button.
5. When the assistant turn ends, the composer re-enables.

### Model and CLI switching

Switching the active model or CLI mid-conversation is honest about the constraint: each CLI binds one model per session.

When the user selects a different model or CLI:

- The current subprocess is asked to close gracefully; if it does not close within a short timeout, it is killed.
- Two follow-up options are offered:
  - **Carry forward**: A new session is started with a system-level preamble of the form `[Continued from previous session] <auto-generated summary>`. The summary is produced from the prior transcript by the CLI itself in a one-shot summarization pass.
  - **Start fresh**: The new session starts with no preamble. The prior transcript remains visible above a divider.

### Resume across restarts

Conversations persist to the local sqlite store under `~/.cast-codes/`:

- On app launch, the panel sidebar lists prior conversations with title, last-message preview, last-used CLI/model, and timestamp.
- Opening a conversation restores the transcript from sqlite immediately. The underlying CLI session is not respawned until the user sends another message.
- On first send after open, if the prior session had a resume token (claude: `--resume <id>`), CastCodes attempts to resume.
- If resume fails (token expired, CLI uninstalled, model no longer available), CastCodes falls back to a "Start new session from here" affordance: a new subprocess is spawned and the prior transcript is summarized into a `[Continued from previous session] …` preamble, same as the model-switch flow.

### Stop / cancel

The composer's send button toggles to a stop button while a turn is streaming. Pressing stop cancels the in-flight turn by closing the CLI's stdin write half (or sending the CLI's documented cancel signal where applicable). Already-applied tool side effects (e.g., a completed file edit) are not rolled back; the transcript notes the cancellation.

### CLI crash or disconnect

If the subprocess exits unexpectedly:

- The transcript notes "Session ended unexpectedly" with the CLI's exit code and last stderr lines.
- A "Restart session" button restarts the subprocess and (if supported) resumes the prior session.
- The full transcript is preserved.

## Settings

A new section under the AI / Agent settings page (or a new top-level "CLI Chat" section, TBD during implementation):

- **Detected CLIs**: list of supported CLIs with version, status (Ready / Needs login / Not installed), and a "Re-check" button.
- **Default CLI**: dropdown.
- **Default model per CLI**: per-CLI dropdown of models the CLI advertises.
- **Custom path override per CLI**: optional absolute-path override for non-PATH installs.
- **Allow file edits from chat**: advisory toggle (default on). When off, CastCodes pre-warns the CLI via prompt prefix; the CLI still owns the actual permission decision.

## Branding and OSS boundary

- All user-visible strings use CastCodes naming, not Warp.
- The panel is feature-gated to OSS builds (or to all builds with the inherited surface coexisting; final gating decided in TECH.md).
- No requests leave the machine except the CLI subprocesses themselves (which call their respective vendor endpoints under their own auth).
- No telemetry is emitted for this feature. Local debug logging only.

## Success criteria

- A user with `claude` installed and logged in can send a message and receive a streaming response inside the panel within 5 seconds end-to-end (on a typical machine, excluding network latency).
- Tool calls and file edits the CLI performs are visible in the transcript without the user leaving the panel.
- Conversations survive an app restart and resume successfully via `claude --resume` when the prior session token is still valid.
- With no CLI installed, the panel renders an actionable empty state and does not silently fail.
- `./script/check_rebrand` passes for all new public-surface strings.
- No new code path calls any `*.warp.dev` domain.

## Open questions for implementation

- Final placement: dedicated sidebar vs. tab in existing pane group. Resolved in TECH.md.
- Whether to ship the codex backend in the initial PR or land claude first and add codex in a follow-up. Tracked as a TECH.md decision.
- How to encode model capabilities (which models support tool use, which don't) — likely a hand-curated table per CLI, refreshed when the CLI version is detected.
