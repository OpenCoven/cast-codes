# CastCodes Chat Panel — Product Spec

## Summary

Add a chat-style panel to CastCodes that renders the conversation flowing between the user and an AI coding CLI (`claude`, `codex`, `gemini`, `opencode` — collectively "supported CLIs") as a navigable, persistent transcript. The CLI itself runs in an ordinary CastCodes terminal pane and emits structured events via its already-supported plugin protocol; the chat panel is a new presentation surface over those events plus a local conversation history. The panel adds nothing to the CLI's auth, billing, or model orchestration — those remain entirely the CLI's concern.

## Background — what already exists

CastCodes inherits a substantial CLI-agent infrastructure from upstream Warp that we are explicitly **not** rebuilding:

- `app/src/terminal/cli_agent_sessions/event/` parses OSC 777 events emitted by vendor plugins into typed `CLIAgentEvent`s.
- `app/src/terminal/cli_agent_sessions/CLIAgentSessionsModel` tracks per-terminal CLI sessions, status (in-progress / blocked / waiting-permission / idle / stopped), and rich-input draft state.
- `app/src/terminal/input/cli_agent.rs` provides a rich-input editor opened via Ctrl-G or a footer button that writes prompts into the running CLI's stdin.
- Per-CLI adapters under `cli_agent_sessions/plugin_manager/` know how to detect installation status and (in upstream) auto-install vendor plugins from Warp-owned marketplace repos.

What is missing is the **chat presentation**. Today the user sees their conversation only as raw terminal scrollback plus small status chips in the vertical-tabs sidebar. There is no transcript view, no cross-restart history, no list of past conversations, and no place to look at a tool call or a file edit in a non-terminal layout. The "chat panel" closes that gap.

## Why

- Reading multi-turn CLI agent conversations in a terminal is awkward — long responses scroll past prompts, tool calls and file edits blend into ordinary shell output, and there's no record after the terminal closes.
- A dedicated chat-style view makes the conversation legible the way users expect from a chat UI, while leaving the CLI in charge of model selection, auth, and tool execution (which is exactly the boundary the fork-local OSS build needs).
- Persistence across restarts lets a user pick up a prior conversation by clicking it in a sidebar list rather than searching terminal scrollback.

## Goals

- A panel that, when bound to a running CLI session, renders that session's transcript live as chat: user prompts, assistant responses, tool calls, file edits, permission requests, idle prompts, stop events.
- Persistence: events for every observed CLI session are stored in a local sqlite database so transcripts survive app restart and terminal close.
- A conversation list (sidebar within the panel) of all past CLI sessions, sortable by recency, openable in read-only "view past" mode.
- The composer in the panel is wired to the existing rich-input flow so the user can send a follow-up without leaving the panel.
- Model picker: a button in the panel lets the user launch a new CLI session in a new (or selected) terminal with a chosen model. We do not start subprocesses ourselves — we run the CLI inside a terminal and pass the model flag.
- Honest empty / error / not-installed states. If the relevant plugin is not present (and therefore no events will arrive), say so clearly. Do not silently fail.
- Public-surface strings and assets pass `./script/check_rebrand`. No new code paths call Warp-owned infrastructure.

## Non-goals (v1)

- We do not spawn the CLI as a subprocess from the panel. The CLI runs in the terminal pane as a regular shell command. Removing the terminal hop is a possible future evolution, not in scope.
- We do not parse stream-JSON directly from the CLI's stdout. The OSC 777 event protocol is the source of truth and is already implemented.
- We do not modify or re-enable the inherited Warp hosted-agent UI (`app/src/ai/agent_conversations_model.rs`, the inherited conversation_list, etc.). That stays dormant in OSS.
- We do not implement direct-provider BYOK HTTP paths (Anthropic / OpenAI / Google / OpenRouter SDKs) or OAuth flows. Auth stays in the CLIs.
- We do not auto-install the vendor plugins via Warp-owned marketplace repos. The fork-local boundary forbids it. We may surface installation instructions, possibly with non-Warp-pointing alternatives where they exist; investigation in TECH.md.
- We do not ship inline-in-block chat, voice input, terminal-block context attachments, or multi-tab parallel chat sessions in the same panel. All worth follow-ups; out of scope.

## User flows

### Discovery

User opens the chat panel for the first time:

- If a supported CLI session is currently active in any terminal, the panel opens to that session's live transcript.
- If no CLI session is active but past sessions exist in local storage, the panel opens to the conversation list with the most recent at the top.
- If no past sessions exist and no CLI is detected on `PATH`, the panel opens to an empty state listing the supported CLIs with the canonical install commands and a "Refresh" button. The composer is disabled.
- If a CLI is on `PATH` but the vendor plugin is not installed (so events won't be emitted), the panel renders a state explaining that the chat panel renders events emitted by the vendor plugin and points the user at vendor documentation. The panel does not auto-install the plugin in OSS (see fork-local boundary).

### Live transcript

The user runs `claude` (or `codex`/etc.) in a CastCodes terminal pane. Once an OSC 777 `session_start` event arrives, the chat panel — if open and bound to that terminal — begins rendering:

- **`prompt_submit`** events render as a user message bubble.
- **`tool_complete`** events render as a collapsible tool-call card in the assistant column (collapsed by default; expand to see tool name and input preview).
- **`permission_request`** events render as a permission card inline in the transcript. The card mirrors what the terminal shows; clicking does not approve here — the user still approves in the terminal. We surface the request so the user knows what's pending without alt-tabbing.
- **`question_asked`** / **`idle_prompt`** events render as a thin info bar that the agent is waiting on the user.
- **`stop`** events finalize the assistant turn. If a `response` field is present in the event, it renders as the assistant's final message; if only `query` and tool events appear (no final response captured by the plugin), the transcript shows the tool sequence and a "Turn complete" footer.

The transcript streams in real time as events arrive. Existing terminal-side state (vertical-tabs chip, status icons) continues to update as it does today — the panel does not replace those, it complements them.

### Composer

The panel includes a composer at the bottom wired into the existing rich-input flow:

- When a CLI session is active, the composer is enabled and typing-then-send delegates to `CLIAgentSessionsModel::open_input` followed by submission. Submission types the prompt into the CLI's terminal stdin, identical to the Ctrl-G / footer-button flow today.
- When no CLI session is active, the composer is disabled with a hint ("Run `claude` or `codex` in a terminal to start chatting").

### Conversation list

A sidebar within the panel lists all known past sessions, identified by the `session_id` carried in events:

- Each entry shows: agent (claude/codex/etc.), title (first user prompt or `summary` field), last activity timestamp, cwd / project hint, status badge for the last known state.
- Opening a past entry switches the panel into read-only "view past" mode: the transcript renders from local storage; the composer is disabled with a note that this is a past session. A "Continue in new terminal…" button opens a new terminal and starts the relevant CLI with the appropriate resume flag (claude: `claude --resume <session_id>`).

### Model picker

A small dropdown in the panel header lets the user start a new CLI session with a chosen model:

- The picker offers per-CLI model options (a hand-curated list, refreshed as TECH.md describes).
- Selecting a model + "New chat" opens a new terminal pane, runs the selected CLI with `--model <id>` (or its equivalent flag), and binds the panel to that new terminal once `session_start` arrives.
- Selecting a different model on an already-bound session does **not** mutate the running session. The picker either starts a fresh session in a new terminal, or offers an optional "carry-forward summary" flow that runs the CLI with a preamble prompt; the user can opt in.

### Restart and resume

On app restart, the conversation list reloads from sqlite. Opening a past conversation always starts in read-only mode. The "Continue in new terminal" button uses the CLI's own resume flag (claude: `--resume <session_id>`; codex: equivalent rollout-based resume) when available.

### CLI / plugin issues

- **CLI not on PATH**: empty state with install commands; the conversation list still works.
- **CLI on PATH but plugin not installed**: panel renders a "Plugin required" state pointing at vendor docs.
- **Terminal closes mid-session**: panel marks the session as ended; transcript remains in the conversation list.
- **OSC 777 events arrive but cannot be parsed (version mismatch)**: events are logged and skipped, the panel surfaces a one-line "Plugin version may be incompatible" notice.

## Settings

A new section under the AI / Agent settings page:

- **Detected CLIs**: list with version (from `claude --version` etc.), plugin status (installed / missing / unknown), "Refresh" button.
- **Default model per CLI**: dropdown of curated model IDs per CLI, used when starting a new chat via the model picker.
- **Show permission cards in the chat transcript**: toggle (default on).
- **Open chat panel automatically when a CLI session starts**: toggle (default off in v1; we don't surprise the user with a new pane).

The settings section explicitly does not include the plugin auto-installer — see fork-local boundary.

## Branding and OSS boundary

- All new user-visible strings use CastCodes naming. `./script/check_rebrand` passes after the work.
- The new panel and its sqlite store live entirely on the local machine. Nothing this feature adds reaches the network.
- We do not call into `plugin_manager/*.rs` from the new panel for installation. We may read from it (`is_installed`, version detection) so long as we do not invoke any code path that consults Warp-owned URLs. If reuse is impractical, we re-implement a minimal "is the plugin installed" check inside the new module.
- The inherited plugin auto-installer remains compiled but is reachable only through inherited UI surfaces, which themselves are dormant in OSS. Compile-gating it out of OSS builds entirely is a follow-up.
- No telemetry is emitted from this feature. Local `tracing`/`log` debug spans only.

## Success criteria

- With `claude` installed and the warp plugin present, a user runs `claude` in a CastCodes terminal, opens the chat panel, types a prompt in the panel composer, sends it, and sees the resulting transcript stream into the panel within the same wall-clock window as the terminal shows it.
- After restart, the user opens the chat panel and sees a list of prior sessions; clicking one reproduces the transcript from local storage.
- With no CLI installed, the panel renders an actionable empty state and does not silently fail.
- `./script/check_rebrand` passes for all new public-surface strings.
- A CI grep guard confirms no new source file under the new module references `warp_multi_agent_api`, `warp_server_client`, `*.warp.dev`, or `warpdotdev/`.

## Open questions

- Whether the panel binds 1:1 to a terminal pane or to a CLI session identifier (which can outlive a terminal). Likely the latter; TECH.md resolves.
- Whether to show a per-session pane-link affordance (jump to the terminal that owns the session). Probably yes; small UI work.
- How to surface multiple parallel CLI sessions in the panel (split view vs. tabs vs. switch). v1 likely shows one active session at a time with a switcher; TECH.md resolves.
- Whether the model picker's "New chat" should open the new terminal in the current workspace, a new tab, or a split. Probably current workspace, new tab; revisitable during implementation.
