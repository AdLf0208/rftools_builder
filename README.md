# RFTools Builder for Horizon

A Horizon native mod (mod API 0.2) that ports **RFTools Builder 1.21-7.0.8**
(McJtyMods/RFToolsBuilder, MIT): the **Builder** block, its **shape cards** and the
**quarry card type**, with the shape geometry behind them.

## What this slice does

- **The shape card** (`rftoolsbuilder:shape_card_def`) carries a shape, a size,
  an offset and solid/hollow in the `rftoolsbuilder:shape_card` component. The
  shapes are the jar's own: box, sphere, top and bottom dome, cylinder, capped
  cylinder, prism, torus, heart and cone — each cell for cell the formula
  `Formulas.Formulas*.isInside` runs, floats and integer truncation included.
  Hollow is the jar's border test: a cell is in the shell when one of its six
  neighbours is outside.
- **Authoring**: sneak-right-click a Builder with the card, then click two
  corners (upstream's `ShapeCardItem.useOn`); or `/rftoolsbuilder shape <shape>
  <dx> <dy> <dz> [solid|hollow]` for a shape corners cannot express, and
  `/rftoolsbuilder offset <x> <y> <z>`.
- **The Builder** (`rftoolsbuilder:builder`) walks its card's shape from the top
  down, x fastest, and places a block into every cell `canBeReplaced`, eight
  cells a tick (`BuilderConfiguration.quarryBaseSpeed`). It takes each block from
  the container directly above it, or below if the one above has none, exactly as
  the jar's `createTakeableItem` does. With no block in reach it waits at that
  cell and says so.
- **The quarry card type**: `/rftoolsbuilder type quarry` (or `quarry_silk`,
  `quarry_fortune`, `quarry_clear`, `quarry_clear_silk`, `quarry_clear_fortune`)
  turns the same card into the jar's `ShapeCardType.CARD_QUARRY*` — it keeps the
  card's box and **breaks** what is in it instead of building. The block's drops
  (plain, silk touch or fortune III, the jar's `getHarvesterTool`) go into the
  container above the Builder, else below; the Builder's replacement block (the
  jar's `minecraft:dirt`, `/rftoolsbuilder replace <x> <y> <z> <block>`) is left
  behind, or air for a clearing card.
- **The void card type**: `/rftoolsbuilder type void` selects the jar's
  `ShapeCardType.CARD_VOID` (`resourceSuffix void`, the `voidBlock` handler, HUD
  `"    Void mode"`, `voidShapeCardFactor = 0.5`). It voids **every** breakable cell
  of the card's box — nothing is collected and the Builder's replacement block is
  left behind. The port reuses the same tested `QuarryStep::Void` path as the void
  list, so a void card over 27 stone leaves 27 dirt and the container above the
  Builder unchanged; a quarry card on the same rig still collects.
- **The void list**: `/rftoolsbuilder void <stone|cobble|dirt|sand|gravel|netherrack|endstone>`
  removes those blocks without collecting them, and `/rftoolsbuilder tagmatching
  on|off` extends each material to its block tag (`getVoidedBlocks`). The card's
  type, void list and toggle are component fields, so they survive being moved.
- **Chunk loading**: a quarry card registers its own ticket controller and,
  when it reaches a cell whose chunk is not loaded, force-loads that chunk
  (releasing the one it held before) so the pass can continue. The ticket is
  released when the pass ends or the card is removed, so another Builder in the
  same chunk must force it for itself. `/rftoolsbuilder forced <pos>` reads the
  chunks a Builder's ticket actually holds from the game's ticket table.
- **Energy**: the Builder declares an FE buffer (`builderMaxRF` 1,000,000, taking
  up to `builderRFPerTick` 20,000 a call) whose amount lives in its saved compound
  under `energy`, as upstream's `GenericEnergyStorage` does. A shaped cell costs
  `builderRfPerOperation` 500 FE, a quarry cell `builderRfPerQuarry` 300 FE, and a
  void card `builderRfPerOperation × voidShapeCardFactor` 250 FE; with no infuser
  the infusion multiplier is 1. A buffer that cannot pay suspends at that cell
  (`not enough power`) without placing, breaking or collecting anything.
  `/rftoolsbuilder energy <pos>` reads the buffer, lifetime spend and per-cell
  costs back; `/rftoolsbuilder charge <pos> <amount>` fills it through the block's
  energy capability (a harness stand-in for a cable).
- **The filter module and filtered placement**: `/rftoolsbuilder filter <block|tag>
  <allow|deny>` authors a filter on the held card. A `#tag` value matches any block
  in that tag, a bare value matches the block id; `allow` (the jar's `blacklist =
  false`) builds only matching cells, `deny` (`blacklist = true`) skips them. A
  filtered-out cell is charged the jar's `builderRfPerSkipped` 50 FE and is never
  built, exactly as `BuilderTileEntity` consumes that cost before `skip(data)`.
  `filter none` clears it (the jar's empty filter slot). The container declares
  the jar's second slot, `SLOT_FILTER`; this package's filter rides the card's
  data component rather than a separate filter-module item.
- **`/rftoolsbuilder status <pos>`** reads a Builder out: its card type, shape,
  size, offset, cell count, the replacement block, the void list, progress, and
  the first cell it refused and why; `/rftoolsbuilder energy <pos>` also reports
  the FE spent on filtered-out cells.
- The card goes into the Builder's card slot on a right click; an empty hand
  reads the Builder out; a sneaking empty hand takes the card back.

## What it does not do

The movers and their controls, the shields, the space chamber, the composer, the
projector, the scanner, the GUI screens and the liquid/pump cards are **not**
carried. The filter's multi-entry GUI is replaced by the single-entry
`/rftoolsbuilder filter` command. The per-block hardness multiplier, card-type cost
factors beyond the void card, and the infuser itself are also out of scope: the
port charges the jar's base per-cell cost. The `GAPS` list in `notes.md` names
each with the upstream file that implements it.

## Building

```sh
cargo horizon-mod check          # the SDK gate (format + manifest + code)
rm -f Cargo.lock && cargo horizon-mod build   # -> target/horizon-mod/rftoolsbuilder.hmod
cp target/horizon-mod/rftoolsbuilder.hmod ./rftoolsbuilder.hmod
```

`cargo test` runs the pure geometry and the glue's own tests without a game.
`.horizon/sdk` is fetched per clone (gitignored) and never packed.

## Licence

MIT, from the jar's own `META-INF/neoforge.mods.toml`; see [`LICENSE`](LICENSE)
and [`LICENSE-NOTICE.md`](LICENSE-NOTICE.md). Carried art is upstream's, with
its sizes and SHA-256 in `CONTENT.md`.
