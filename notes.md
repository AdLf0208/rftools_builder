# RFTools Builder → Horizon

Ported from **RFTools Builder 1.21-7.0.8** (`McJtyMods/RFToolsBuilder`, **MIT**),
Minecraft Java 1.21.1 NeoForge. The jar is
`ports/_shared/licence-cache/rftoolsbuilder-1.21-7.0.8.jar` (1,054,376 B, 615
entries); the behaviour was read from branch `1.21_neo` at commit
`4090494663bd15e5776d9e407a59ec982129ef1f` (`gradle.properties`
`version=1.21-7.0.8`, the jar's own version) — analysis only, this is a
clean-room Rust rewrite of the behaviour, never a translation of the code.

**Licence: MIT → rank 0 (permissive).** Read from the jar's own
`META-INF/neoforge.mods.toml`
(`license="MIT License https://github.com/McJtyMods/RFToolsBuilder/blob/1.21_neo/LICENCE"`);
`audit_licences.py --mod rftoolsbuilder --fetch` → AUDITED, matches. The port
carries the upstream MIT notice in [`LICENSE`](LICENSE) and its provenance in
[`LICENSE-NOTICE.md`](LICENSE-NOTICE.md). Local-only until the maintainer
approves publishing (ARCHITECTURE.md §8); MIT permits redistribution with its
notice, so the port is publishable under MIT with this source.

**Format: the SDK package (2026-11-18), version 0.2.0 — first publication.**
One flat `.hmod` — `horizon-mod.toml` + `Cargo.toml` + `src/` + `assets/` +
`data/` — built by the launcher against SDK **`0b14f8acfd97fa2c750e42795249dcc981178ca4`**
(SDK commit `0b14f8a`, `package_format` 6, `content_api` "0.2") with horizon-mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, game 26.3, launcher 0.2.78.
`.hnmod`, `mixins/`, `horizon-mixin.toml`, per-hook helper files and
`compatibility.json` are retired with that container and are not recreated.

Status: **ported-partial** — slice 1 (the Builder's shaped mode with its shape
cards) is implemented, its 25 geometry tests and 8 glue tests (33 in all) are green, the SDK
gate is green and the package is built; it has **not** been run in a
launcher-built game on this machine (no `horizon-launcher` here — gap 1), so no
in-game claim is made. The gap list below is the rest of the mod.

## Class + tier and the approach

- **Class:** `content`; **tier:** `deep`; kind mechanical; role add-on
  (`ports/ledger.csv` row 404, `ports/_shared/source-map/atm10-sources.csv`).
- **Approach** (`horizon/docs/modpacks.md`, "Classes"/"Tiers" and "Code: one
  agent per mod"): the `deep` tier's shape here is a **BlockEntity + item data +
  a per-tick planner**, not entities or renderers — the slice is the Builder's
  shaped mode, i.e. a machines-shaped block entity (card slot, tick, saved
  state) whose value is the pure geometry behind it. The `deep` row is kept
  because the mod as a whole has the movers' entities, the shield renderer and
  the space-chamber dimensions; none of those is in slice 1.
- **Carry-over vs agent port:** the blockstate, the block and item models, the
  Builder's front texture, the card's texture, the recipes, the loot table and
  the lang lines are carried (mechanically); the three machine casing textures
  come from RFToolsBase (same author, MIT) so the Builder renders without it. The
  shape formulas, the clamps, the walk, the card model and its authoring, the
  tick and the readout are the **agent port**.
- **Parents:** `rftoolsbase` (required upstream; ours is `ports/rftoolsbase`,
  `ported-partial`) and `mcjtylib` (a library, mapped, not ported). The port's
  Rust calls neither; RFToolsBase is only the `rftoolsbase:machine_frame`
  ingredient of the Builder recipe (the same dependency the sibling ports
  record).
- **Family shapes followed:** `ports/rftoolsbase` (block declaration on a
  directional template, `data/<id>/horizon/*`, carried casing textures),
  `ports/rftoolsstorage` (`BlockEntityPart` + declared `block_container` + card
  slot + `/… status` readout + `notes.md`/`VALIDATION.md` layout) and
  `ports/rftoolsutility` (content-only subset, item declarations). All three
  were read, none edited.

## The jar, measured

| Family | Files | This slice |
|---|---:|---|
| `assets/rftoolsbuilder/blockstates` | 26 | 1 (builder) |
| `assets/rftoolsbuilder/models/block` | 38 | 1 |
| `assets/rftoolsbuilder/models/item` | 38 | 2 |
| `assets/rftoolsbuilder/textures/block` | 47 | 4 (1 + 3 casings) |
| `assets/rftoolsbuilder/textures/item` | 15 | 1 |
| `assets/rftoolsbuilder/textures/gui`, `gui/` | 9 + 4 | 0 |
| `assets/rftoolsbuilder/lang` | 2 | 1, cut down |
| `data/rftoolsbuilder/recipe` | 42 | 2 |
| `data/rftoolsbuilder/loot_table` | 22 | 1 |
| `data/rftoolsbuilder/advancement` | 42 | 0 |
| `data/minecraft`, `data/neoforge`, `data/c` | 5 + 2 + 0 | 4 tags |
| classes (`mcjty/…`) | 222 | rewritten in Rust |

(The brief said "26 blockstates, 38 item models, 87 recipes, 24 loot tables";
measured against the jar it is **42 recipes**, **22 loot tables** and **2** lang
files — the artefact wins, and the slice only carries two recipes anyway.)

## What the slice does

- **The shape card** (`rftoolsbuilder:shape_card_def`): one item whose
  `rftoolsbuilder:shape_card` component holds `shape`, `solid`, `dimension`,
  `offset`, `rotation`, `flipY` plus the corner-selection state (`mode`,
  `selected`, `selectedDimension`, `corner1`) — upstream's `ShapeCardData`
  field names for the part this slice keeps. A stack with no component is
  upstream's `ShapeCardData.DEFAULT`: a solid 5×5×5 box at offset 0.
- **The shapes**: the jar's ten pure formulas — `SHAPE_BOX`, `SHAPE_TOPDOME`,
  `SHAPE_BOTTOMDOME`, `SHAPE_SPHERE`, `SHAPE_CYLINDER`, `SHAPE_CAPPEDCYLINDER`,
  `SHAPE_PRISM`, `SHAPE_TORUS`, `SHAPE_HEART`, `SHAPE_CONE` — transcribed cell
  for cell, with Java's `float` arithmetic, its `(int)` truncation (the cone's
  reach, the `davg` averages) and the `(dim % 2 != 0) ? 0.0f : -.5f` centre so a
  5-wide sphere is 81 cells and a 3-wide one is 19, not 27. `SHAPE_COMPOSITION`
  and `SHAPE_SCAN` are **not** carried (they need the composer and the scanner);
  the card refuses their names.
- **Solid and hollow**: upstream's `IFormula.correctFormula(false)` replaces the
  inside test with the border test — a cell is in the shell when one of its six
  neighbours is outside. The jar's quirky consequence is kept: the cylinder's
  `isInside` ignores y, so a hollow cylinder has **no cap**, only its rim (98
  cells for a hollow 5×5×5 box, 36 for a hollow 5×3×5 cylinder, `notes` tests).
- **The clamps**: dimension into `0..=512` (`maxBuilderDimension`) and offset
  into `-260..=260` (`maxBuilderOffset`), exactly `getClampedDimension` /
  `getClampedOffset`; a card whose clamped box holds more than **50,000,000**
  cells (`ShapeCardItem.MAXIMUM_COUNT`) is refused with those numbers.
- **Authoring**: sneak-right-click a Builder with the card, then click two
  corners — `ShapeCardItem.useOn`'s arithmetic (`ceil((c1 + c2) / 2)` centre,
  `|c1 - c2| + 1` size, offset from the Builder's own position, shape = box,
  solid, selection cleared) and its three messages and three guards (no Builder,
  another dimension, the Builder itself).
  `/rftoolsbuilder shape <shape> <dx> <dy> <dz> [solid|hollow]`,
  `/rftoolsbuilder offset <x> <y> <z>` and `/rftoolsbuilder reset` write the
  held card for anything the corners cannot express.
- **The Builder** (`rftoolsbuilder:builder`): a block entity with a one-slot
  card container (`BuilderTileEntity.SLOT_TAB`), a saved compound
  (`cursor`, `placed`, `skipped`, `done`, `refusal`, `refusalPos`) and a server
  tick that walks the card's shape from the top down (x fastest, then z, then y)
  and places the first placeable block item from the container **above** it, or
  **below** if that one has none (`createTakeableItem`'s order), at
  `quarryBaseSpeed` = **8 blocks a tick** and at most 800 look-ups a tick —
  upstream's `100 × quarryBaseSpeed`.
- **The player surface**: a card in hand goes into the card slot (the slot is
  the container, so a broken Builder spills it); an empty hand reads the Builder
  out in the action bar; a sneaking empty hand takes the card back.
- **The readout**: `/rftoolsbuilder status <pos>` prints the shape, its size and
  offset, the cell count, the progress and the first cell it refused and why —
  byte-stable text pinned by `the_status_line_is_byte_stable`.

## Hook map (SDK 0.2)

| Hook / Part | What it does |
|---|---|
| `register` → `Registrar` | declares the block-entity type `rftoolsbuilder:builder_block_entity` over the Builder, attaches its `BlockEntityPart`, declares the block's 1-slot container, attaches the card's `ItemPart` |
| `component_types` | declares `rftoolsbuilder:shape_card`, the namespaced NBT component a card stack carries |
| `BlockEntityPart::block_entity` | gives the Builder an empty saved compound |
| `BlockEntityPart::block_entity_ticks` | `ticks = true` (`EntityBlock.getTicker`) |
| `BlockEntityPart::block_entity_tick` | the walk and the placement, `Unchanged` when idle |
| `ItemPart::use_on` | the card's corner authoring |
| `right_click_block` (cancel) | install the card / read out / take the card back |
| `register_commands` + `run_mod_command` | `/rftoolsbuilder status|shape|offset|reset` |

## Upstream source read

`ShapeCardItem.java` (`useOn`, `composeFormula`, `getMinCorner`/`getMaxCorner`,
`getClampedDimension`/`getClampedOffset`, `getShapeDataDimension`,
`MAXIMUM_COUNT`), `ShapeCardType.java`, `data/ShapeCardData.java` (every default),
`shapes/Formulas.java` (all ten `isInside` bodies and their `setup`),
`shapes/IFormula.java` (`correctFormula`, `isBorder`, `isVisible`),
`shapes/Shape.java`, `shapes/ShapeRotation.java`, `shapes/ShapeOperation.java`,
`shapes/ShapeModifier.java`, `blocks/BuilderTileEntity.java`
(`tickServer`, `checkStateServerShaped`, `handleBlockShaped`, `handleSingleBlock`,
`buildBlock`, `calculateBoxShaped`, `restartScan`, `nextLocation*`,
`createTakeableItem`, `findBlockTakeableItem`, `isEmptyOrReplacable`),
`BuilderConfiguration.java` (every number this slice uses),
`blocks/BuilderMode.java`, `blocks/AnchorMode.java`, `blocks/RotateMode.java`.

Behaviours extracted (not code): the ten formulas and their constants; the
per-axis parity nudge; the integer truncations; the border rule; the clamps;
the walk order; the eight-blocks-a-tick rate and the 100-look-up inner loop; the
two-corner arithmetic; the material search above-then-below; the default card.

## Slice boundary (out of scope, recorded)

Each line names where upstream implements it, so the next slice can start from
the artefact:

- **The quarry**: `ShapeCardType.CARD_QUARRY*` (`quarryBlock`,
  `silkQuarryBlock`, `commonQuarryBlock`, `clearOrDirtBlock`), the void list
  (`getVoidedBlocks`), the replacement block (`getQuarryReplace`), chunk
  loading (`chunkLoad`/`chunkUnload`). The **quarry card is a card type, not a
  shape**: it has no geometry of its own — it uses the card's dimension and
  replaces what it breaks. There is no "quarry shape" in the jar, so none is
  invented here.
- **No "frame" shape either**: the jar has solid and hollow only
  (`IFormula.correctFormula`); a `frame` would be a new shape, so it is not
  written. The brief's "box/hollow/frame" is answered with box + hollow and this
  note.
- **The movers** (`modules/mover/*`: `MoverTileEntity`, `MoverControllerTileEntity`,
  `EntityMovementLogic`, the four control pages, the status block, the vehicle
  builder and card).
- **The shields** (`modules/shield/*`: `ShieldProjectorTileEntity`,
  `ShieldTemplateBlock`, `ShieldingTileEntity`, the custom baked model, the
  damage types).
- **The space chamber** (`SpaceChamberControllerBlock`,
  `SpaceChamberControllerTileEntity`, `SpaceChamberRepository`) and the copy/move
  modes it feeds (`BuilderMode.MODE_COPY`/`MODE_MOVE`/`MODE_SWAP`, `sourceToDest`,
  the entity modes, `isMovable`, the support blocks).
- **Composer, projector and scanner** (`modules/scanner/*`: `ShapeHandler`,
  `ProjectorOpcode`, `ProjectorOperation`, the scan data managers): this is also
  why `SHAPE_COMPOSITION` and `SHAPE_SCAN` are not carried.
- **The GUI** (`client/GuiBuilder`, `GuiShapeCard`, and the mod's networking):
  replaced by `/rftoolsbuilder` commands and the action bar.
- **The liquid and pump cards** (`placeLiquidBlock`, `pumpBlock`,
  `consumeLiquid`, the tank capability).
- **The filter module** (`SLOT_FILTER`, `FilterModuleItem.getCache`): not
  carried, so the Builder's container declares **one** slot, not two.
- **Energy** (`GenericEnergyStorage`, `builderRfPerOperation`,
  `builderRfPerQuarry`, the infusion factor): the Builder needs no power in this
  slice; the card's shape builds the same way at any FE.
- **Builder flags**: silent, support mode, entity mode, loop mode, hilight mode,
  wait mode (the port always waits, which is upstream's default `wait = true`).
- **The progress HUD** (`showProgressHud`, the `IHudSupport` block) and the
  advancing/hilight renderer.

## Upstream defects fixed

**1. The shaped scan skipped whole columns of a shape that straddles a chunk
boundary.**

- **Evidence:** `BuilderTileEntity.nextLocationQuarry`
  (`src/main/java/mcjty/rftoolsbuilder/modules/builder/blocks/BuilderTileEntity.java:2198-2228`)
  advances `x` by one, but whenever it advances `z` or drops a layer it snaps
  **both** `x` and `z` down to the enclosing 16-block chunk
  (`new BlockPos((x >> 4) << 4, y, z + 1)` and
  `new BlockPos((x >> 4) << 4, y - 1, (z >> 4) << 4)`). `restartScan`
  (`:2099-2118`) starts at `(minBox.x, maxBox.y, minBox.z)`, so the first (top)
  layer is walked from the shape's own low corner and is complete, while every
  layer below it starts at the **chunk base** — and the part of the box lying in
  the chunk before that base is never looked at.
- **Measured:** a direct transcription of the walk over the default 5×5×5 box at
  a Builder on a chunk boundary (`min = (-2, 62, -2)`, `max = (3, 67, 3)`) visits
  **50 of the box's 125 cells** and then restarts at the same cell, so the other
  75 are never built, on any pass. A box inside one chunk (`min = (6, 62, 6)`)
  visits all 125 — which is why the defect reads as "sometimes the box has a
  missing wall". The shaped path is the one every shape card takes
  (`tickServer` → `checkStateServerShaped` → `handleBlockShaped` →
  `nextLocation` → `nextLocationQuarry`).
- **What the port does instead:** `runtime::next_cursor` keeps the jar's order —
  x fastest, then z, then y from the top down — over the **walked box itself**,
  and drops the chunk-base snapping (the remnant of the quarry's chunk loading,
  which this slice does not carry). Every cell of the box is looked at exactly
  once per pass; `the_walk_looks_at_every_cell_of_a_straddling_box` asserts 125
  of 125 for the straddling box, so reproducing the defect makes that test fail.
- **What a user notices:** a shape that crosses a chunk boundary is built
  completely instead of losing the cells in the earlier chunk.
- **Worth an upstream report:** yes — it is a one-line-shape change and it
  silently loses blocks. Not filed (the brief says file only when asked).

**2. The jar's top-of-scan layer is empty.** `restartScan` sets the cursor to
`(min.x, max.y, min.z)`, one block **above** the box (whose `max` is exclusive),
so every pass spends its first layer looking at cells no shape contains. The port
starts on the topmost cell of the box (`max.y - 1`). No user-visible behaviour
changes (a miss costs nothing but look-ups); recorded because it is a deliberate
difference from the artifact.

## Deviations (deliberate, not defects)

1. **One pass per card.** Upstream restarts the scan while the machine is enabled
   (`restartScan` when `scan == null`), so a Builder keeps re-filling what is
   broken. This slice completes the pass, records `done` and keeps saying
   `scan complete`; the saved state holds the card's fingerprint (shape,
   solidity, size, offset and turn), so writing any of those, installing another
   card or swapping it in the slot starts a fresh pass. No loop-mode switch is
   carried, so the endless loop would have no way to be turned off.
2. **Messages go to the action bar**, not chat (`Logging.message` upstream), so
   the two-corner authoring does not spam chat. Same words.
3. **No fake player.** Upstream places through `FakePlayerGetter` and runs the
   block's `setPlacedBy` and place sound; the SDK has no fake player, so the port
   writes the block's default state with `Level.set_block` (neighbours and
   clients updated) and plays the state's place sound. A block that does work in
   `setPlacedBy` (a bed's other half, a door's top) therefore appears only half
   placed — recorded as a gap, not silently.
4. **The card's turn and mirror** (`rotation`, `flipY`) are the jar's
   *composition* transform (`FormulaComposition.isInside`): upstream applies them
   to composer children only, so on a standalone card they are inert. The port
   applies them to the card's own shape when set (which a card can only get
   through `/rftoolsbuilder`… — there is no composer in this slice), and the
   default modifier is a no-op, so the default path is byte-identical to the
   jar's. Documented as an extension, not fidelity.
5. **The Builder's container has one slot** (the card), not two: the filter
   module is not carried.
6. **No energy.** Upstream's Builder consumes FE per block and scales with the
   infusion factor; this slice builds without power (the FE capability is a
   recorded gap).
7. **`wait` is always on.** Upstream's `BuilderData.DEFAULT_FLAGS` has
   `wait = true`; the port has no GUI to turn it off, so it always waits at a
   cell it has no block for rather than skipping it.

## Gaps

1. **No in-game run.** There is no `horizon-launcher` on this machine
   (`which horizon-launcher` is empty), so no game carrying the package can be
   built and `tools/e2e.sh` has **not** been run. The script is written for a
   launcher-built game (`horizon-launcher native-build … --trust-native-code`)
   and keeps a counterfactual (a furnace in the Builder's place) and a selftest
   (26 solid cells for a 27-cell box) so the first run is fail-capable.
2. **The quarry**, its resource scan, silk/fortune drops, the void list and the
   replacement block (upstream `ShapeCardType.CARD_QUARRY*`).
3. **The movers**, their controls and the vehicle builder (`modules/mover`).
4. **The shields** and their renderer (`modules/shield`).
5. **The space chamber**, copy/move/swap modes, support blocks and the entity
   modes (`SpaceChamberControllerBlock`, `BuilderMode`).
6. **Composer, projector and scanner**, hence `SHAPE_COMPOSITION` and
   `SHAPE_SCAN` shapes (`modules/scanner`).
7. **The GUI** and the mod's own networking (`client/Gui*`, `network/*`).
8. **The liquid/pump cards** and fluid handling (`placeLiquidBlock`,
   `pumpBlock`, `consumeLiquid`).
9. **The filter module** and filtered placement (`FilterModuleItem`).
10. **Energy**: no FE buffer, no per-block cost, no infusion factor
    (`GenericEnergyStorage`, `BuilderConfiguration.builderRfPer*`).
11. **`block_entity_set_placed_by` / `setPlacedBy`** — the pin has no fake
    player, so a placed block's own placement hook does not run (deviation 3).
12. **Chunk loading**: upstream force-loads the chunk it is quarrying
    (`quarryChunkloads`, `ticketController.forceChunk`); the port reads
    `is_loaded` and waits instead (a recorded difference, no ticket).
13. **The hilight mode, the progress HUD and the Builder's own renderer**
    (`updateHilight`, `IHudSupport`, `BuilderRenderer`).
14. **The two recipes need `rftoolsbase:machine_frame`** (sibling port) and the
    `c:ender_pearls` / `c:ingots/iron` tags (declared here as the family does).

## Verification

Every command ran through `ports/tools/build-slot.sh` (one lease per step); logs
in `ports/_shared/evidence/rftoolsbuilder/`. `/tmp` was full from other sessions'
work during this run, so `TMPDIR` was pointed at the port's own
`target/tmp` (nothing outside this port was deleted).

| Step | Command | Result | Log |
|---|---|---|---|
| runtime + glue tests | `cargo test --offline` | `33 passed; 0 failed` (25 geometry + 8 glue), doc-tests 0 | `test.log` |
| fail-capable check (geometry) | `RED_CHECK=1 cargo test` with `inside_shape` returning `false` | `16 passed; 17 failed` — the geometry tests catch a missing predicate | `red-check.log` |
| fail-capable check (glue) | `RED_CHECK=1 cargo test` with the card reader, the corner arithmetic and the state writer removed | `30 passed; 3 failed` | `red-check-glue.log` |
| lints | `cargo clippy --all-targets -- -D warnings` | `CLIPPY_EXIT:0`, 0 warnings | `clippy.log` |
| format | `rustfmt --edition 2024 --check src/lib.rs src/support.rs src/runtime.rs` | `FMT_EXIT:0` | `fmt.log` |
| SDK gate | `cargo horizon-mod check` | **CHECK_EXIT:0** — "RFTools Builder uses 421 items of the API (5 Part kinds, 1 associated item, 2 capabilitys, 86 fields or variants, 12 hooks, 20 hook arguments, 2 macros, 155 methods, 31 traits, 107 types). Made for 0b14f8a." + the `ModContainer` compatibility note | `check.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/rftoolsbuilder.hmod`, **BUILD_EXIT:0** | `build.log` |
| package, twice | the same command again on the same tree | `cmp` clean — **byte-identical** | `build2.log` |
| package check | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.0 (native-mod, format native-1/0b14f8a/e59da90, for 26.3)` / **51,246 bytes**, SHA-256 `231e5d00…5777be`, unsigned / **FILE_CHECK_EXIT:0** | `file-check.log` |
| uses | `cargo horizon-mod check rftoolsbuilder.hmod --uses` | **USES_EXIT:0** | `uses.json` |
| fits | `cargo horizon-mod fits rftoolsbuilder.hmod --build nightly` | `RFTools Builder:` / `0b14f8a  made for` / **FITS_EXIT:0** | `fits.log` |
| package hash | `sha256sum rftoolsbuilder.hmod` | `231e5d00d134f8a622d0568b2096ff7306cb18e82ee5ce984a78f296505777be` | `sha256.txt` |
| package entries | `unzip -Z1 rftoolsbuilder.hmod` | **31 files**: 22 content (`assets/` + `data/`), `horizon-mod.toml`, `Cargo.toml`, `src/{lib,runtime,support}.rs`, `uses.json`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`; **no `Cargo.lock`** | `package-entries.txt` |
| carry diff | jar vs port, size + SHA-256 per file | 6 verbatim, 2 content-equal recipes, 1 rewritten loot table, 13 declared, the rest listed as out of scope | `carry-diff.txt` |

The geometry's fail-capable cases (all in `src/runtime.rs`): exact counts
(3×3×3 box 27, its shell 26, 5×5×5 box 125, shell 98, sphere 81 at five and 19
at three, domes 51/51, cylinder 105, capped 105, cone 21, prism 35, torus 19,
heart 247), the sphere's mirror on all three axes, the heart's deliberate
asymmetry on all three (its three constants), the cone flipped, a turned box vs
a turned heart, the jar's heightless hollow cylinder (12 for 5×1×5, 36 for
5×3×5), the offset leaving the world and the clip to the build height, the
512/260 clamps and the 50,000,000-cell refusal with its words, the 128³ pass cap
and its readout, the walk over a straddling box, the scan's rebuild entry and its restart when a card changes under it, and
the byte-stable readout in all six of its branches.

**Not run:** the in-game e2e (gap 1).

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Publishing (first publication)

MIT, rank 0. The artifact is `ports/rftoolsbuilder/rftoolsbuilder.hmod`
(51,246 B, `231e5d00…5777be`), version **0.2.0** — the first publication of this
mod, so the entry is a first `[[versions]]` record rather than a supersede of an
earlier PR. Publishing still needs the maintainer's approval (ARCHITECTURE.md
§8), and the PR body must say what the slice implements and what it does not
(this file's slice boundary and gaps).

**First-publication changelog:**

- The Builder block and its shape card, on SDK 0.2 (`api = "0.2"`, game 26.3,
  Horizon `0b14f8a`, mixins `e59da90`).
- The ten shape formulas of the jar, solid and hollow, the 512/260 clamps and the
  50M-cell refusal.
- Card authoring by two corners and by `/rftoolsbuilder shape|offset|reset`.
- The Builder's top-down walk at 8 cells a tick, taking blocks from the
  container above or below it, with `/rftoolsbuilder status`.
- The upstream walk defect fixed (a shape across a chunk boundary is built
  completely).
- Not included: the quarry, movers, shields, space chamber, composer, projector,
  scanner, GUI, liquid cards, filter module, energy, and the composition/scan
  shapes.
