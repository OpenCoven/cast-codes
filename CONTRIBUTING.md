# Contributing to CastCodes

> **⚠️ Contribution Status — Updated May 2026**
>
> **We are currently only accepting Issues and Bug Reports.**
> Pull Requests will not be reviewed or merged until **July 2026**.
> Please do not open PRs at this time — they will be closed without review.
> If you have a fix or feature in mind, open an issue to track it and we will pick it up when the contribution window reopens.

CastCodes accepts small, focused patches that preserve the existing architecture and keep public fork behavior honest.

## Rebrand Boundaries

- Use `CastCodes` for public UI, docs, app metadata, installers, packages, and release surfaces.
- Use `cast-codes` for public binaries, packages, and slugs.
- Use `castcodes` for public URL schemes and reverse-DNS organization segments.
- Do not blindly rename internal crates, modules, inherited dependency names, protocol identifiers, or historical tests.
- Do not add upstream service endpoints to the public CastCodes build.

## Development

```bash
./script/run
```

### Worktree-by-default branching

CastCodes is built across many concurrent branches (feature work, review checkouts,
release prep). The default workflow for any non-trivial branch is to put it in its
own git worktree so builds and target/ caches don't collide:

```bash
./script/worktree feat/my-thing                 # new branch off HEAD in .worktrees/feat/my-thing
./script/worktree feat/my-thing origin/main     # base off something other than HEAD
./script/worktree --list                        # see active worktrees
./script/worktree --remove feat/my-thing        # tear it down when done
```

The script always creates a *new* branch (it refuses to reuse an existing local
branch name) and always lands the worktree under `.worktrees/` at the repo root,
which is gitignored. A branch can only live in one worktree at a time — that's a
`git worktree` rule, not a CastCodes rule.

**Override:** the wrapper is a convention, not a gate. Work directly in the
primary checkout, or call `git worktree add` yourself, whenever the extra
isolation isn't worth it.

For rebrand-sensitive work:

```bash
./script/check_rebrand
cargo check -p warp --bin cast-codes --features gui
```

Add focused tests when changing path behavior, channel identity, app IDs, URL schemes, or service-gating logic.

## Pull Requests

Keep PRs narrow and squashable. Separate required fixes from optional cleanup. Mention any intentional remaining legacy upstream names references so reviewers can distinguish compatibility from missed rebrand work.

## Conduct

All contributors are expected to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
