# Validation record — `rftoolsbuilder` 0.2.10

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
  version 0.2.10     107914 bytes
  sha256  6d8c51ea9f8d2fc8c18a8defc3bdf649b4a947a475e523b42eb011264f034a3b
  [native] game 26.3 · horizon 0a875da91dbb0fdcb7ba8c42c11341aa229af51a · mixins e59da9094f0c2fdda6c627fe9eff22dec6a906f0
  45 entries, no packed Cargo.lock
```

`verify-sdk-package.py rftoolsbuilder` → **1/1** (`package matches the tree`,
`runtime byte-identical c6bbec5e23f2`, R9 clean, 45 entries, no packed
`Cargo.lock`, `tests 80 passed`, skills gate PASS).

## 0.2.10 (current)

| # | command | exact result | log |
|---|---|---|---|
| 1 | `cargo test --offline --lib` | `running 80 tests` · `test result: ok. 80 passed; 0 failed` | `sdk-test.log` |
| 2 | `cargo clippy --all-targets --offline -- -D warnings` | 0 warnings, `CLIPPY_EXIT:0` | `clippy.log` |
| 3 | `rustfmt --edition 2024 --check src/{lib,support,runtime}.rs` | clean, `FMT_EXIT:0` | `fmt.log` |
| 4 | `cargo horizon-mod check` | `RFTools Builder uses 536 items of the API (6 Part kinds, 1 associated item, 3 capabilitys, 124 fields or variants, 1 function, 13 hooks, 27 hook arguments, 2 macros, 205 methods, 36 traits, 118 types). Made for 0a875da.` | `check.log` |
| 5 | `rm -f Cargo.lock && cargo horizon-mod build` ×2 | `CMP_IDENTICAL`, 107,914 B | `build.log`, `build2.log` |
| 6 | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.10 (native-mod, … for 26.3)`, 107,914 B, `SHA-256 6d8c51ea…34a3b`, `FILE_CHECK_EXIT:0` | `file-check-fits.log` |
| 7 | `cargo horizon-mod fits … --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for` | `sdk-fits.log` |
| 8 | `cargo horizon-mod fits … --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `0b14f8a  fits` | `sdk-fits-other.log` |
| 9 | `verify-sdk-package.py rftoolsbuilder` | `1/1 package(s) verified` | — |
| 10 | package e2e + negatives | `PASS` + `EXIT:0`; control and selftests `EXIT:1` | `../rftoolsbuilder-e2e/*.log` |
| 11 | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=25 ok=25 missing-exit=0 bad-exit=0`, exit 0 | — |

## Red proofs (0.2.10)

```
error[E0425]: cannot find function `mover_available` in this scope
error[E0425]: cannot find function `arrival_flags` in this scope
error[E0425]: cannot find function `mover_line` in this scope
error[E0425]: cannot find function `controller_line` in this scope
error: could not compile `rftoolsbuilder` (lib test) due to 33 previous errors
TEST_EXIT:101
```

(`red-mover-compile.log`.) With `total_ticks` inverted to `ceil`:
`thread 'runtime::tests::total_ticks_is_the_floored_centre_distance' panicked
… assertion \`left == right\` failed`, `test result: FAILED. 79 passed; 1 failed`,
`TEST_EXIT:101` (`red-mover-rule.log`), source restored byte-for-byte.

## In-game legs

The game carries **both** `rftoolsbuilder` and `rftoolsbase` (the Builder's
recipe needs `rftoolsbase:machine_frame`, upstream's required parent):
`ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh` builds and exports it
after the stale `native-0a875da-*` tags are removed
(`Compiling rftoolsbase (1 of 2)` / `Compiling rftoolsbuilder (2 of 2)`,
`native-0a875da-169cea786318`).

| leg | command | result | log |
|---|---|---|---|
| package | `GAME=<bundle game> tools/e2e.sh` | **`PASS`** + `EXIT:0` | `../rftoolsbuilder-e2e/e2e-package.log` |
| unmodded control | `GAME=$CONTROL_GAME` | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` | `e2e-control.log` |
| `SELFTEST=1` | the same game | `FAIL: built 24 cells, expected 25` + `EXIT:1` | `e2e-selftest-1.log` |
| `SELFTEST=quarry` | the same game | `FAIL: the quarry did not replace the 27 cells with minecraft:cobblestone (27 cells differ)` + `EXIT:1` | `e2e-selftest-quarry.log` |
| `SELFTEST=void` | the same game | `FAIL: the void card moved 0 items; expected 1` + `EXIT:1` | `e2e-selftest-void.log` |
| `SELFTEST=energy` | the same game | `FAIL: the quarry spent 8100 FE; expected 8101` + `EXIT:1` | `e2e-selftest-energy.log` |
| `SELFTEST=filter` | the same game | `FAIL: the filter skipped 18 cells; expected 17` + `EXIT:1` | `e2e-selftest-filter.log` |
| `SELFTEST=filterquarry` | the same game | `FAIL: the quarry filter skipped 18 cells; expected 17` + `EXIT:1` | `e2e-selftest-filterquarry.log` |
| `SELFTEST=pump` | the same game | `FAIL: the pump moved 27000 mB; expected 27001` + `EXIT:1` | `e2e-selftest-pump.log` |
| `SELFTEST=mover` | the same game | `FAIL: the cow did not arrive at the destination mover (expected x=-10)` + `EXIT:1` | `e2e-mover-selftest.log` |
| chunk load | `GAME=<bundle game> tools/e2e-chunkload.sh` | **`PASS`** + `EXIT:0` | `e2e-chunkload.log` |
| chunk-load control / `SELFTEST=chunkload` | the same script | `FAIL` + `EXIT:1` | `e2e-chunkload-control.log`, `e2e-chunkload-selftest.log` |
| log gate | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=25 ok=25 missing-exit=0 bad-exit=0` | — |

The `package`, `control`, `mover-selftest`, `SELFTEST=1/quarry/void/energy/
filter` and all three chunk-load legs above were re-run on the **0.2.10** game.
The canonical `e2e-pump-selftest.log`, `e2e-filterquarry-selftest.log`,
`e2e-void-selftest.log` and `e2e-quarry-selftest.log` files remain from 0.2.9 and
are unchanged by this slice (no code path they assert was touched); the two
transient logs from load-failed retries were removed rather than kept as false
evidence. Some retries died before launch with `EXIT:3` (btrfs metadata ENOSPC
or a saturated box) — those runs produced no assertions and are not recorded.

The mover phase's own numbers, from `e2e-package.log`: a NoAI cow at
`-13.5, 102.0, -22.5` on the source mover, the controller reply
`move: -13 101 -22 -> -9 101 -22, 4 ticks, 1000 FE`, the charge
`0 -> 100000 FE (inserted 100000)`, and after the pass
`cow after: [[-9.5,102.0,-22.5]]`, source `4/4 ticks … delivered 0`,
destination `delivered 1`, `cows at destination -9 -22: 1; at source -13 -22: 0`,
and `destination block: rftoolsbuilder:mover`. The cardless control refuses
with `no vehicle card in the source mover` and leaves its cow at the source.

## Do not read these as the current package

`retired-0.2.0/` holds the **0.2.0** measurements; `e2e-launcher*.log` are the
pre-SDK `.hnmod` leg's transcripts. The 0.2.9 artifact's file-level numbers are
in the historical `notes.md` section. They describe older artifacts, not this
one.

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
SELFTEST=mover GAME=$GAME CTL=$CTL bash tools/e2e.sh        # EXIT:1
GAME=$GAME CTL=$CTL bash tools/e2e-chunkload.sh              # PASS + EXIT:0
SELFTEST=chunkload GAME=$GAME CTL=$CTL bash tools/e2e-chunkload.sh  # EXIT:1
python3 ../../ports/tools/check-e2e-logs.py rftoolsbuilder-e2e --strict
```
