# CastCodes FAQ

## What is CastCodes?

CastCodes is an open-source terminal and code workspace fork focused on local terminal and code workflows. The first rebrand pass changes external product identity while intentionally leaving many internal Rust crate/module names unchanged.

## Is CastCodes connected to upstream hosted services?

No. CastCodes does not provide upstream hosted services. The public CastCodes build disables sign-in, upstream cloud flows, hosted telemetry, hosted crash reporting, billing, shared sessions, upstream release feeds, and upstream feedback flows by default. Feedback links route to this fork's GitHub issue tracker.

## Why do internal names still say `warp`?

This pass avoids a risky blind rename. Internal names such as `warp_core`, `warpui`, `warp_terminal`, inherited upstream dependencies, protocol names, tests, and historical references may remain until a later internal rename.

## Where does CastCodes store local data?

New public CastCodes builds write to CastCodes paths such as `.cast-codes`, Linux `cast-codes`, Windows `castcodes\\CastCodes`, and macOS `dev.castcodes.CastCodes`. Legacy upstream paths are only compatibility references and should not receive newly-created CastCodes data.

## What should I run before opening a PR?

Run the most relevant checks for your change. For rebrand-facing changes, start with:

```bash
./script/check_rebrand
cargo check -p warp --bin cast-codes --features gui
```

If you touch path or channel identity code, also run the focused `warp_core` tests.
