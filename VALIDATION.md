# Validation record — `rftoolsbuilder` 0.2.9

What was run, on which file, and the exact line it printed. Every log named
here lives in `ports/_shared/evidence/rftoolsbuilder/` unless it says otherwise.
Rust steps went through `ports/tools/build-slot.sh` (one lease per step,
`CARGO_INCREMENTAL=0`, relative `--port ports/rftoolsbuilder`, job
`W1-rftoolsbuilder`, no `--target-dir`) and **forwarded to `AdLf-PC`** — the
runner printed `build-slot: forwarding <step> for ports/rftoolsbuilder to the
remote runner`. The `horizon-launcher` game build ran locally (`BUILD_LOCAL=1`
on that step only: no launcher or prepared 26.3 resources on the remote);
`--step e2e` is not forwarded by the runner at all.

## The file every number applies to

```
ports/rftoolsbuilder/rftoolsbuilder.hmod
  version 0.2.9     92715 bytes
  sha256  626a18010cc73899efdad449bcc666c7af3a4992eb352a3727e463372c8b01db
  [native] game 26.3 · horizon 0a875da91dbb0fdcb7ba8c42c11341aa229af51a · mixins e59da9094f0c2fdda6c627fe9eff22dec6a906f0
  31 entries, no packed Cargo.lock
```

`verify-sdk-package.py rftoolsbuilder` → **1/1** (`package matches the tree`,
`runtime byte-identical a55911481c03`, R9 clean, `31 entries`, no packed
`Cargo.lock`, skills gate PASS).

## 0.2.9 (current)

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline --lib` | `running 74 tests` · `test result: ok. 74 passed; 0 failed` | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | 0 warnings, exit 0 | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/{lib,support,runtime}.rs` | clean, exit 0 | `fmt.log` |
| 4 | `cargo horizon-mod check` | `530 items of the API (…3 capabilitys…). Made for 0a875da.` | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `CMP_IDENTICAL`, 92,715 B | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.9 (native-mod, … for 26.3)`, 92,715 B, `SHA-256 626a1801…01db` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits … --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for` | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits … --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `0b14f8a  fits` | `sdk-fits-other.log` |
| 9 | `verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | package e2e + 8 negatives + 3 chunk-load legs | `PASS` + `EXIT:0`; control and 8 selftests `EXIT:1`; `SELFTEST=pump`: `FAIL: the pump moved 27000 mB; expected 27001` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=19 ok=19 missing-exit=0 bad-exit=0`, exit 0 | — |

## Ladder (historical 0.2.8)

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline` | `running 69 tests` · `test result: ok. 69 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.05s` · `Doc-tests rftoolsbuilder` · `running 0 tests` · `test result: ok. 0 passed; …`; `TEST_EXIT:0` | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | `CLIPPY_EXIT:0`, 0 warnings | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs` | `FMT_EXIT:0`, empty output | `fmt.log` |
| 4 | `cargo horizon-mod check` | `RFTools Builder uses 518 items of the API (5 Part kinds, 1 associated item, 3 capabilitys, 118 fields or variants, 13 hooks, 26 hook arguments, 2 macros, 198 methods, 36 traits, 116 types). Made for 0a875da.` + compatibility notes; `CHECK_EXIT:0` | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `Built ./target/horizon-mod/rftoolsbuilder.hmod` twice, `CMP_IDENTICAL` | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.8 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `86300 bytes, SHA-256 3b27c182…b81d1` / `FILE_CHECK_EXIT:0` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `RFTools Builder:` / `0a875da  made for` / `FITS_PIN_EXIT:0` | `file-check-fits.log` |
| 8 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `RFTools Builder:` / `0b14f8a  fits` / `FITS_OTHER_EXIT:0` | `file-check-fits.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | the in-game legs (below) | `PASS` + `EXIT:0`; control and eight selftests `EXIT:1` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `python3 ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=18 ok=18 missing-exit=0 bad-exit=0`, exit 0 | — |
| 12 | `python3 tools/check_recipes.py .` | `recipes PASS: 2 carried recipe files use 26.3 bare item/tag ingredients` | `recipe-guard-green.log` |

The two `test result:` lines above are the whole `cargo test` output for this
crate: one lib test binary (69) and doc-tests (0). The 57/12 split in `notes.md`
is the `runtime::tests` / `support::tests` count inside that 69.

## Red proofs (0.2.8)

The three quarry-filter tests were written first, so the first run could not compile:

```
error[E0560]: struct `runtime::CellFacts` has no field named `filter_passes`
error[E0599]: no variant or associated item named `Filtered` found for enum `runtime::SkipReason`
error: could not compile `rftoolsbuilder` (lib test) due to 8 previous errors
TEST_EXIT:101
```

(`red-filterquarry-compile.log`.) A second red caught a defect in the **test**
(`test-green-quarryfilter.log` first attempt): a deny-by-id filter must not reject
a tag member, so the tag case was rewritten with a `#minecraft:stones` filter.

The in-game counterfactual runs the new leg on the **0.2.7** game before the
glue existed: `27 quarried, scan complete` with no skips, then
`FAIL: the quarry filter skipped  cells; expected 18` + `EXIT:1`
(`../rftoolsbuilder-e2e/e2e-filterquarry-027-red.log`). The earlier red proofs
remain in `red-filter-compile.log`, `red-energy-compile.log`,
`red-chunkload-compile.log`, `red-void-compile.log`, `red-quarry-compile.log`,
`red-quarry-rule.log` and `red-quarry-void.log`.

## In-game legs

The game carries **both** `rftoolsbuilder` and `rftoolsbase` (the Builder's
recipe needs `rftoolsbase:machine_frame`, upstream's required parent):
`ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh` builds and exports it
after the stale `native-0a875da-*` tags are removed
(`Compiling rftoolsbase (1 of 2)` / `Compiling rftoolsbuilder (2 of 2)`,
`native-0a875da-9b9d38c35710`).

| leg | command | result | log |
|---|---|---|---|
| package | `GAME=<bundle game> tools/e2e.sh` | **`PASS`** + `EXIT:0` | `../rftoolsbuilder-e2e/e2e-package.log` |
| unmodded control | `GAME=$CONTROL_GAME` | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` | `e2e-control.log` |
| `SELFTEST=1` | the same game | `FAIL: built 24 cells, expected 25` + `EXIT:1` | `e2e-selftest.log` |
| `SELFTEST=quarry` | the same game | `FAIL: the quarry did not replace the 27 cells with minecraft:cobblestone (27 cells differ)` + `EXIT:1` | `e2e-quarry-selftest.log` |
| `SELFTEST=void` | the same game | `FAIL: the void card moved 0 items; expected 1` + `EXIT:1` | `e2e-void-selftest.log` |
| `SELFTEST=energy` | the same game | `FAIL: the quarry spent 8100 FE; expected 8101` + `EXIT:1` | `e2e-energy-selftest.log` |
| `SELFTEST=filter` | the same game | `FAIL: the filter skipped 18 cells; expected 17` + `EXIT:1` | `e2e-filter-selftest.log` |
| `SELFTEST=filterquarry` | the same game | `FAIL: the quarry filter skipped 18 cells; expected 17` + `EXIT:1` | `e2e-filterquarry-selftest.log` |
| quarry-filter, before the glue (0.2.7) | the same leg on the old game | `FAIL: the quarry filter skipped  cells; expected 18` + `EXIT:1` | `e2e-filterquarry-027-red.log` |
| chunk load | `GAME=<bundle game> tools/e2e-chunkload.sh` | **`PASS`** + `EXIT:0` | `e2e-chunkload.log` |
| chunk-load control | `GAME=$CONTROL_GAME tools/e2e-chunkload.sh` | `FAIL: the first Builder is not at …` + `EXIT:1` | `e2e-chunkload-control.log` |
| `SELFTEST=chunkload` | the same script | `FAIL: chunkload selftest: the pass completed in the unloaded chunk` + `EXIT:1` | `e2e-chunkload-selftest.log` |
| log gate | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=18 ok=18 missing-exit=0 bad-exit=0` | — |

The energy phase's own numbers, from `e2e-package.log`: a fresh Builder charged
`0 -> 1000000 FE (inserted 1000000)`, `energy 1000000/1000000 FE, spent 0 FE`
before the pass; after a 27-cell quarry, `energy 991900/1000000 FE, spent 8100 FE`
(27 × 300); after a 27-cell void card, `energy 993250/1000000 FE, spent 6750 FE`
(27 × 500 × 0.5); and an uncharged Builder reads `energy 0/1000000` and stalls
at `0 quarried, refused at …: not enough power`. The filter phase reads:

```
filter status: Builder at 34 102 -10: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, 9 placed, 18 skipped, scan complete
filter energy after: ... spent 5400 FE, ..., filter skipped 900 FE
filter-control energy: ... spent 4500 FE, ..., filter skipped 0 FE
```

The new quarry-filter phase (a fresh charged Builder, 18 stone + 9 dirt, the
replacement set to cobblestone) reads:

```
quarry-filter status: ... 27 cells, quarry (Normal quarry), replacing with minecraft:cobblestone, 9 quarried, 18 skipped, scan complete
quarry-filter energy after: ... energy 996400/1000000 FE, spent 3600 FE, ..., filter skipped 900 FE
quarry-filter chest: 9 dirt, 0 cobblestone (total 9)
quarry-filter volume: 18 stone kept, 9 replacement
quarry-filter-control status: ... 27 quarried, scan complete
quarry-filter-control chest: 9 dirt, 18 cobblestone (total 27)
```

`9 × 300 = 2700` FE for the allowed cells plus `18 × 50 = 900` FE for the denied
ones is the exact `spent 3600`; the control spends 0 skip FE and collects all 27.

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
rm -rf "$HOME/.cache/horizon-e2e/root/data/horizon-launcher/versions"/native-0a875da-*
eval "$(../../ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh env)"
GAME=$GAME CTL=$CTL bash tools/e2e.sh                       # PASS + EXIT:0
GAME=$CONTROL_GAME CTL=$CTL bash tools/e2e.sh               # EXIT:1 (control)
SELFTEST=1 GAME=$GAME CTL=$CTL bash tools/e2e.sh            # EXIT:1
SELFTEST=quarry GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1
SELFTEST=void GAME=$GAME CTL=$CTL bash tools/e2e.sh         # EXIT:1
SELFTEST=energy GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1
SELFTEST=filter GAME=$GAME CTL=$CTL bash tools/e2e.sh       # EXIT:1
SELFTEST=filterquarry GAME=$GAME CTL=$CTL bash tools/e2e.sh # EXIT:1
SELFTEST=pump GAME=$GAME CTL=$CTL bash tools/e2e.sh         # EXIT:1
GAME=$GAME CTL=$CTL bash tools/e2e-chunkload.sh              # PASS + EXIT:0
SELFTEST=chunkload GAME=$GAME CTL=$CTL bash tools/e2e-chunkload.sh  # EXIT:1
python3 ../../ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```
