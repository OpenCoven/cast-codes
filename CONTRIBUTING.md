# Contributing to CastCodes

> **⚠️ Contribution Status — Updated June 2026**
>
> **Independence Day PR Event:** external Pull Requests open starting **Saturday, July 4, 2026**.
> Until then, we are accepting Issues and Bug Reports only.
> If you have a fix or feature in mind, open an issue to track it and bring the PR when the event opens.

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

---

## OpenCoven DCO and Patent Terms

Thank you for your interest in contributing. OpenCoven is MIT licensed and community-driven. We want contributing to be easy, open, and safe for everyone.

## Developer Certificate of Origin (DCO)

OpenCoven uses the **Developer Certificate of Origin (DCO) v1.1** for all contributions. This is a lightweight mechanism — not a CLA — that asks you to certify that you have the right to submit what you're submitting.

By making a contribution to this project, you certify that:

> (a) The contribution was created in whole or in part by you and you have the right to submit it under the open source license indicated in the file; or
>
> (b) The contribution is based upon previous work that, to the best of your knowledge, is covered under an appropriate open source license and you have the right under that license to submit that work with modifications, whether created in whole or in part by you, under the same open source license (unless you are permitted to submit under a different license), as indicated in the file; or
>
> (c) The contribution was provided directly to you by some other person who certified (a), (b) or (c) and you have not modified it.
>
> (d) You understand and agree that this project and the contribution are public and that a record of the contribution (including all personal information you submit with it, including your sign-off) is maintained indefinitely and may be redistributed consistent with this project or the open source license(s) involved.

### How to Sign Off

Add a `Signed-off-by` line to your commit message:

```
git commit -s -m "Your commit message"
```

This produces:

```
Your commit message

Signed-off-by: Your Name <your.email@example.com>
```

### Patent Non-Assertion

By contributing, you additionally agree not to assert any patent claims — now held or later acquired — against this project or its users that arise from your contribution. See [PATENTS](./PATENTS) for the full non-assertion pledge.

## What We're Looking For

- Bug fixes and reliability improvements
- Documentation and example improvements
- New skills, tools, and integrations
- Performance improvements
- Community-requested features

## What We're Not

OpenCoven is not a contribution vehicle for proprietary forks. If you are building a closed-source derivative of OpenCoven's architecture, please do not use contribution as a means to learn implementation details that are not yet public. We welcome genuine collaborators.

## Getting Started

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes with signed-off commits: `git commit -s`
4. Open a pull request with a clear description

## Questions?

Join the Discord: https://discord.gg/OpenCoven
