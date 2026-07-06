# CLAUDE.md — CastCodes

Read [`AGENTS.md`](AGENTS.md). It is the source of truth for agents working in
this repo: the No AI Attribution hard rule, the rebrand guard, the Phase 1
design contract, verification gates, and pull-request rules.

This file exists so a coding harness that auto-loads `CLAUDE.md` is routed to the
same ruleset. There is no separate workflow here — follow `AGENTS.md`, and run
`./script/check_ai_attribution` and `./script/check_rebrand` before submitting.
