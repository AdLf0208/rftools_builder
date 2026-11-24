# Validation record — `rftoolsbuilder` 0.2.7

What was run, on which file, and the exact line it printed. Every log named
here lives in `ports/_shared/evidence/rftoolsbuilder/` unless it says otherwise.
Rust steps went through `ports/tools/build-slot.sh` (one lease per step,
`CARGO_INCREMENTAL=0`, relative `--port ports/rftoolsbuilder`, job
`W1-rftoolsbuilder`, no `--target-dir`) and **forwarded to `AdLf-PC`** — the
runner printed `build-slot: forwarding <step> for ports/rftoolsbuilder to the
remote runner`. The `horizon-launcher` game build and all in-game legs ran
locally (`BUILD_LOCAL=1` on the build that genuinely cannot run remotely: no
launcher, prepared 26.3 resources or `jq` there); `--step e2e` is not forwarded
by the runner at all.

## The file every number applies to

```
ports/rftoolsbuilder/rftoolsbuilder.hmod
  version 0.2.7     85091 bytes
  sha256  6b61c2be83f6f0a35ff2a41883eae322f018fd6fc48e5793cd6c5947f02fefdc
  [native] game 26.3 · horizon 0a875da91dbb0fdcb7ba8c42c11341aa229af51a · mixins e59da9094f0c2fdda6c627fe9eff22dec6a906f0
  31 entries, no packed Cargo.lock
```

`verify-sdk-package.py rftoolsbuilder` → **1/1** (`package matches the tree`,
`runtime byte-identical 2f54aad2c19d`, R9 clean, `31 entries`, no packed
`Cargo.lock`, skills gate PASS).

## Ladder

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline` | `running 66 tests` · `test result: ok. 66 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.86s` · `Doc-tests rftoolsbuilder` · `running 0 tests` · `test result: ok. 0 passed; …`; `TEST_EXIT:0` | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | `CLIPPY_EXIT:0`, 0 warnings | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs` | `FMT_EXIT:0`, empty output | `fmt.log` |
| 4 | `cargo horizon-mod check` | `RFTools Builder uses 518 items of the API (5 Part kinds, 1 associated item, 3 capabilitys, 118 fields or variants, 13 hooks, 26 hook arguments, 2 macros, 198 methods, 36 traits, 116 types). Made for 0a875da.` + the compatibility notes; `CHECK_EXIT:0` | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `Built ./target/horizon-mod/rftoolsbuilder.hmod` twice, `CMP_IDENTICAL` | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.7 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `85091 bytes, SHA-256 6b61c2be…f02fefdc` / `FILE_CHECK_EXIT:0` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `RFTools Builder:` / `0a875da  made for` / `FITS_PIN_EXIT:0` | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `RFTools Builder:` / `0b14f8a  fits` / `FITS_OTHER_EXIT:0` | `sdk-fits-other.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | the eleven in-game legs (below) | `PASS` + `EXIT:0`; control and seven selftests `EXIT:1` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `python3 ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=16 ok=16 missing-exit=0 bad-exit=0`, exit 0 | — |
| 12 | `python3 tools/check_recipes.py .` | `recipes PASS: 2 carried recipe files use 26.3 bare item/tag ingredients` | `recipe-guard-green.log` |

The two `test result:` lines above are the whole `cargo test` output for this
crate: one lib test binary (66) and doc-tests (0). The 54/12 split in `notes.md`
is the `runtime::tests` / `support::tests` count inside that 66 (`grep -c` over
`sdk-test.log`).

## Red proofs (0.2.7)

The five filter tests were written first, so the first run could not compile:

```
error[E0422]: cannot find struct, variant or union type `Filter` in this scope
error[E0433]: cannot find type `FilterMode` in this scope
error[E0425]: cannot find function `filter_match` in this scope
error[E0425]: cannot find function `filter_skip_cost` in this scope
error: could not compile `rftoolsbuilder` (lib test) due to 24 previous errors
TEST_EXIT:101
```

(`red-filter-compile.log`.) The earlier red proofs remain in
`red-energy-compile.log`, `red-chunkload-compile.log`, `red-void-compile.log`,
`red-quarry-compile.log`, `red-quarry-rule.log` and `red-quarry-void.log`.

A second red caught a harness/API defect while adding the in-game leg: the
filter value was first registered as `<value:word>`, whose brigadier parser
rejects `:` (`is_allowed_in_unquoted` has no colon), so the command never
answered; and `minecraft:grass` is not a registered block at this pin. The
value now uses `<value:resource:minecraft:block>` + a `filter none` usage, and
the mixed volume uses `minecraft:stone` / `minecraft:air`.

## In-game legs

The game carries **both** `rftoolsbuilder` and `rftoolsbase` (the Builder's
recipe needs `rftoolsbase:machine_frame`, upstream's required parent):
`ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh` builds and exports it
(`Compiling rftoolsbase (1 of 2)` / `Compiling rftoolsbuilder (2 of 2)`,
`native-0a875da-f33935421aa0`).

| leg | command | result | log |
|---|---|---|---|
| package | `GAME=<bundle game> tools/e2e.sh` | **`PASS`** + `EXIT:0` | `../rftoolsbuilder-e2e/e2e-package.log` |
| unmodded control | `GAME=$CONTROL_GAME` | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` | `e2e-control.log` |
| `SELFTEST=1` | the same game | `FAIL: built 24 cells, expected 25` + `EXIT:1` | `e2e-selftest.log` |
| `SELFTEST=quarry` | the same game | `FAIL: the quarry did not replace the 27 cells with minecraft:cobblestone (27 cells differ)` + `EXIT:1` | `e2e-quarry-selftest.log` |
| `SELFTEST=void` | the same game | `FAIL: the void card moved 0 items; expected 1` + `EXIT:1` | `e2e-void-selftest.log` |
| `SELFTEST=energy` | the same game | `FAIL: the quarry spent 8100 FE; expected 8101` + `EXIT:1` | `e2e-energy-selftest.log` |
| `SELFTEST=filter` | the same game | `FAIL: the filter skipped 18 cells; expected 17` + `EXIT:1` | `e2e-filter-selftest.log` |
| chunk load | `GAME=<bundle game> tools/e2e-chunkload.sh` | **`PASS`** + `EXIT:0` | `e2e-chunkload.log` |
| 0.2.4 counterfactual | the same script on the old game | `0 quarried … refused at …: the chunk is not loaded` + `EXIT:1` | `e2e-chunkload-024-red.log` |
| chunk-load control | `GAME=$CONTROL_GAME tools/e2e-chunkload.sh` | `FAIL: the first Builder is not at …` + `EXIT:1` | `e2e-chunkload-control.log` |
| `SELFTEST=chunkload` | the same game | `FAIL: chunkload selftest: the pass completed in the unloaded chunk` + `EXIT:1` | `e2e-chunkload-selftest.log` |
| log gate | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=16 ok=16 missing-exit=0 bad-exit=0` | — |

The energy phase's own numbers, from `e2e-package.log`: a fresh Builder charged
`0 -> 1000000 FE (inserted 1000000)`, `energy 1000000/1000000 FE, spent 0 FE`
before the pass; after a 27-cell quarry, `energy 991900/1000000 FE, spent 8100 FE`
(27 × 300); after a 27-cell void card, `energy 993250/1000000 FE, spent 6750 FE`
(27 × 500 × 0.5); and an uncharged Builder reads `energy 0/1000000` and stalls
at `0 quarried, refused at …: not enough power`. The filter phase reads:

```
filter command: Card filters minecraft:stone blocks (deny)
filter status: Builder at 33 102 -11: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, 9 placed, 18 skipped, scan complete
filter energy after: Builder at 33 102 -11: energy 994600/1000000 FE, spent 5400 FE, build 500 FE/cell, quarry 300 FE/cell, void 250 FE/cell, skip 50 FE/cell, infusion 0, filter skipped 900 FE
filter command: Card filter cleared
filter-control status: Builder at 37 102 -15: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, 9 placed, 18 skipped, scan complete
filter-control energy: Builder at 37 102 -15: energy 995500/1000000 FE, spent 4500 FE, …, filter skipped 0 FE
```

The 900 skip FE is exactly 18 × `builderRfPerSkipped` 50; the control spends 0.

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
SELFTEST=1 GAME=$GAME CTL=$CTL bash tools/e2e.sh            # EXIT:1
SELFTEST=quarry GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1
SELFTEST=void GAME=$GAME CTL=$CTL bash tools/e2e.sh         # EXIT:1
SELFTEST=energy GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1
SELFTEST=filter GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1
GAME=$GAME CTL=$CTL bash tools/e2e-chunkload.sh              # PASS + EXIT:0
SELFTEST=chunkload GAME=$GAME CTL=$CTL bash tools/e2e-chunkload.sh  # EXIT:1
python3 ../../ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```
