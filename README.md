# RFTools Builder — a Horizon port

**RFTools Builder** is a Minecraft mod. The RFTools Builder, Shield, Projector, Scanner,. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [McJtyMods/RFToolsBuilder](https://github.com/McJtyMods/RFToolsBuilder) — licence **MIT**.

| | |
|---|---|
| Upstream mod | RFTools Builder ([source](https://github.com/McJtyMods/RFToolsBuilder)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `rftoolsbuilder.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What this slice does

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

## Not yet ported

- **No in-game run.** There is no `horizon-launcher` on this machine
- **The quarry**, its resource scan, silk/fortune drops, the void list and the
- **The movers**, their controls and the vehicle builder (`modules/mover`).
- **The shields** and their renderer (`modules/shield`).
- **The space chamber**, copy/move/swap modes, support blocks and the entity
- **Composer, projector and scanner**, hence `SHAPE_COMPOSITION` and
- **The GUI** and the mod's own networking (`client/Gui*`, `network/*`).
- **The liquid/pump cards** and fluid handling (`placeLiquidBlock`,

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `rftoolsbuilder.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

RFTools Builder was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/McJtyMods/RFToolsBuilder

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run rftoolsbuilder` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/rftoolsbuilder/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
