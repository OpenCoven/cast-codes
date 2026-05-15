# CastCodes Chat Panel — Manual Verification Checklist

## Empty states
- [ ] No CLI on PATH: panel shows "No supported CLI detected"
- [ ] No prior conversations: list area shows "No conversations yet"

## Live transcript
- [ ] Run `claude` in a terminal with the plugin installed. Open panel (Cmd+Shift+H). See events stream as chat entries.
- [ ] Tool calls render as "[tool] ToolName()" cards
- [ ] Stop events show "Turn complete" marker

## Composer
- [ ] With live session: type + Enter sends to terminal PTY
- [ ] With no live session: composer disabled with placeholder

## Persistence
- [ ] Quit + restart CastCodes. Prior conversations appear in list.
- [ ] Click a past conversation: transcript loads from sqlite.

## Model picker
- [ ] Click "New chat": new terminal opens running `claude --model claude-opus-4-7`

## Error handling
- [ ] Error banner appears after 3+ skipped events

## Boundary
- [ ] `./script/check_cli_chat_boundary` passes
- [ ] No `warp.dev` references in cli_chat source
