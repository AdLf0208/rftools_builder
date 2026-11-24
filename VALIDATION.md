# Validation record — `rftoolsbuilder` 0.2.6

What was run, on which file, and the exact line it printed. Every log named
here lives in `ports/_shared/evidence/rftoolsbuilder/` unless it says otherwise.
Rust steps went through `ports/tools/build-slot.sh` (one lease per step,
`CARGO_INCREMENTAL=0`, relative `--port ports/rftoolsbuilder`, job
`W1-rftoolsbuilder`, no `--target-dir`) and **forwarded to `AdLf-PC`** — the
runner printed `build-slot: forwarding <step> for ports/rftoolsbuilder to the
remote runner`. The `horizon-launcher` game build and all in-game legs ran
locally (`BUILD_LOCAL=1` on the one build that genuinely cannot run remotely,
no launcher, prepared 26.3 resources or `jq` there); `--step e2e` is not
forwarded by the runner at all.

## The file every number applies to

```
ports/rftoolsbuilder/rftoolsbuilder.hmod
  version 0.2.6     81105 bytes
  sha256  a7161ca6521683986586859a6fe60a45ebfa79bce0f17ffa1a4c3a0761b17a92
  [native] game 26.3 · horizon 0a875da91dbb0fdcb7ba8c42c11341aa229af51a · mixins e59da9094f0c2fdda6c627fe9eff22dec6a906f0
  31 entries, no packed Cargo.lock
```

`verify-sdk-package.py rftoolsbuilder` → **1/1** (`package matches the tree`,
R9 clean, `31 entries`, no packed `Cargo.lock`, skills gate PASS).

## Ladder

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline` | `running 61 tests` · `test result: ok. 61 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.87s` · `Doc-tests rftoolsbuilder` · `running 0 tests` · `test result: ok. 0 passed; …`; `TEST_EXIT:0` | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | `CLIPPY_EXIT:0`, 0 warnings | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs` | `FMT_EXIT:0`, empty output | `fmt.log` |
| 4 | `cargo horizon-mod check` | `RFTools Builder uses 518 items of the API (5 Part kinds, 1 associated item, 3 capabilitys, 118 fields or variants, 13 hooks, 26 hook arguments, 2 macros, 198 methods, 36 traits, 116 types). Made for 0a875da.` + the compatibility notes; `CHECK_EXIT:0` | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `Built ./target/horizon-mod/rftoolsbuilder.hmod` twice, `CMP_IDENTICAL` | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.6 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `81105 bytes, SHA-256 a7161ca6…b17a92` / `FILE_CHECK_EXIT:0` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `RFTools Builder:` / `0a875da  made for` / `FITS_PIN_EXIT:0` | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `RFTools Builder:` / `0b14f8a  fits` / `FITS_OTHER_EXIT:0` | `sdk-fits-other.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | the ten in-game legs (below) | `PASS` + `EXIT:0`; control and six selftests `EXIT:1` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `python3 ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=15 ok=15 missing-exit=0 bad-exit=0`, exit 0 | — |
| 12 | `python3 tools/check_recipes.py .` | `recipes PASS: 2 carried recipe files use 26.3 bare item/tag ingredients` | `recipe-guard-green.log` |

The two `test result:` lines above are the whole `cargo test` output for this
crate: one lib test binary (61) and doc-tests (0). The 49/12 split in `notes.md`
is the `runtime::tests` / `support::tests` count inside that 61 (`grep -c` over
`sdk-test.log`).

## Red proofs (0.2.6)

The five energy tests were written first, so the first run could not compile:

```
error[E0425]: cannot find value `ENERGY_CAPACITY` in this scope
error[E0433]: cannot find type `CellCharge` in this scope
error[E0425]: cannot find function `energy_line` in this scope
error: could not compile `rftoolsbuilder` (lib test) due to 31 previous errors
TEST_EXIT:101
```

(`red-energy-compile.log`.) A harness defect caught while adding the in-game
leg (the charge reply used commas where the other readouts use spaces) is kept
in the first, red energy attempt of `e2e-package.log`:
`FAIL: the energy rig did not charge:`; the reply was made space-delimited and
the helper now retries until the reply lands. The earlier red proofs remain in
`red-chunkload-compile.log`, `red-void-compile.log`, `red-quarry-compile.log`,
`red-quarry-rule.log` and `red-quarry-void.log`.

## In-game legs

The game carries **both** `rftoolsbuilder` and `rftoolsbase` (the Builder's
recipe needs `rftoolsbase:machine_frame`, upstream's required parent):
`ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh` builds and exports it
(`Compiling rftoolsbase (1 of 2)` / `Compiling rftoolsbuilder (2 of 2)`,
`native-0a875da-38c46be051d0`).

| leg | command | result | log |
|---|---|---|---|
| package | `GAME=<bundle game> tools/e2e.sh` | **`PASS`** + `EXIT:0` | `../rftoolsbuilder-e2e/e2e-package.log` |
| unmodded control | `GAME=$CONTROL_GAME` | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` | `e2e-control.log` |
| `SELFTEST=1` | the same game | `FAIL: built 24 cells, expected 25` + `EXIT:1` | `e2e-selftest.log` |
| `SELFTEST=quarry` | the same game | `FAIL: the quarry did not replace the 27 cells with minecraft:cobblestone (27 cells differ)` + `EXIT:1` | `e2e-quarry-selftest.log` |
| `SELFTEST=void` | the same game | `FAIL: the void card moved 0 items; expected 1` + `EXIT:1` | `e2e-void-selftest.log` |
| `SELFTEST=energy` | the same game | `FAIL: the quarry spent 8100 FE; expected 8101` + `EXIT:1` | `e2e-energy-selftest.log` |
| chunk load | `GAME=<bundle game> tools/e2e-chunkload.sh` | **`PASS`** + `EXIT:0` | `e2e-chunkload.log` |
| 0.2.4 counterfactual | the same script on the old game | `0 quarried … refused at …: the chunk is not loaded` + `EXIT:1` | `e2e-chunkload-024-red.log` |
| chunk-load control | `GAME=$CONTROL_GAME tools/e2e-chunkload.sh` | `FAIL: the first Builder is not at …` + `EXIT:1` | `e2e-chunkload-control.log` |
| `SELFTEST=chunkload` | the same game | `FAIL: chunkload selftest: the pass completed in the unloaded chunk` + `EXIT:1` | `e2e-chunkload-selftest.log` |
| log gate | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=15 ok=15 missing-exit=0 bad-exit=0` | — |

The energy phase's own numbers, from `e2e-package.log`: a fresh Builder charged
`0 -> 1000000 FE (inserted 1000000)`, `energy 1000000/1000000 FE, spent 0 FE`
before the pass; after a 27-cell quarry, `energy 991900/1000000 FE, spent 8100 FE`
(27 × 300); after a 27-cell void card, `energy 993250/1000000 FE, spent 6750 FE`
(27 × 500 × 0.5); and an uncharged Builder reads `energy 0/1000000` and stalls
at `0 quarried, refused at …: not enough power`. The chunk-load leg still holds
`Forced chunks at 21 102 1: [17,-17]` in rig 2 and releases it at pass end.

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
SELFTEST=energy GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1 (energy counterfactual)
GAME=$GAME CTL=$CTL SOCK=/home/adlf/.cache/horizon-e2e/scratch/rftoolsbuilder-chunkload.sock \
  bash tools/e2e-chunkload.sh                                # PASS + EXIT:0
SELFTEST=chunkload GAME=$GAME CTL=$CTL SOCK=/home/adlf/.cache/horizon-e2e/scratch/rftoolsbuilder-chunkload-selftest.sock \
  bash tools/e2e-chunkload.sh                                # EXIT:1 (chunk-load counterfactual)
python3 ../../ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```
