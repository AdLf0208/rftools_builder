# Carried content — RFTools Builder

The jar is `ports/_shared/licence-cache/rftoolsbuilder-1.21-7.0.8.jar` (1,054,376 B,
MIT). Every file below was compared with the jar: sizes and SHA-256 are in
`ports/_shared/evidence/rftoolsbuilder/carry-diff.txt`.

## Blocks

| id | template | why |
|---|---|---|
| `rftoolsbuilder:builder` | `minecraft:stonecutter` | the jar's blockstate is the four-way horizontal `facing`; `stonecutter` carries exactly that state (the sibling `rftoolsbase:machine_infuser` uses the same template). Properties from McJty's `defaultProperties()`: hardness 2.0, blast resistance 2.0, correct tool required, metal (`minecraft:iron_block`) sounds, full-cube outline and collision. |

`assets/rftoolsbuilder/blockstates/builder.json` is the jar's own file, byte for
byte (367 B, `51e6e01c…aaf8f6b54`): `facing=east` is the model turned 90°,
north 0°, south 180°, west 270°. The model is the jar's
(`models/block/builder.json`, 415 B) — a vanilla `cube` whose north face is
`rftoolsbuilder:block/machinebuilder` and whose other faces are the RFToolsBase
machine casing.

## Items

| id | template | notes |
|---|---|---|
| `rftoolsbuilder:shape_card_def` | `minecraft:paper` | `max_stack_size: 1`, as upstream's `stacksTo(1)`; its shape lives in the `rftoolsbuilder:shape_card` component. |

The quarry (0.2.3) and the void card (0.2.4) add **no content**: the jar has nine
shape-card items and this port carries one, so the card *type* (`ShapeCardType`) rides
the same component as the shape (`type`, `voiding`, `tagMatching`), and the replacement
block is the Builder's own saved data. No texture, model, lang line or recipe is added
for either card type.

## Textures

| file | source | size | SHA-256 |
|---|---|---:|---|
| `textures/block/machinebuilder.png` | the jar | 857 B | `cb3671ca…3d70be5` |
| `textures/item/shapecarditem.png` | the jar | 194 B | `fda63a33…11bb57` |
| `textures/block/base/machinebottom.png` | RFToolsBase 1.21-6.0.11 (MIT, same author) | 471 B | `bb8ecd76…9d7272e7` |
| `textures/block/base/machineside.png` | RFToolsBase 1.21-6.0.11 | 549 B | `19f7bcb4…96690272d` |
| `textures/block/base/machinetop.png` | RFToolsBase 1.21-6.0.11 | 475 B | `6b07d177…473336460` |

The three casing textures travel with this port, as they do with
`rftoolsbase`, `rftoolsstorage` and `rftoolsutility`, so the Builder renders
without RFToolsBase installed.

## Data

- `data/rftoolsbuilder/horizon/block/builder.json` — the block declaration
  (`stonecutter` template, `item: true`).
- `data/rftoolsbuilder/horizon/item/shape_card_def.json` — the card's item
  declaration.
- `data/rftoolsbuilder/horizon/creative_tab/rftoolsbuilder.json` — the tab, with
  the two ids this slice carries.
- `data/rftoolsbuilder/loot_table/blocks/builder.json` — the jar's table with
  `copy_name`; the `copy_components` entry for `mcjtylib:*` and
  `rftoolsbuilder:builder_data` is dropped (those components are not carried;
  the card slot spills on a break like any container).
- `data/rftoolsbuilder/recipe/builder.json`,
  `data/rftoolsbuilder/recipe/shape_card_def.json` — the jar's two recipes with
  their patterns and results unchanged; their **ingredients are rewritten to the
  26.3 dialect** (bare ids and `#tag` strings instead of the 1.21.1
  `{"item": …}` / `{"tag": …}` objects, which 26.3 refuses — see `notes.md`,
  *Carry defect fixed*). The Builder's needs `rftoolsbase:machine_frame`, a
  sibling port (recorded as a dependency, and proven in game on a game carrying
  both packages).
- `data/c/tags/item/ender_pearls.json`, `data/c/tags/item/ingots/iron.json` —
  the two `c:` tags the recipes name.
- `data/minecraft/tags/block/mineable/pickaxe.json`,
  `data/minecraft/tags/block/needs_iron_tool.json` — the Builder is a pickaxe
  block that wants an iron one, as upstream's `requiresCorrectToolForDrops`.

## Not carried (the slice boundary)

The other 25 blockstates (composer, projector, scanner, mover and its four
controls + status, the space chamber and its controller, the five shield parts,
the three shield template colours, support block, vehicle builder), their 73
models and 69 textures, the 21 other loot tables, the 40 other recipes, the 42
advancements, the GUI textures, the sounds, and the Patchouli guide — see the
"jar families not carried" table in `carry-diff.txt` and the gaps in
`notes.md`.
