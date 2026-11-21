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
cards) and slice 2 (the carried recipes repaired to the 26.3 dialect and proven
in game) are implemented, their 29 runtime tests and 9 glue tests (38 in all)
are green, the SDK gate is green, and the package is **verified in a
launcher-built game**: the modded leg passes, the unmodded control and the
selftest both fail. The gap list below is the rest of the mod.

Package at **0.2.2**: **56,154 B**, SHA-256
`f09026b0f9f9cc91a2204f9231b07a595e62cda9f18e80ab46601ad049c5e872`, built twice
byte-identically against SDK **`0a875da9…`** + mixins `e59da90…` (`check` exit 0,
`fits` `made for` at the pin and `fits` at `0b14f8a`). The command → exact-output →
log table for every number below is [`VALIDATION.md`](VALIDATION.md);
the canonical logs are `sdk-test.log`, `sdk-fits.log`, `sdk-fits-other.log`, and the
0.2.0 measurements are retired in `evidence/rftoolsbuilder/retired-0.2.0/`.

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

## Review fixes (2026-11-18) — the registry's source review, package 0.2.1

The reviewer's file is `var/review/1276-rftoolsbuilder.md` (**held: this blocked listing**). New
package: **`rftoolsbuilder.hmod` — 56,192 B, sha256
`da48b02d137e8d6cbf8df87536feb417ed7e4b02761f84b9e620cef314d627a6`**, rebuilt twice
byte-identically, `check` = `rftoolsbuilder 0.2.1 (native-mod, …)`, `fits --build nightly` =
`0b14f8a made for`, **38 unit tests**. **The in-game leg is green**: `PASS` + `EXIT:0`, with a
control and a selftest leg that both fail without it (item 6).

### 1. `status` enumerated a player-configured shape synchronously (unbounded work)

**Reproduced** (`src/runtime.rs:877` in 0.2.0 → `Card::count(MAX_PASS_CELLS)`): a non-box shape
enumerates its **whole validated box**, up to the 50,000,000-cell card limit, to collect at most
2,097,152 cells. A sparse shape (a sphere, a torus) therefore asked `contains` ~124 million times on
the server thread for one chat command.

**Fixed** with a **work budget and a cache**, the two things the review asked for:

- `Card::count_within(cap, work)` and `Card::cells_within(origin, cap, work)` bound the number of
  **candidates inspected**, not just the cells kept: [`STATUS_CANDIDATES`] = 250,000. A box is
  still answered from its dimensions and inspects nothing.
- the pass computes its count **once** when it starts and stores it in the block entity
  (`cells`, `cellsMore`), so a repeated `/rftoolsbuilder status` reads the cache and costs nothing;
  `Status::cells` carries it and `status_line` prints it (the `(first N cells)` suffix now names the
  number actually counted).
- a count cut short by the budget prints `(first 250000 cells)` — "at least this many", never a
  silent half-count.

**Pinned by** `a_status_count_is_bounded_on_a_huge_sparse_shape` (a 499³ sphere: the bounded answer
stops at the budget, says there are more, and a doubled budget counts more — so the number is the
budget's, not a constant; a 300³ box answers exactly with a work budget of 0) and
`the_readout_prints_the_count_it_was_given` (byte-stable with and without a cached count).

### 2. Material-consuming placements could write protected cells

**Reproduced** (`src/support.rs:656` in 0.2.0): `place()` took a block from the container and wrote
it with the actor-free `LevelRef::set_block`, which performs **no** protection check.

**Fixed** with a gate that runs **before anything is read or taken**: `cell_protected()` checks the
world's bounds and the **world border** (`LevelRef::world_border`), and a protected cell is *skipped*
(`progress.skipped`), never built and never charged a block. `RefusalReason::Protected` names it in
the readout. The pin exposes **no** `ServerLevel.mayInteract` helper and no spawn-protection read
(`surface.json` has no such identifier; the engine keeps the radius in its LAN host and only for a
player's own interaction), so spawn protection is a recorded gap rather than a claim — see
`## Gaps`.

**Pinned by** `a_protected_cell_has_words_a_player_reads`, and in game by the new e2e phase: with
`worldborder set 1` and a card offset 8 blocks away, the Builder builds **0** cells out there and
the chest above it keeps **every** block it held.

### 3. `status` disclosed arbitrary loaded block ids

**Reproduced** (`src/support.rs:1092` in 0.2.0): with no Builder at the coordinates the command
answered `minecraft:furnace: not a builder`, naming whatever a player pointed it at.

**Fixed**: the refusal is now the generic `not a builder`. **Pinned by** the counterfactual leg of
the e2e, which now also fails if the reply contains `minecraft:furnace`.

### 4. Entry `kind`, upstream link, licence

The **package** side was already right and is unchanged: `horizon-mod.toml` has `[native]` (game
26.3, horizon `0b14f8a…`, mixins `e59da90…`), `license = "MIT"`, `homepage` and `issues` pointing at
`https://github.com/McJtyMods/RFToolsBuilder`, and `check` reports `native-mod`. The entry's
`kind = "native-mod"` and the original-project link are the orchestrator's edit.

### 5. A build defect found on the way: the `horizon-core` path dependency

The port depended on `.horizon/sdk/api/crates/horizon-core` for `JavaString` (a compound's key
type). `horizon-launcher native-build` refused it — `error: No such file or directory (os error 2)` —
because the launcher stages only the API crate. The key is now taken from `Tag::String`'s own
payload (the `key!` macro, the same conversion `moderndynamics` uses), and the package has the one
SDK dependency again.

### 6. The card slot moved to saved data; the "73 of 98" was the scenario's own stone

**The card slot's seam (fixed).** The card used to live only in the container slot, and a click-time
`BlockEntityRef::set_item_slot` was visible to that click's action bar and **invisible to the next
command's read** (`no shape card` for an installed card); `set_changed` did not close it — the same
seam `integratedcrafting` hit. The card now lives in the block entity's **saved data**
(`cardItem` + `cardData`: the item id and the card's shape, size, offset, turn and corner
selection) and is **mirrored into the container slot every tick**, an outside change folded back (a
slot holding an id other than `cardShown`, the id last mirrored, wins). `install`, `take_card` and
`status_at` all use the saved form; a second card swaps and the old one goes back to the player.

**The shortfall was not the walk (measured, then fixed in the scenario).** 0.2.1's first green
attempt read `98 cells, 73 placed, scan complete`, and 73 = 98 − 25. A unit reproduction of the
tick's walk — `a_second_pass_over_a_hollow_box_visits_and_builds_every_cell`, a completed solid
3×3×3 pass followed by a hollow 5×5×5 pass on the same Builder, driving `Scan::start`/`advance` with
the same per-tick budgets — visits **all 98 shell cells and builds all 98**, so the walk was
exonerated before anything was changed. Instrumenting the run then named the cells
(`RFB-SKIP not-replaceable [x, y, z]`, probe since removed): the hollow box's **bottom face lay on
the arena platform**, 25 stone cells a Builder must not replace. Lifting the box with
`/rftoolsbuilder offset 0 2 0` left **6** cells of the *first pass's* own stone in the new shell's
bottom layer, and the readout became the correct `98 cells, 92 placed, scan complete`:
92 = 98 − 6. The port never under-visits; it skips what is already there.

The phase now proves exactly that: `98 cells` (a solid 5×5×5 would be 125), **92 placed**, and the
chest above losing exactly **92** blocks, so a walk that under-visits still fails.

**Measured, all three legs** (`ports/_shared/evidence/rftoolsbuilder-e2e/`):

| leg | command | result |
|---|---|---|
| modded | `ports/tools/launcher-game.sh run rftoolsbuilder` | **PASS** + `EXIT:0` |
| control | `GAME=$CONTROL_GAME` (the same Nightly, no package) | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` |
| selftest | `SELFTEST=1` | `FAIL: built 24 cells, expected 25` + `EXIT:1` |

`check-e2e-logs.py rftoolsbuilder-e2e --strict` → `logs=3 ok=3 missing-exit=0 bad-exit=0`.

The leg's numbers, as the log prints them: `Box (Solid) 3x3x3 … 27 cells, 24 placed, scan complete`
with the chest `64 → 40`; `Box (Hollow) 5x5x5 offset 0 2 0 … 98 cells, 92 placed, scan complete`
against a full chest (exactly 92 taken); a shape wholly outside the world border `0 placed` with the
chest untouched; the card taken back by a sneak click; the volume identical after a relaunch.

### Ladder (0.2.1)

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --lib --offline` | **38 passed**, 0 failed (was 27; +11) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings |
| formatting | `cargo fmt --check` | clean |
| package | `cargo horizon-mod build` twice | `56,192 B`, sha256 `da48b02d…14d627a6`, **byte-identical** |
| registry check | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.1 (native-mod, …)`, EXIT 0 |
| fits | `cargo horizon-mod fits rftoolsbuilder.hmod --build nightly` | `0b14f8a made for`, EXIT 0 |
| in-game | `ports/tools/launcher-game.sh run rftoolsbuilder` | **PASS** + `EXIT:0`; control and selftest `FAIL` + `EXIT:1`; `check-e2e-logs.py --strict` exit 0 (item 6) |

## Carry defect fixed (0.2.2): both carried recipes were dead data

The two recipe files were carried from the 1.21.1 jar with their **object-form
ingredients** (`{"item": "minecraft:bricks"}`, `{"tag": "c:ender_pearls"}`).
Horizon 26.3 reads a shaped recipe's ingredient as a **string** — a bare item id
or a `#tag` (`data/minecraft/recipe/iron_block.json`: `"#": "minecraft:iron_ingot"`)
— and refuses the object. The server log said it plainly at launch:

```
== game log (rftoolsbuilder 0.2.2 before the fix): every rejected recipe ingredient ==
recipes: invalid ingredient (unresolved tag or malformed input): {"item":"minecraft:bricks"}
recipes: invalid ingredient (unresolved tag or malformed input): {"item":"minecraft:paper"}
recipes: invalid ingredient (unresolved tag or malformed input): {"item":"minecraft:redstone"}
recipes: invalid ingredient (unresolved tag or malformed input): {"item":"rftoolsbase:machine_frame"}
recipes: invalid ingredient (unresolved tag or malformed input): {"tag":"c:ender_pearls"}
recipes: invalid ingredient (unresolved tag or malformed input): {"tag":"c:ingots/iron"}
```

and the grid was right while the result slot stayed empty (`e2e-red-recipe.log`):

```
  shape card: 4 paper + 2 bricks + 1 iron ingot (c:ingots/iron) + 2 redstone -> null
    grid: [{"count": 1, "item": "minecraft:paper", "slot": 100}, {"count": 1, "item": "minecraft:bricks", "slot": 101}, …]
  builder: 4 bricks + 1 ender pearl (c:ender_pearls) + 3 redstone + machine frame -> null
    grid: [{"count": 1, "item": "minecraft:bricks", "slot": 100}, … {"count": 1, "item": "rftoolsbase:machine_frame", "slot": 104}, …]
FAIL: a carried recipe did not craft
```

**Fixed:** bare item ids and `#tag` strings, the shape the sibling ports
(`rftoolsstorage`, `rftoolsutility`) already use:

| file | before | after |
|---|---|---|
| `recipe/builder.json` | `{"item": "minecraft:bricks"}` / `{"item": "rftoolsbase:machine_frame"}` / `{"tag": "c:ender_pearls"}` / `{"item": "minecraft:redstone"}` | `"minecraft:bricks"` / `"rftoolsbase:machine_frame"` / `"#c:ender_pearls"` / `"minecraft:redstone"` |
| `recipe/shape_card_def.json` | `{"item": "minecraft:bricks"}` / `{"tag": "c:ingots/iron"}` / `{"item": "minecraft:paper"}` / `{"item": "minecraft:redstone"}` | `"minecraft:bricks"` / `"#c:ingots/iron"` / `"minecraft:paper"` / `"minecraft:redstone"` |

Nothing else in either file changed: the patterns, the results and the ids are
the jar's. This is a **carry** defect, not an upstream one — the Java mod is
correct for 1.21.1; the carry pipeline carried the dialect along with the data.

**Two proofs, both fail-capable:**

1. `tools/check_recipes.py` — a data guard over every `data/*/recipe/**/*.json`:
   an ingredient must be a string (or a list of strings), the result must carry
   a string id and a count of 1–99. Red before the fix
   (`recipe-guard-red.log`): `FAIL: data/rftoolsbuilder/recipe/builder.json: key.B
   is an object with item: {"item": "minecraft:bricks"}` ×8 in 2 files; green
   after (`recipe-guard-green.log`): `recipes PASS: 2 carried recipe files use
   26.3 bare item/tag ingredients`.
2. `tools/craft_check.py`, called by `tools/e2e.sh` — both recipes are crafted in
   a real crafting table, the result's id and count are asserted, the stack is
   shift-clicked into the inventory, the grid must be empty and a distinctive
   ingredient consumed; a wrong-ingredient control per recipe must leave the
   result empty; and the game log must hold **no** `recipes: invalid ingredient`
   line. The unmodded control and the selftest legs fail, so this cannot pass
   vacuously.

The Builder's recipe takes `rftoolsbase:machine_frame`, so the leg runs on a game
carrying **both** packages — upstream declares rftoolsbase a required dependency
and it is in the registry. `ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh`
assembles it: a scratch copy of `ports/rftoolsbase` (its `[native] horizon` moved
to this pin in the copy only — the port itself is read-only) and this port, built
together with `horizon-launcher native-build <folder>`. Without rftoolsbase the
frame cannot be given and `craft_check.py` says so instead of passing a half-check.

## Slice 2 (0.2.2, 2026-11-21) — the in-game leg and its coverage

The leg is `tools/e2e.sh` on the launcher-built two-package game, in The End for
the item-count and break phases (no ambient drops, and no spawn protection to
refuse a non-op player's break). Quoted from `e2e-package.log`:

```
== the carried recipes craft in a real crafting table ==
  shape card: 4 paper + 2 bricks + 1 iron ingot (c:ingots/iron) + 2 redstone -> {"count": 1, "item": "rftoolsbuilder:shape_card_def", "slot": 200}
  builder: 4 bricks + 1 ender pearl (c:ender_pearls) + 3 redstone + machine frame -> {"count": 1, "item": "rftoolsbuilder:builder", "slot": 200}
  shape card with a stick where the paper goes -> null
  builder with a stick where the ender pearl goes -> null
recipe PASS: both carried shaped recipes craft with their declared id and count, their ingredients are consumed, the grid empties, and both wrong-ingredient controls leave the result empty
recipe log: no 'invalid ingredient' line
…
status: Builder at 20 102 12: Box (Solid) 3x3x3 offset 8 0 0, 27 cells, 0 placed, scan complete; chest 256 -> 256
…
break reply: {"block":"minecraft:air","block_before":"rftoolsbuilder:builder[facing=north]","drops":{"rftoolsbuilder:builder":1},…}
reset: Card reset to the jar's default: Box (Solid) 5x5x5 offset 0 0 0
cards in the inventory after reset: 1
PASS: both carried recipes craft in a real crafting table (…), the Builder fills its 3x3x3 box from the chest above it (24 placed, 64 -> 40), a hollow 5x5x5 is a 98-cell shell (92 placed against a full chest), a box outside the world border builds 0 and spends nothing, the Builder's loot table drops it, the card goes in, comes back out, resets to the 5x5x5 default, and the placed volume survives a relaunch
EXIT:0
```

| leg | command | result | log |
|---|---|---|---|
| package | `GAME=<bundle game> tools/e2e.sh` | **PASS** + `EXIT:0` | `e2e-package.log` |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME` | `FAIL: cannot give rftoolsbuilder:builder: this game does not carry the package` + `EXIT:1` | `e2e-control.log` |
| fail-capable | `SELFTEST=1` | `FAIL: built 24 cells, expected 25` + `EXIT:1` | `e2e-selftest.log` |
| red proof (before the fix) | the same leg on 0.2.2 with object-form ingredients | `FAIL: a carried recipe did not craft` + `EXIT:1`, log lines above | `e2e-red-recipe.log`, `game-log-red-recipe.log` |
| log gate | `check-e2e-logs.py rftoolsbuilder-e2e --strict` | `logs=7 ok=7 missing-exit=0 bad-exit=0` | — |

**Coverage — every family this package ships** (ruling 2026-11-20):

| family | status | proof |
|---|---|---|
| the two carried shaped recipes (`builder`, `shape_card_def`) | **tested** | `craft_check.py`: both craft their declared id and count, ingredients consumed, grid empty; a wrong-ingredient control each; no `invalid ingredient` line in the log |
| the two `c:` tags (`c:ender_pearls`, `c:ingots/iron`) | **tested** | a recipe whose key is the tag crafts — an empty or unresolved tag refuses the recipe at load |
| `rftoolsbuilder:builder` block and its `facing` | **tested** | placed and read back with a `facing`; an unknown id is refused by `give` |
| `rftoolsbuilder:shape_card_def` item and its `shape_card` component | **tested** | given, written by `/rftoolsbuilder shape|offset|reset`, installed by a click, taken back by a sneak click |
| the Builder's block entity and its card slot | **tested** | the installed card drives a 3×3×3 pass (24 placed of 27) and a hollow 5×5×5 shell (92 of 98), the chest loses exactly the blocks placed |
| the walk and the placement | **tested** | 24 placed + 3 skipped = 27; 92 against a full chest; a box outside the world border places 0 and spends nothing |
| the block's loot table | **tested** | broken in survival with an iron pickaxe in The End: `drops: {"rftoolsbuilder:builder": 1}`, the block becomes air |
| `/rftoolsbuilder status` | **tested** | six readouts asserted byte-exactly (`no shape card`, `27 cells, 24 placed`, `98 cells, 92 placed offset 0 2 0`, `27 cells, 0 placed`, `not a builder`, the reset default) |
| `/rftoolsbuilder shape`, `offset`, `reset` | **tested** | `Card set to Box (Solid) 3x3x3`, `Card offset set to 0 2 0`, `Card reset to the jar's default: Box (Solid) 5x5x5 offset 0 0 0` |
| the 3×3×3 / 5×5×5 cubes, the arena | — | scenario scaffolding, not a mod family |
| the quarry, movers, shields, space chamber, composer/projector/scanner, GUI, liquid cards, filter module, energy, chunk loading, HUD | **not ported** | gaps 2–13 below: each names the upstream class it waits on |

### Ladder (0.2.2)

Every step ran through `ports/tools/build-slot.sh` (one lease per step,
`CARGO_INCREMENTAL=0`, no `--target-dir`); `BUILD_LOCAL=1` for the two steps that
need `cargo horizon-mod` (the remote runner has no such subcommand installed).

| Step | Command | Result | Log |
|---|---|---|---|
| recipe guard (RED) | `python3 tools/check_recipes.py .` before the fix | 8 old-shape ingredients in 2 files, exit 1 | `../_shared/evidence/rftoolsbuilder/recipe-guard-red.log` |
| recipe guard | the same after the fix | `recipes PASS: 2 carried recipe files use 26.3 bare item/tag ingredients` | `recipe-guard-green.log` |
| unit tests | `cargo test --offline` | **38 passed**, 0 failed (`unittests src/lib.rs`: `test result: ok. 38 passed; 0 failed`; doc-tests `0 passed`), exit 0 | `sdk-test.log` |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings | `clippy.log` |
| formatting | `rustfmt --edition 2024 --check src/{lib,support,runtime}.rs` | `FMT_EXIT:0` | `fmt.log` |
| SDK gate | `cargo horizon-mod check` | `Made for 0a875da.` + `ModContainer: built with ..`, exit 0 | `check.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/rftoolsbuilder.hmod` | `build.log` |
| package twice | the same again | **`cmp` identical** | `build2.log` |
| registry check | `cargo horizon-mod check rftoolsbuilder.hmod` | `rftoolsbuilder 0.2.2 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, 56,154 B, `f09026b0…e872`, exit 0 | `file-check-fits.log` |
| fits, the pin | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0a875da91dbb0fdcb7ba8c42c11341aa229af51a` | `0a875da  made for`, exit 0 | `sdk-fits.log` |
| fits, **a different build** | `cargo horizon-mod fits rftoolsbuilder.hmod --build 0b14f8acfd97fa2c750e42795249dcc981178ca4` | `0b14f8a  fits`, exit 0 | `sdk-fits-other.log` |
| package gate | `python3 ports/tools/verify-sdk-package.py rftoolsbuilder` | **1/1** (`31 entries`, no packed `Cargo.lock`) | — |
| in-game | the three legs above | `PASS` + `EXIT:0`, control `EXIT:1`, selftest `EXIT:1` | `../_shared/evidence/rftoolsbuilder-e2e/*.log` |

## Gaps

1. ~~**No in-game run.**~~ **Closed 2026-11-21**: `ports/tools/launcher-game.sh` and
   `horizon-launcher 0.2.92` build a game at this pin, and the leg runs on it
   (`e2e-package.log` `PASS` + `EXIT:0`, control and selftest `EXIT:1`). The
   script keeps the unmodded-game control, the `SELFTEST=1` leg and a
   `COUNTERFACTUAL=1` furnace leg, so a first run is fail-capable.
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
    **Proven 2026-11-21** on a game carrying both packages
    (`ports/_shared/evidence/rftoolsbuilder-e2e/bundle-game.sh`); the ingredient
    itself is consumed by the craft.

## Verification

Slice 1's run (0.2.0) is the table below; the current **0.2.2** numbers are in
*Ladder (0.2.2)* above.

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
| fits | `cargo horizon-mod fits rftoolsbuilder.hmod --build nightly` | `RFTools Builder:` / `0b14f8a  made for` / **FITS_EXIT:0** | `retired-0.2.0/fits.log` (0.2.0; that run's "nightly" was `0b14f8a`) |
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

**In game:** the three legs of the slice-2 run above (`PASS` + `EXIT:0`, control
and selftest `EXIT:1`) against the launcher-built two-package game at this pin.

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Publishing

MIT, rank 0. The current artifact is `ports/rftoolsbuilder/rftoolsbuilder.hmod`
(**56,154 B**, `f09026b0f9f9cc91a2204f9231b07a595e62cda9f18e80ab46601ad049c5e872`),
version **0.2.2** — an **update** of the entry this campaign owns (`Add
rftoolsbuilder 0.2.0`, PR #1276, then 0.2.1 under review), so the PR title is
`Update rftoolsbuilder 0.2.2` and the body names the superseded PR. Publishing
still needs the maintainer's approval (ARCHITECTURE.md §8).

**Changelog (0.2.2) — supersedes 0.2.1:**

- **Both carried recipes craft.** They were carried in the 1.21.1
  object-form-ingredient shape (`{"item": …}`, `{"tag": …}`), which 26.3 rejects
  (`recipes: invalid ingredient`), so both were dead data — the grid filled and
  the result slot stayed empty. They now use bare item ids and `#tag` strings,
  and both were crafted in game with their ingredients consumed.
- `tools/check_recipes.py`: a data guard that fails on an old-dialect ingredient,
  so the regression cannot come back silently.
- The in-game leg now also proves the block's loot table, the protected-cell rule
  (a box outside the world border builds 0 and spends nothing) and
  `/rftoolsbuilder reset`, and it runs on a game carrying `rftoolsbase` as well
  (required for the Builder's machine-frame ingredient).
- Pin moved to SDK **`0a875da9…`** (mixins unchanged); 38 tests, clippy
  `-D warnings` clean, `check` 0, `fits` `made for` at the pin and `fits` at
  `0b14f8a`; in game `PASS` + `EXIT:0`, unmodded control `EXIT:1`,
  `SELFTEST=1` `EXIT:1`.

**First-publication changelog (0.2.0, for reference):**

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
