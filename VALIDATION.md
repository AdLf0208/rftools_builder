# Validation record — `rftoolsbuilder` 0.2.2

What was run, on which file, and the exact line it printed. Every log named
here lives in `ports/_shared/evidence/rftoolsbuilder/` unless it says otherwise;
every build/test/fits step went through `ports/tools/build-slot.sh` (one lease
per step, `CARGO_INCREMENTAL=0`, no `--target-dir`), with `BUILD_LOCAL=1` on the
two steps that need `cargo horizon-mod` (the remote runner has no such
subcommand).

## The file every number applies to

```
ports/rftoolsbuilder/rftoolsbuilder.hmod
  version 0.2.2     56154 bytes
  sha256  f09026b0f9f9cc91a2204f9231b07a595e62cda9f18e80ab46601ad049c5e872
  [native] game 26.3 · horizon 0a875da91dbb0fdcb7ba8c42c11341aa229af51a · mixins e59da9094f0c2fdda6c627fe9eff22dec6a906f0
  31 entries, no packed Cargo.lock
```

`verify-sdk-package.py rftoolsbuilder` → **1/1** (`runtime byte-identical 9f5ab2a0c9d7`).

## Ladder

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline` | `Running unittests src/lib.rs (…)` · `running 38 tests` · `test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.00s` · `Doc-tests rftoolsbuilder` · `running 0 tests` · `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`; exit 0 | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs` | `FMT_EXIT:0`, empty output | `fmt.log` (0 bytes) |
| 4 | `cargo horizon-mod check` | `Made for 0a875da.` + `` `ModContainer`: built with `..` `` ; exit 0 | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `Built ./target/horizon-mod/rftoolsbuilder.hmod` twice, `cmp` identical | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.2 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `56154 bytes, SHA-256 f09026b0…e872` / `FILE_CHECK_EXIT:0` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `RFTools Builder:` / `0a875da  made for` / `FITS_PIN_EXIT:0` | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `RFTools Builder:` / `0b14f8a  fits` / `FITS_OTHER_EXIT:0` | `sdk-fits-other.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | the in-game legs (below) | `PASS` + `EXIT:0`; control and selftest `EXIT:1` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `python3 ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=7 ok=7 missing-exit=0 bad-exit=0`, exit 0 | — |
| 12 | `python3 tools/check_recipes.py .` | `recipes PASS: 2 carried recipe files use 26.3 bare item/tag ingredients` | `recipe-guard-green.log` |

The two `test result:` lines above are the whole `cargo test` output for this
crate: one lib test binary (38) and doc-tests (0). There is no second binary and
no ignored test, so 38 is the total; the 29/9 split in `notes.md` is the
`runtime::tests` / `support::tests` count inside that 38 (`grep -c` over
`sdk-test.log`).

## In-game legs

The game carries **both** `rftoolsbuilder` and `rftoolsbase` (the Builder's
recipe needs `rftoolsbase:machine_frame`, upstream's required parent):
`ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh` builds and exports it.

| leg | result | log |
|---|---|---|
| package | **`PASS`** + `EXIT:0` | `../rftoolsbuilder-e2e/e2e-package.log` |
| unmodded control | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` | `../rftoolsbuilder-e2e/e2e-control.log` |
| `SELFTEST=1` | `FAIL: built 24 cells, expected 25` + `EXIT:1` | `../rftoolsbuilder-e2e/e2e-selftest.log` |
| red proof (0.2.2 before the recipe fix) | `FAIL: a carried recipe did not craft` + `EXIT:1`, with the `recipes: invalid ingredient` lines | `../rftoolsbuilder-e2e/e2e-red-recipe.log`, `game-log-red-recipe.log` |

## Do not read these as the current package

`retired-0.2.0/` holds the **0.2.0** measurements (`fits.log`: `0b14f8a  made
for`; `runtime-test.log`: 22 tests). They describe the 51,246 B artifact, not
this one; their README says so. `file-check.log`, `sha256.txt`, `sha256-first.txt`,
`uses.log`, `carry-diff.txt` and `red-check*.log` are likewise 0.2.0/slice-1
records and are referenced only by the historical table in `notes.md`.

## Reproduce

```sh
cd ports/rftoolsbuilder
cargo test --offline
cargo clippy --all-targets --offline -- -D warnings
rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs
rm -f Cargo.lock && cargo horizon-mod build && cp target/horizon-mod/rftoolsbuilder.hmod .
cargo horizon-mod check rftoolsbuilder.hmod
cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a
cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4
python3 ../../ports/tools/verify-sdk-package.py rftoolsbuilder
eval "$(../../ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh env)"
GAME=$GAME CTL=$CTL bash tools/e2e.sh                       # PASS + EXIT:0
GAME=$CONTROL_GAME CTL=$CTL bash tools/e2e.sh               # EXIT:1 (control)
SELFTEST=1 GAME=$GAME CTL=$CTL bash tools/e2e.sh            # EXIT:1 (selftest)
python3 ../../ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```
