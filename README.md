# RFTools Builder for Horizon

A Horizon native mod (mod API 0.2) that ports **RFTools Builder 1.21-7.0.8**
(McJtyMods/RFToolsBuilder, MIT): the **Builder** block and its **shape cards**,
and the shape geometry behind them.

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
- **`/rftoolsbuilder status <pos>`** reads a Builder out: shape, size, offset,
  cell count, progress, and the first cell it refused and why.
- The card goes into the Builder's card slot on a right click; an empty hand
  reads the Builder out; a sneaking empty hand takes the card back.

## What it does not do

The quarry (its resource scan, silk/fortune drops and replacement), the movers
and their controls, the shields, the space chamber, the composer, the projector,
the scanner, the GUI screens and the liquid/pump cards are **not** carried. The
`GAPS` list in `notes.md` names each with the upstream file that implements it.

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
