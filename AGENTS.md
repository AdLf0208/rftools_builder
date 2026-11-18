# Working on this native Horizon mod (SDK format)

This port is **one flat `.hmod`** in the SDK layout (mod API 0.2, 2026-11-18):
`horizon-mod.toml` is the only manifest, `src/` is the code the launcher compiles
against the pinned game build, `assets/` and `data/` are the carried content.
`mixins/`, `horizon-mixin.toml`, `horizon-native.json`, per-hook helper files,
`compatibility.json`, `rust-toolchain.toml` and the old `.hnmod` were retired
with the container — do not recreate them.

- `src/runtime.rs` — the pure geometry and the walk; **no `mod_api` types**, and
  every rule has a test. The jar's shape formulas are transcribed cell for cell,
  in `f32` where the Java is `float`.
- `src/support.rs` — the game glue (the block-entity Part, the card slot, the
  card's item Part, the tick, the readout commands). No business rules here:
  they belong in `runtime.rs` with a test.
- `src/lib.rs` — the one `#[mod_api::horizon_mod] impl Hooks`.
- `notes.md` — class/tier, the jar → package map, the defect fixed, the gaps.

Validation (every build/test step goes through the campaign's
`ports/tools/build-slot.sh` as the lease, never a bare `cargo` outside it):

```sh
cargo test
cargo clippy --all-targets -- -D warnings
rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs
cargo horizon-mod check                       # the SDK gate (format + manifest + code)
rm -f Cargo.lock                              # never pack a lock: it changes the artifact
cargo horizon-mod build                       # -> target/horizon-mod/rftoolsbuilder.hmod
cp target/horizon-mod/rftoolsbuilder.hmod ./rftoolsbuilder.hmod
cargo horizon-mod check rftoolsbuilder.hmod
cargo horizon-mod fits rftoolsbuilder.hmod --build nightly
```

Do **not** run `cargo fmt --all`: it reaches into the vendored `.horizon/sdk`
and rewrites the SDK's own sources. The delivered `rftoolsbuilder.hmod` lives in
the port root, never in `target/`. `.horizon/sdk` is fetched per clone (gitignored)
and never packed; `horizon-core` is a second path dependency from the same tree
only because `horizon-mod-api` does not re-export `JavaString`.

The in-game e2e needs a **launcher-built** game carrying the package:
`horizon-launcher native-build ports/rftoolsbuilder --instance <i> --trust-native-code`,
then `GAME=<that horizon-client> tools/e2e.sh`. There is no launcher at this pin
on this machine, so `tools/e2e.sh` has **not been run** — do not claim it has.
