# Validation record — `rftoolsbuilder` 0.2.5

What was run, on which file, and the exact line it printed. Every log named
here lives in `ports/_shared/evidence/rftoolsbuilder/` unless it says otherwise.
Rust steps went through `ports/tools/build-slot.sh` (one lease per step,
`CARGO_INCREMENTAL=0`, relative `--port ports/rftoolsbuilder`, job
`G12-rftoolsbuilder`, no `--target-dir`) and **forwarded to `AdLf-PC`** — the
runner printed `build-slot: forwarding <step> for ports/rftoolsbuilder to the
remote runner`. The `horizon-launcher` game build and all in-game legs ran
locally (no launcher, prepared 26.3 resources or `jq` on the remote); that one
local build used `BUILD_LOCAL=1` because it genuinely cannot run there, while
`--step e2e` is not forwarded by the runner at all.

## The file every number applies to

```
ports/rftoolsbuilder/rftoolsbuilder.hmod
  version 0.2.5     75185 bytes
  sha256  b647bc0a80cc03265d886e78cbec0617b7a63cb20f2a63f3da29bd5df31dcefc
  [native] game 26.3 · horizon 0a875da91dbb0fdcb7ba8c42c11341aa229af51a · mixins e59da9094f0c2fdda6c627fe9eff22dec6a906f0
  31 entries, no packed Cargo.lock
```

`verify-sdk-package.py rftoolsbuilder` → **1/1** (`package matches the tree`,
R9 clean, `31 entries`, no packed `Cargo.lock`).

## Ladder

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline` | `running 54 tests` · `test result: ok. 54 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.8s` · `Doc-tests rftoolsbuilder` · `running 0 tests` · `test result: ok. 0 passed; …`; `TEST_EXIT:0` | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | `CLIPPY_EXIT:0`, 0 warnings | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs` | `FMT_EXIT:0`, empty output | `fmt.log` |
| 4 | `cargo horizon-mod check` | `RFTools Builder uses 502 items of the API (5 Part kinds, 1 associated item, 2 capabilitys, 113 fields or variants, 13 hooks, 26 hook arguments, 2 macros, 190 methods, 35 traits, 115 types). Made for 0a875da.` + the compatibility notes; `CHECK_EXIT:0` | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `Built ./target/horizon-mod/rftoolsbuilder.hmod` twice, `CMP_IDENTICAL` | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.5 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `75185 bytes, SHA-256 b647bc0a…3ab1a8` / `FILE_CHECK_EXIT:0` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `RFTools Builder:` / `0a875da  made for` / `FITS_PIN_EXIT:0` | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `RFTools Builder:` / `0b14f8a  fits` / `FITS_OTHER_EXIT:0` | `sdk-fits-other.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | the nine in-game legs (below) | `PASS` + `EXIT:0`; control and five selftests `EXIT:1` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `python3 ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=14 ok=14 missing-exit=0 bad-exit=0`, exit 0 | — |
| 12 | `python3 tools/check_recipes.py .` | `recipes PASS: 2 carried recipe files use 26.3 bare item/tag ingredients` | `recipe-guard-green.log` |

The two `test result:` lines above are the whole `cargo test` output for this
crate: one lib test binary (54) and doc-tests (0). The 44/10 split in `notes.md`
is the `runtime::tests` / `support::tests` count inside that 54 (`grep -c` over
`sdk-test.log`).

## Red proofs (0.2.5)

The four ticket tests were written first, so the first run could not compile:

```
error[E0425]: cannot find function `ticket_step` in this scope
error[E0433]: cannot find type `TicketPlan` in this scope
error: could not compile `rftoolsbuilder` (lib test) due to 14 previous errors
TEST_EXIT:101
```

(`red-chunkload-compile.log`.) The 0.2.4 counterfactual on the same never-visited
chunk rig (`e2e-chunkload-024-red.log`):

```
first readout: Builder at 20 102 2: Box (Solid) 3x3x3 offset 260 -152 260, 27 cells, quarry (Normal quarry), replacing with minecraft:dirt, 0 quarried, refused at 279 -49 261: the chunk is not loaded
FAIL: 0.2.4 counterfactual (expected red): ... 0 quarried, refused at 279 -49 261: the chunk is not loaded
EXIT:1
```

The earlier red proofs remain in `red-void-compile.log`, `red-quarry-compile.log`,
`red-quarry-rule.log` and `red-quarry-void.log`.

## In-game legs

The game carries **both** `rftoolsbuilder` and `rftoolsbase` (the Builder's
recipe needs `rftoolsbase:machine_frame`, upstream's required parent):
`ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh` builds and exports it
(`Compiling rftoolsbase (1 of 2)` / `Compiling rftoolsbuilder (2 of 2)`).

| leg | command | result | log |
|---|---|---|---|
| package | `GAME=<bundle game> tools/e2e.sh` | **`PASS`** + `EXIT:0` | `../rftoolsbuilder-e2e/e2e-package.log` |
| unmodded control | `GAME=$CONTROL_GAME` | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` | `e2e-control.log` |
| `SELFTEST=1` | the same game | `FAIL: built 24 cells, expected 25` + `EXIT:1` | `e2e-selftest.log` |
| `SELFTEST=quarry` | the same game | `FAIL: the quarry did not replace the 27 cells with minecraft:cobblestone (27 cells differ)` + `EXIT:1` | `e2e-quarry-selftest.log` |
| `SELFTEST=void` | the same game | `FAIL: the void card moved 0 items; expected 1` + `EXIT:1` | `e2e-void-selftest.log` |
| chunk load | `GAME=<bundle game> tools/e2e-chunkload.sh` | **`PASS`** + `EXIT:0` | `e2e-chunkload.log` |
| 0.2.4 counterfactual | the same script on the old game | `0 quarried … refused at 279 -49 261: the chunk is not loaded` + `EXIT:1` | `e2e-chunkload-024-red.log` |
| chunk-load control | `GAME=$CONTROL_GAME tools/e2e-chunkload.sh` | `FAIL: the first Builder is not at …` + `EXIT:1` | `e2e-chunkload-control.log` |
| `SELFTEST=chunkload` | the same game | `FAIL: chunkload selftest: the pass completed in the unloaded chunk` + `EXIT:1` | `e2e-chunkload-selftest.log` |
| log gate | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=14 ok=14 missing-exit=0 bad-exit=0` | — |

The void phase's own numbers, from `e2e-package.log`: `quarry (Void mode),
replacing with minecraft:dirt, 27 quarried, 27 voided, scan complete`, all 27 cells
`minecraft:dirt`, chest `159 -> 159`, and the normal-quarry differential on the same
rig `159 -> 186` (27 cobblestone collected). The chunk-load leg (`e2e-chunkload.log`)
holds `Forced chunks at 19 102 7: [17,16]` during the pass, then `none`; a second
Builder holds `[17,10]` in another never-visited chunk and releases it too.

## Do not read these as the current package

`retired-0.2.0/` holds the **0.2.0** measurements (`fits.log`: `0b14f8a  made
for`; `runtime-test.log`: 22 tests). They describe the 51,246 B artifact, not
this one; their README says so. `file-check.log`, `sha256.txt`, `sha256-first.txt`,
`uses.log`, `carry-diff.txt` and `red-check*.log` are likewise 0.2.0/slice-1
records and are referenced only by the historical table in `notes.md`.
`e2e-launcher*.log` are the pre-SDK `.hnmod` leg's transcripts.

## Reproduce

```sh
cd ports/rftoolsbuilder
cargo test --offline
cargo clippy --all-targets --offline -- -D warnings
rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs
cargo horizon-mod check
rm -f Cargo.lock && cargo horizon-mod build && cp target/horizon-mod/rftoolsbuilder.hmod .
cargo horizon-mod check rftoolsbuilder.hmod
cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a
cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4
python3 ../../ports/tools/verify-sdk-package.py rftoolsbuilder
eval "$(../../ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh env)"
GAME=$GAME CTL=$CTL bash tools/e2e.sh                       # PASS + EXIT:0
GAME=$CONTROL_GAME CTL=$CTL bash tools/e2e.sh               # EXIT:1 (control)
SELFTEST=1 GAME=$GAME CTL=$CTL bash tools/e2e.sh            # EXIT:1 (selftest)
SELFTEST=quarry GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1 (quarry counterfactual)
SELFTEST=void GAME=$GAME CTL=$CTL bash tools/e2e.sh         # EXIT:1 (void counterfactual)
GAME=$GAME CTL=$CTL SOCK=/home/adlf/.cache/horizon-e2e/scratch/rftoolsbuilder-chunkload.sock \
  bash tools/e2e-chunkload.sh                                # PASS + EXIT:0
SELFTEST=chunkload GAME=$GAME CTL=$CTL SOCK=/home/adlf/.cache/horizon-e2e/scratch/rftoolsbuilder-chunkload-selftest.sock \
  bash tools/e2e-chunkload.sh                                # EXIT:1 (chunk-load counterfactual)
python3 ../../ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```
