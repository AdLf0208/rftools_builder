# Validation record — `rftoolsbuilder` 0.2.3

What was run, on which file, and the exact line it printed. Every log named
here lives in `ports/_shared/evidence/rftoolsbuilder/` unless it says otherwise;
every build/test/fits step went through `ports/tools/build-slot.sh` (one lease
per step, `CARGO_INCREMENTAL=0`, no `--target-dir`), with `BUILD_LOCAL=1` on the
steps that need `cargo horizon-mod` and on the e2e legs.

## The file every number applies to

```
ports/rftoolsbuilder/rftoolsbuilder.hmod
  version 0.2.3     69798 bytes
  sha256  9efbb76f1a8092a217c81410ebc58f55b0e1a9691841f43d90183e0ee66da8b9
  [native] game 26.3 · horizon 0a875da91dbb0fdcb7ba8c42c11341aa229af51a · mixins e59da9094f0c2fdda6c627fe9eff22dec6a906f0
  31 entries, no packed Cargo.lock
```

`verify-sdk-package.py rftoolsbuilder` → **1/1** (`runtime byte-identical 5b633e62b965`,
`package matches the tree`, R9 clean, `31 entries`).

## Ladder

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline` | `running 45 tests` · `test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.05s` · `Doc-tests rftoolsbuilder` · `running 0 tests` · `test result: ok. 0 passed; …`; exit 0 | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | `CLIPPY_EXIT:0`, 0 warnings | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs` | `FMT_EXIT:0`, empty output | `fmt.log` (0 bytes) |
| 4 | `cargo horizon-mod check` | `RFTools Builder uses 481 items of the API (5 Part kinds, 1 associated item, 2 capabilitys, 110 fields or variants, 13 hooks, 26 hook arguments, 2 macros, 178 methods, 34 traits, 110 types). Made for 0a875da.` + the two compatibility notes; `CHECK_EXIT:0` | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `Built ./target/horizon-mod/rftoolsbuilder.hmod` twice, `cmp` identical | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.3 (native-mod, format native-1/0a875da91dbb0fdcb7ba8c42c11341aa229af51a/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)` / `69798 bytes, SHA-256 9efbb76f…a8b9` / `FILE_CHECK_EXIT:0` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `RFTools Builder:` / `0a875da  made for` / `FITS_PIN_EXIT:0` | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `RFTools Builder:` / `0b14f8a  fits` / `FITS_OTHER_EXIT:0` | `sdk-fits-other.log` |
| 9 | `python3 ports/tools/verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | the four in-game legs (below) | `PASS` + `EXIT:0`; control, `SELFTEST=1` and `SELFTEST=quarry` `EXIT:1` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `python3 ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=8 ok=8 missing-exit=0 bad-exit=0`, exit 0 | — |
| 12 | `python3 tools/check_recipes.py .` | `recipes PASS: 2 carried recipe files use 26.3 bare item/tag ingredients` | `recipe-guard-green.log` |

The two `test result:` lines above are the whole `cargo test` output for this
crate: one lib test binary (45) and doc-tests (0). The 35/10 split in `notes.md`
is the `runtime::tests` / `support::tests` count inside that 45 (`grep -c` over
`sdk-test.log`).

## Red proofs (0.2.3)

The seven quarry tests were written first, so the first run could not compile:

```
error[E0425]: cannot find type `Quarry` in this scope
   --> src/runtime.rs:1634:33
error: could not compile `rftoolsbuilder` (lib test) due to 84 previous errors
TEST_EXIT:101
```

(`red-quarry-compile.log`.) With the rule implemented, two inversions prove the tests can fail
(`red-quarry-rule.log`, `red-quarry-void.log`, source restored byte-for-byte afterwards):

```
assertion `left == right` failed: the quarry does not re-quarry its own replacement
  left: Harvest
 right: Skip(Replacement)
test result: FAILED. 44 passed; 1 failed
```

```
assertion `left == right` failed
  left: Harvest
 right: Void
test result: FAILED. 44 passed; 1 failed
```

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
| log gate | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=8 ok=8 missing-exit=0 bad-exit=0` | — |

The quarry phases' own numbers, from `e2e-package.log`: `27 quarried, 2 voided, scan complete`
with `25 cobblestone` collected; the put-back `0 quarried`; the tag-off pass `27 quarried,
1 voided` and `+1 coarse_dirt`; the replacement `minecraft:cobblestone`; the clearing pass
all-air with `104 cobblestone`; the silk pass `27 stone` and no silk-less cobblestone; the
fortune pass `1 flint, 0 gravel`; and the box-card control `0 placed` with the chest unchanged
(`159 -> 159`).

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
python3 ../../ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```
