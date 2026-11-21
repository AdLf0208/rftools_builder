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

The in-game e2e needs a **launcher-built** game carrying the package, and the
Builder's recipe takes `rftoolsbase:machine_frame`, so the game carries **both**
packages (upstream declares rftoolsbase a required dependency):

```sh
ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh      # builds + exports it, prints the env
eval "$(ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh env)"
ports/tools/build-slot.sh run --job W1-rftoolsbuilder --port ports/rftoolsbuilder --step e2e -- \
    bash tools/e2e.sh                                         # PASS + EXIT:0
GAME=$CONTROL_GAME ... tools/e2e.sh                           # the unmodded control: EXIT:1
SELFTEST=1 ... tools/e2e.sh                                   # the fail-capable leg: EXIT:1
python3 ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```

`bundle-game.sh` copies `ports/rftoolsbase` into a scratch folder and moves the
copy's `[native] horizon` to this pin — the port itself is read-only and never
edited. `tools/craft_check.py` (called by `tools/e2e.sh`) crafts both recipes in
a real table and `tools/check_recipes.py` is the data guard against the 1.21.1
ingredient dialect.

The 0.2.2 leg ran on 2026-11-21 (`PASS` + `EXIT:0`, control and selftest
`EXIT:1`); the transcripts are in `ports/_shared/evidence/rftoolsbuilder-e2e/`.
