// RFTools Builder for Horizon — the shape geometry and the Builder's scan (pure).
//
// Ported from RFToolsBuilder 1.21-7.0.8 (`McJtyMods/RFToolsBuilder`, MIT),
// Minecraft Java 1.21.1 NeoForge, branch `1.21_neo` at commit
// `4090494663bd15e5776d9e407a59ec982129ef1f` (`gradle.properties`
// `version=1.21-7.0.8`, the jar's own version). The upstream source was read for
// behaviour only; this is a clean-room Rust rewrite.
//
// A shape card names a shape, a size, an offset and whether the shape is solid or
// hollow, and the Builder places a block into every replaceable cell of that shape.
// A **quarry** card is a card *type*, not a shape (`ShapeCardType.CARD_QUARRY*`): it
// keeps the card's box and breaks what is in it, collecting the drops and leaving the
// Builder's replacement block (or air, when the card clears); the card's void list
// removes blocks without collecting them. The movers, the shields, the space chamber
// and the composer are not carried (notes.md).
//
// Everything here is pure: no `mod_api` type, no host, no world. The glue in
// `support.rs` reads the game, calls these rules and writes the result back.

/// The block this slice implements, exactly as the content declares it.
pub const BUILDER: &str = "rftoolsbuilder:builder";
/// The one shape card this slice carries (`rftoolsbuilder:shape_card_def`).
pub const SHAPE_CARD: &str = "rftoolsbuilder:shape_card_def";
/// The item component that carries a card's shape (`component_types`).
pub const COMPONENT: &str = "rftoolsbuilder:shape_card";

/// `BuilderConfiguration.maxBuilderDimension`: the largest side a card's
/// dimension is clamped to before the shape is built.
pub const MAX_DIMENSION: i32 = 512;
/// `BuilderConfiguration.maxBuilderOffset`: the farthest a card's offset is
/// clamped to, either way.
pub const MAX_OFFSET: i32 = 260;
/// `ShapeCardItem.MAXIMUM_COUNT`: a card whose shape holds more cells than this
/// is refused (the GUI shows `#Blocks: ++` past it).
pub const MAX_CARD_CELLS: u64 = 50_000_000;
/// `maxSpaceChamberDimension^3` (128³): the most cells one Builder pass keeps,
/// the cap upstream passes to `composeFormula` from `getCachedBlocks`.
pub const MAX_PASS_CELLS: u64 = 2_097_152;
/// `BuilderConfiguration.quarryBaseSpeed`: blocks the Builder handles in one
/// tick at infusion 0 (the shaped path's `checkStateServerShaped`).
pub const BLOCKS_PER_TICK: u32 = 8;
/// The most shape cells one `/rftoolsbuilder status` readout may inspect. The card limit allows
/// 50,000,000, and a sparse shape (a shell, a torus) has to look at each of them to count its own:
/// enumerating that on the server thread stall the tick (review 1276-rftoolsbuilder). A status
/// answer therefore stops after this many candidates and says the count is at least what it found.
/// A box answers from its dimensions and never enumerates.
pub const STATUS_CANDIDATES: u64 = 250_000;

/// `100 * quarryBaseSpeed`: the most cells one tick looks at. Upstream's
/// `handleBlockShaped` looks at up to 100 cells per call and is called
/// `quarryBaseSpeed` times a tick, so a scan never touches more than this.
pub const LOOKUPS_PER_TICK: u32 = 800;

/// `ShapeCardData.DEFAULT_DIMENSIONS`: a fresh card is a 5×5×5 box.
pub const DEFAULT_DIMENSION: [i32; 3] = [5, 5, 5];

/// `BuilderConfiguration.quarryReplace`'s default: what a quarry leaves behind when the
/// card is not clearing. The jar's value is a server config; this port keeps it per Builder
/// (there is no config file in the package format yet — `notes.md`).
pub const QUARRY_REPLACE: &str = "minecraft:dirt";
/// The tool the quarry harvests with. Upstream's `SuperHarvestingTool` is
/// `new DiggerItem(Tiers.NETHERITE, BlockTags.MINEABLE_WITH_PICKAXE, …)`, so a netherite
/// pickaxe is the same item in everything the loot tables read.
pub const QUARRY_TOOL: &str = "minecraft:netherite_pickaxe";
/// `getHarvesterTool(…, …, fortune)`: the level a fortune quarry's tool carries
/// (`int fortune = getCardType().isFortune() ? 3 : 0`).
pub const FORTUNE_LEVEL: u32 = 3;
/// `BuilderConfiguration.voidShapeCardFactor` (0.5, range 0..1,000,000). The config's own comment
/// attaches it to "the RF per operation of the builder", so this port's void card costs
/// `builderRfPerOperation × 0.5` = 250 FE a cell (`cell_cost`). Two readings of the jar disagree
/// with that comment and are recorded in `notes.md`: `ShapeCardType`'s `CARD_VOID` supplier casts
/// the factor with `(int)` (0.5 → 0, so it charges nothing), and `handleSingleBlock` applies it to
/// `builderRfPerQuarry` (300 × 0.5 = 150). The port adopts the config's stated meaning.
pub const VOID_RF_FACTOR: f64 = 0.5;
/// `BuilderConfiguration.builderMaxRF`: the FE the Builder can hold (the jar's
/// `GenericEnergyStorage(1_000_000, …)` capacity).
pub const ENERGY_CAPACITY: i32 = 1_000_000;
/// `BuilderConfiguration.builderRFPerTick`: the most one insert into the buffer moves
/// (`GenericEnergyStorage(…, 20_000)`, the jar's receive-per-tick limit).
pub const ENERGY_MAX_INSERT: i32 = 20_000;
/// `BuilderConfiguration.builderRfPerOperation`: the base cost of a shaped build cell (500).
pub const RF_PER_OPERATION: u32 = 500;
/// `BuilderConfiguration.builderRfPerQuarry`: the base cost of a broken cell (300).
pub const RF_PER_QUARRY: u32 = 300;
/// `BuilderConfiguration.builderRfPerSkipped`: the cost of a filtered-out cell (50). The port
/// carries no filter module, so no cell is skipped for a filter and this cost never fires here.
pub const RF_PER_SKIPPED: u32 = 50;

/// `BuilderConfiguration.builderRfPerLiquid`: the FE the jar charges per pumped or
/// placed liquid cell (`sipush 300` under "Base RF per block operation ... as a pump").
pub const RF_PER_LIQUID: u32 = 300;

/// One bucket in millibuckets. The jar's `checkFluidTank` accepts a fill only when
/// `fill(stack, SIMULATE) == 1000` and `findAndConsumeLiquid` drains exactly 1000 mB:
/// one source block in or out is a whole bucket, never a partial one.
pub const BUCKET_MB: u32 = 1000;

/// The capacity of the Builder's fluid tank. The jar keeps the pumped fluid in an
/// external tank above or below the Builder and the package has no tank block, so
/// the Builder declares its own 64-bucket tank (the millibucket rule is the jar's).
pub const FLUID_TANK_CAPACITY: u32 = 64_000;
/// `BuilderConfiguration.quarryInfusionSpeedFactor` (20): blocks per tick gained per infusion
/// point. The jar's *cost* uses `getInfusedFactor()` as `(3 - factor) / 3` instead.
pub const INFUSION_SPEED_FACTOR: i32 = 20;
/// The Builder's infusion factor. This package has no infuser, so it is the jar's un-infused
/// value and the cost multiplier is exactly 1.
pub const INFUSION_FACTOR: f32 = 0.0;
/// `commonQuarryBlock`'s own guard: a 3×3×3 box around the Builder is never quarried.
pub const BUILDER_CELLS: i32 = 1;
/// `BuilderTileEntity.DONT_REMOVE_ME`, a block tag upstream creates and ships no members
/// for: a block a pack marks with it is never removed (the port also never rolls its
/// drops — upstream's order did, see `notes.md`).
pub const DONT_REMOVE_ME: &str = "rftoolsbuilder:dontremoveme";

/// The three blocks `BlockState.isAir()` answers true for.
pub const AIR_BLOCKS: [&str; 3] = ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"];

/// Whether a block id is air (`BlockState.isAir`, which covers the three air blocks).
#[must_use]
pub fn is_air_block(block: &str) -> bool {
    AIR_BLOCKS.contains(&block)
}

/// `ShapeCardItem.getVoidedBlocks`'s material keys, in the jar's own order.
pub const VOID_MATERIALS: [&str; 7] = [
    "stone",
    "cobble",
    "dirt",
    "sand",
    "gravel",
    "netherrack",
    "endstone",
];

/// The material key a block belongs to by its own id: `getVoidedBlocks` adds exactly
/// `Blocks.STONE` for `stone`, `Blocks.COBBLESTONE` for `cobble`, `Blocks.DIRT` **and**
/// `Blocks.GRASS_BLOCK` for `dirt`, and one block for each other key. A block that is only
/// a tag member is not named here — that is what the card's tag matching is for.
#[must_use]
pub fn material_of(block: &str) -> Option<&'static str> {
    Some(match block {
        "minecraft:stone" => "stone",
        "minecraft:cobblestone" => "cobble",
        "minecraft:dirt" | "minecraft:grass_block" => "dirt",
        "minecraft:sand" => "sand",
        "minecraft:gravel" => "gravel",
        "minecraft:netherrack" => "netherrack",
        "minecraft:end_stone" => "endstone",
        _ => return None,
    })
}

/// The block tag a material expands to when the card's `tagMatching` is on
/// (`Tags.Blocks.STONES`, `COBBLESTONES`, `BlockTags.DIRT`, `SANDS`, `GRAVELS`,
/// `NETHERRACKS`, `END_STONES`).
#[must_use]
pub fn tag_of(material: &str) -> &'static str {
    match material {
        "stone" => "c:stones",
        "cobble" => "c:cobblestones",
        "dirt" => "minecraft:dirt",
        "sand" => "c:sands",
        "gravel" => "c:gravels",
        "netherrack" => "c:netherracks",
        _ => "c:end_stones",
    }
}

/// The shapes a card can name (`mcjty.rftoolsbuilder.shapes.Shape` minus the
/// three this slice does not carry: composition, scan, and the space card).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    /// `SHAPE_BOX`: the card's dimension, as a box.
    Box,
    /// `SHAPE_TOPDOME`: the sphere's upper half.
    TopDome,
    /// `SHAPE_BOTTOMDOME`: the sphere's lower half.
    BottomDome,
    /// `SHAPE_SPHERE`.
    Sphere,
    /// `SHAPE_CYLINDER`: the ellipse over the whole height.
    Cylinder,
    /// `SHAPE_CAPPEDCYLINDER`: the ellipse clipped to the card's height.
    CappedCylinder,
    /// `SHAPE_PRISM`: a box narrowing towards the top.
    Prism,
    /// `SHAPE_TORUS`.
    Torus,
    /// `SHAPE_HEART`.
    Heart,
    /// `SHAPE_CONE`.
    Cone,
}

/// Which way a composition child is turned (`ShapeRotation`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Rotation {
    /// `NONE`.
    #[default]
    None,
    /// `X`: y and z swap.
    X,
    /// `Y`: x and z swap.
    Y,
    /// `Z`: x and y swap.
    Z,
}

/// A card's turn and mirror (`ShapeModifier`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifier {
    /// Which way the shape is turned.
    pub rotation: Rotation,
    /// Whether the shape is mirrored top to bottom inside its box.
    pub flip_y: bool,
}

/// One quarry card type (`ShapeCardType.CARD_QUARRY*`): the three flags that decide what
/// `commonQuarryBlock` does to a cell. `silk` is upstream's handler choice
/// (`silkQuarryBlock` vs `quarryBlock`), `fortune` its `isFortune()` and `clearing` its
/// `isClearing()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Quarry {
    /// `silkQuarryBlock`: harvest with silk touch.
    pub silk: bool,
    /// `isFortune()`: harvest with fortune III.
    pub fortune: bool,
    /// `isClearing()`: leave air behind instead of the replacement block.
    pub clearing: bool,
    /// `ShapeCardType.CARD_VOID`: the `voidBlock` handler voids every breakable cell instead of
    /// rolling its drops. The jar has no `isVoid()` flag — the handler is a different enum
    /// constant — so this field is how the one card item this port carries names that constant.
    pub voids: bool,
}

/// One of the jar's `CARD_PUMP*` types. `clearing` is `isClearing()` on
/// `CARD_PUMP_CLEAR`; a pump removes the fluid source either way, so the flag is
/// carried for the readout and for parity with the quarry's clearing cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pump {
    /// `ShapeCardType.isClearing()` (false for `pump`, true for `pump_clear`).
    pub clearing: bool,
}

impl Pump {
    /// `CARD_PUMP` (`resourceSuffix "pump"`, handler `pumpBlock`).
    pub const PUMP: Pump = Pump { clearing: false };
    /// `CARD_PUMP_CLEAR` (`resourceSuffix "pump_clear"`, handler `pumpBlock`).
    pub const PUMP_CLEAR: Pump = Pump { clearing: true };

    /// The card type's resource suffix, the tail of the jar's item id.
    #[must_use]
    pub fn suffix(self) -> &'static str {
        if self.clearing { "pump_clear" } else { "pump" }
    }

    /// The jar's `hudLogEntry` for both pump cards (`"    Pump"`), without its
    /// four leading spaces as for every other card type.
    #[must_use]
    pub fn description(self) -> &'static str {
        "Pump"
    }
}

/// The card's type: upstream's `ShapeCardType`, which the jar reads from the **item id**
/// (`ShapeCardItem.type`). This port carries one card item, so the type rides the card's
/// own component (a documented deviation, `notes.md`); a card with none is upstream's
/// `shape_card_def`, `CARD_SHAPE`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CardKind {
    /// `CARD_SHAPE`: place a block into every cell of the shape.
    #[default]
    Shape,
    /// One of the six `CARD_QUARRY*` plus `CARD_VOID`: break the cells' blocks.
    Quarry(Quarry),
    /// `CARD_PUMP*` (`pump`, `pump_clear`): move a fluid source into the Builder's tank.
    Pump(Pump),
    /// `CARD_PUMP_LIQUID` (`liquid`): place a bucket of fluid from the tank.
    Liquid,
}

/// Which blocks a quarry card removes and collects nothing for (`getVoidedBlocks`).
///
/// Upstream keeps a `Set<String>` of the seven material keys; the port keeps the same keys
/// in a bitset, so a card stays `Copy` and the component still stores the jar's string list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Voiding(u8);

/// What the quarry rule needs to know about one cell (`commonQuarryBlock`'s inputs).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellFacts {
    /// The cell is inside the Builder's own 3×3×3 box.
    pub near_builder: bool,
    /// `BlockState.isAir()`.
    pub air: bool,
    /// `BlockState.getDestroySpeed`: below zero means the block cannot be broken.
    pub destroy_speed: f32,
    /// The block is the Builder's replacement block.
    pub is_replacement: bool,
    /// The block is on the card's void list.
    pub voided: bool,
    /// The card's filter accepts this block (`FilterModuleCache.test` on its clone stack).
    /// `true` for no filter, as upstream's empty filter slot 1 has no predicate.
    pub filter_passes: bool,
}

/// What the quarry does at one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuarryStep {
    /// Nothing: the block stays, and no material is spent on it.
    Skip(SkipReason),
    /// Remove the block after clearing or replacing it, and **collect nothing** (it is on
    /// the card's void list).
    Void,
    /// Roll the block's drops into the Builder's container, then clear or replace it.
    Harvest,
}

/// Why a quarry skips a cell (`commonQuarryBlock`'s early returns).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// The Builder's own 3×3×3 box.
    NearBuilder,
    /// The cell is air: the jar's `forquarry` filter never puts it in the map.
    Empty,
    /// `getDestroySpeed < 0`: bedrock and the like.
    Unbreakable,
    /// The block is already the replacement block, and the card is not clearing.
    Replacement,
    /// The card's filter rejected the block (`FilterModuleCache.test` was false): the cell is
    /// skipped, charged `builderRfPerSkipped`, and its drops are never rolled.
    Filtered,
}

/// One shape card's parameters: the subset of upstream's `ShapeCardData` that
/// decides where a shape is. `channel`, `scanId`, `children`, `ghostBlock` and the
/// corner-selection `mode` are not carried (notes.md); `kind`, `voiding` and
/// `tag_matching` are the quarry's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    /// The shape this card draws.
    pub shape: Shape,
    /// Solid (every cell) or hollow (the shape's shell).
    pub solid: bool,
    /// The box the shape is fitted to, as the card stores it.
    pub dimension: [i32; 3],
    /// Where the shape's centre sits relative to the Builder.
    pub offset: [i32; 3],
    /// The card's turn and mirror.
    pub modifier: Modifier,
    /// The card's type: a shape card builds, a quarry card breaks.
    pub kind: CardKind,
    /// The materials this card's quarry collects nothing for.
    pub voiding: Voiding,
    /// `ShapeCardData.tagMatching`: expand the void list through each material's block tag.
    pub tag_matching: bool,
}

/// Why a card cannot be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardError {
    /// `ShapeCardItem.MAXIMUM_COUNT`: the shape holds more cells than a card
    /// may name.
    TooLarge {
        /// The cells the clamped dimension holds.
        cells: u64,
        /// `MAX_CARD_CELLS`.
        limit: u64,
    },
}

/// Why the Builder stopped at a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalReason {
    /// The cell is empty and nothing the Builder can place is in reach.
    NoMaterial,
    /// The shape lies outside the world's build height.
    OutOfWorld,
    /// The cell is protected: outside the world border, or spawn-protected.
    Protected,
    /// A quarry's drops do not fit in the container above or below it.
    NoRoom,
    /// The cell's chunk is not loaded, so it cannot be read or written this tick.
    Unloaded,
    /// The Builder's FE buffer cannot pay for this cell (the jar's `Not enough power!`
    /// suspension). Nothing is broken or placed while it waits.
    NoPower,
    /// A liquid placement found no whole bucket in the tank ("Cannot find liquid!").
    NoLiquid,
}

/// The cell the Builder is waiting at, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// Where the Builder stopped.
    pub at: [i32; 3],
    /// What stopped it.
    pub reason: RefusalReason,
}

/// How far one scan has got.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Cells a block was placed into (a shape card) or a block was broken in (a quarry).
    pub placed: u64,
    /// Cells looked at that needed nothing (already filled, not in the shape, or a quarry
    /// cell it must not break).
    pub skipped: u64,
    /// Quarry cells whose block was removed without its drops being collected (the card's
    /// void list).
    pub voided: u64,
}

/// A Builder's scan: where it looks next, and the box it walks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scan {
    /// The card being built.
    pub card: Card,
    /// The Builder's own position.
    pub origin: [i32; 3],
    /// The low corner of the walk, from the (turned) dimension and the offset.
    pub min: [i32; 3],
    /// The high corner, exclusive.
    pub max: [i32; 3],
    /// The next cell the walk looks at; `None` before the first tick and after
    /// a completed pass.
    pub cursor: Option<[i32; 3]>,
}

/// What a status readout says about one Builder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// The Builder's position.
    pub at: [i32; 3],
    /// Its card, when one is installed and usable.
    pub card: Option<Card>,
    /// Why the card is unusable, when it is installed but refused.
    pub refused_card: Option<CardError>,
    /// How far the scan has got.
    pub progress: Progress,
    /// Where it stopped, when it did.
    pub refusal: Option<Refusal>,
    /// Whether the pass has finished.
    pub complete: bool,
    /// The cells the readout prints: the pass's cached count when the block entity holds one, so a
    /// repeated `/rftoolsbuilder status` costs nothing. `None` computes it within
    /// [`STATUS_CANDIDATES`] candidates.
    pub cells: Option<CellCount>,
    /// The Builder's replacement block (`BuilderConfiguration.quarryReplace`), as the block
    /// entity holds it. Only a non-clearing quarry card prints it.
    pub replace: String,
}

/// A shape's cell count as a readout prints it: the number found, and whether the shape holds
/// more (past [`MAX_PASS_CELLS`] kept cells, or past the candidates the readout may inspect).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellCount {
    /// The cells counted, at least.
    pub cells: u64,
    /// Whether the shape holds more than `cells`.
    pub more: bool,
}

impl Default for Card {
    /// `ShapeCardData.DEFAULT`: a solid 5×5×5 box around the Builder.
    fn default() -> Self {
        Self {
            shape: Shape::Box,
            solid: true,
            dimension: DEFAULT_DIMENSION,
            offset: [0, 0, 0],
            modifier: Modifier::default(),
            kind: CardKind::Shape,
            voiding: Voiding::empty(),
            tag_matching: false,
        }
    }
}

impl Quarry {
    /// The six `CARD_QUARRY*` constants, in the jar's enum order, followed by `CARD_VOID`
    /// (the seventh card type this port carries).
    pub const ALL: &'static [Quarry] = &[
        Quarry {
            silk: false,
            fortune: false,
            clearing: false,
            voids: false,
        },
        Quarry {
            silk: true,
            fortune: false,
            clearing: false,
            voids: false,
        },
        Quarry {
            silk: false,
            fortune: true,
            clearing: false,
            voids: false,
        },
        Quarry {
            silk: false,
            fortune: false,
            clearing: true,
            voids: false,
        },
        Quarry {
            silk: true,
            fortune: false,
            clearing: true,
            voids: false,
        },
        Quarry {
            silk: false,
            fortune: true,
            clearing: true,
            voids: false,
        },
        Self::VOID,
    ];

    /// `ShapeCardType.CARD_VOID`: resourceSuffix `void`, quarry=false, clearing=false,
    /// fortune=false, handler `BuilderTileEntity::voidBlock`, HUD `"    Void mode"` and
    /// `voidShapeCardFactor = 0.5` ([`VOID_RF_FACTOR`]). Voids every breakable cell instead of
    /// collecting its drops; the port uses the already-tested [`QuarryStep::Void`] path, which
    /// leaves the Builder's replacement block behind (a deliberate difference from the jar's
    /// `voidBlock`, which writes air — recorded in `notes.md`).
    pub const VOID: Quarry = Quarry {
        silk: false,
        fortune: false,
        clearing: false,
        voids: true,
    };

    /// The card type's resource suffix (`ShapeCardType.getResourceSuffix`), which is the
    /// tail of the jar's item id (`rftoolsbuilder:shape_card_quarry_silk`).
    #[must_use]
    pub fn suffix(self) -> &'static str {
        if self.voids {
            return "void";
        }
        match (self.clearing, self.silk, self.fortune) {
            (false, false, false) => "quarry",
            (false, true, _) => "quarry_silk",
            (false, false, true) => "quarry_fortune",
            (true, false, false) => "quarry_clear",
            (true, true, _) => "quarry_clear_silk",
            (true, false, true) => "quarry_clear_fortune",
        }
    }

    /// The card type a suffix names; the jar's item id (`shape_card_quarry_silk`) reads
    /// too. `None` for anything that is not one of the seven.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let wanted = bare_name(name);
        Self::ALL
            .iter()
            .copied()
            .find(|quarry| quarry.suffix() == wanted)
    }

    /// The header the HUD log prints (`hudLogEntry`), which the readout reuses. The jar's
    /// `CARD_VOID` entry is `"    Void mode"`; like the other entries, the four leading spaces
    /// are dropped for the readout.
    #[must_use]
    pub fn description(self) -> &'static str {
        if self.voids {
            "Void mode"
        } else if self.fortune {
            "Fortune quarry"
        } else if self.silk {
            "Silktouch quarry"
        } else {
            "Normal quarry"
        }
    }
}

impl CardKind {
    /// The type's suffix, as the card's component stores it.
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            CardKind::Shape => "shape",
            CardKind::Quarry(quarry) => quarry.suffix(),
            CardKind::Pump(pump) => pump.suffix(),
            CardKind::Liquid => "liquid",
        }
    }

    /// The jar's `hudLogEntry` for this card type, without its four leading spaces.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            CardKind::Shape => "",
            CardKind::Quarry(quarry) => quarry.description(),
            CardKind::Pump(pump) => pump.description(),
            CardKind::Liquid => "Place liquids",
        }
    }

    /// The card type a name names: the quarry suffixes plus `void`, the two pump
    /// suffixes, `liquid`, or the plain shape card (`shape`, `def`, `shape_card_def`).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        if matches!(bare_name(name), "shape" | "def") {
            return Some(CardKind::Shape);
        }
        if let Some(quarry) = Quarry::parse(name) {
            return Some(CardKind::Quarry(quarry));
        }
        match bare_name(name) {
            "pump" => Some(CardKind::Pump(Pump::PUMP)),
            "pump_clear" => Some(CardKind::Pump(Pump::PUMP_CLEAR)),
            "liquid" => Some(CardKind::Liquid),
            _ => None,
        }
    }
}

/// A card type name without the namespace and upstream's `shape_card_` item prefix:
/// `rftoolsbuilder:shape_card_quarry_silk` and `quarry_silk` both read as `quarry_silk`.
fn bare_name(name: &str) -> &str {
    let bare = name
        .trim()
        .trim_start_matches('/')
        .rsplit(':')
        .next()
        .unwrap_or_default();
    bare.strip_prefix("shape_card_").unwrap_or(bare)
}

impl Voiding {
    /// The list with nothing on it.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// The list a card's `voiding` string list names, ignoring keys it does not know (a jar
    /// card may carry any string).
    #[must_use]
    pub fn from_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names
            .into_iter()
            .fold(Self::empty(), |void, name| void.with(name.as_ref()))
    }

    /// The bit a material key names.
    fn bit(material: &str) -> Option<u8> {
        VOID_MATERIALS
            .iter()
            .position(|known| known.eq_ignore_ascii_case(material.trim()))
            .and_then(|index| u8::try_from(index).ok())
            .map(|index| 1 << index)
    }

    /// Whether the card voided this material.
    #[must_use]
    pub fn contains(self, material: &str) -> bool {
        Self::bit(material).is_some_and(|bit| self.0 & bit != 0)
    }

    /// This list plus `material`; an unknown key changes nothing.
    #[must_use]
    pub fn with(self, material: &str) -> Self {
        Self(self.0 | Self::bit(material).unwrap_or(0))
    }

    /// This list without `material`.
    #[must_use]
    pub fn without(self, material: &str) -> Self {
        Self(self.0 & !Self::bit(material).unwrap_or(0))
    }

    /// Whether the card voids nothing at all.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The materials, in the jar's own order.
    pub fn iter(self) -> impl Iterator<Item = &'static str> {
        VOID_MATERIALS
            .iter()
            .enumerate()
            .filter(move |(index, _)| {
                u8::try_from(*index).is_ok_and(|index| self.0 & (1 << index) != 0)
            })
            .map(|(_, material)| *material)
    }

    /// The materials, in the jar's own order, as the readout and the component write them.
    #[must_use]
    pub fn names(self) -> String {
        self.iter().collect::<Vec<_>>().join(", ")
    }
}

/// What a quarry card does at one cell: the jar's `commonQuarryBlock` decision tree, in its
/// own order. It says *what* to do, not how — rolling the drops, filling the container and
/// writing the block are the glue's.
#[must_use]
pub fn quarry_step(quarry: Quarry, facts: CellFacts) -> QuarryStep {
    if facts.near_builder {
        return QuarryStep::Skip(SkipReason::NearBuilder);
    }
    if facts.air {
        return QuarryStep::Skip(SkipReason::Empty);
    }
    if facts.destroy_speed < 0.0 {
        return QuarryStep::Skip(SkipReason::Unbreakable);
    }
    if !quarry.clearing && facts.is_replacement {
        return QuarryStep::Skip(SkipReason::Replacement);
    }
    // The filter module is consulted in **both** paths (`commonQuarryBlock` bytecode 298..351,
    // after `allowedToBreak`, before the void list and the drop roll): a rejected block is
    // skipped without being broken or handing out drops, and the glue charges 50 FE for it.
    if !facts.filter_passes {
        return QuarryStep::Skip(SkipReason::Filtered);
    }
    // `CARD_VOID`'s `voidBlock` — every breakable cell goes down the same no-drops path the
    // quarry's void list uses; no per-material list is consulted for this type.
    if quarry.voids || facts.voided {
        return QuarryStep::Void;
    }
    QuarryStep::Harvest
}

impl Shape {
    /// Every shape this slice carries, in the jar's enum order.
    pub const ALL: &'static [Shape] = &[
        Shape::Box,
        Shape::TopDome,
        Shape::BottomDome,
        Shape::Sphere,
        Shape::Cylinder,
        Shape::CappedCylinder,
        Shape::Prism,
        Shape::Torus,
        Shape::Heart,
        Shape::Cone,
    ];

    /// The jar's enum name, as `Shape.getSerializedName` writes it.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Shape::Box => "SHAPE_BOX",
            Shape::TopDome => "SHAPE_TOPDOME",
            Shape::BottomDome => "SHAPE_BOTTOMDOME",
            Shape::Sphere => "SHAPE_SPHERE",
            Shape::Cylinder => "SHAPE_CYLINDER",
            Shape::CappedCylinder => "SHAPE_CAPPEDCYLINDER",
            Shape::Prism => "SHAPE_PRISM",
            Shape::Torus => "SHAPE_TORUS",
            Shape::Heart => "SHAPE_HEART",
            Shape::Cone => "SHAPE_CONE",
        }
    }

    /// The name the card's tooltip shows (`Shape.getDescription`).
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Shape::Box => "Box",
            Shape::TopDome => "Top Dome",
            Shape::BottomDome => "Bottom Dome",
            Shape::Sphere => "Sphere",
            Shape::Cylinder => "Cylinder",
            Shape::CappedCylinder => "Capped Cylinder",
            Shape::Prism => "Prism",
            Shape::Torus => "Torus",
            Shape::Heart => "Heart",
            Shape::Cone => "Cone",
        }
    }

    /// The shape a name, enum id or description names, ignoring case, spaces
    /// and underscores (`box`, `SHAPE_BOX`, `Capped Cylinder`, `cappedcylinder`).
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        let wanted: String = name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        Self::ALL.iter().copied().find(|shape| {
            let id: String = shape
                .id()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect();
            let description: String = shape
                .description()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .map(|c| c.to_ascii_lowercase())
                .collect();
            wanted == id || wanted == description
        })
    }
}

impl Rotation {
    /// The jar's `ShapeRotation.getCode`.
    #[must_use]
    pub fn id(self) -> &'static str {
        match self {
            Rotation::None => "0",
            Rotation::X => "X",
            Rotation::Y => "Y",
            Rotation::Z => "Z",
        }
    }

    /// The rotation a code names (`ShapeRotation.getByName`); `0` is none.
    #[must_use]
    pub fn parse(code: &str) -> Option<Self> {
        match code.trim().to_ascii_uppercase().as_str() {
            "0" | "NONE" => Some(Rotation::None),
            "X" => Some(Rotation::X),
            "Y" => Some(Rotation::Y),
            "Z" => Some(Rotation::Z),
            _ => None,
        }
    }

    /// The box a shape of this dimension is turned into
    /// (`ShapeRotation.transformDimension`).
    #[must_use]
    pub fn transform_dimension(self, [x, y, z]: [i32; 3]) -> [i32; 3] {
        match self {
            Rotation::None => [x, y, z],
            Rotation::X => [x, z, y],
            Rotation::Y => [z, y, x],
            Rotation::Z => [y, x, z],
        }
    }
}

impl Card {
    /// This card with the jar's clamps applied: the dimension into
    /// `0..=MAX_DIMENSION`, the offset into `-MAX_OFFSET..=MAX_OFFSET`
    /// (`getClampedDimension` / `getClampedOffset`).
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            dimension: self.dimension.map(|d| d.clamp(0, MAX_DIMENSION)),
            offset: self.offset.map(|o| o.clamp(-MAX_OFFSET, MAX_OFFSET)),
            ..self
        }
    }

    /// The cells the card's *requested* dimension names, before clamping: the
    /// product, saturating.
    #[must_use]
    pub fn requested_cells(self) -> u64 {
        cells_of(self.dimension.map(|d| d.max(0)))
    }

    /// The cells the clamped bounding box holds: the product of the clamped
    /// dimension, saturating.
    #[must_use]
    pub fn box_cells(self) -> u64 {
        cells_of(self.clamped().dimension)
    }

    /// This card ready to build, or `CardError::TooLarge` when its clamped box
    /// holds more than [`MAX_CARD_CELLS`] (upstream's `#Blocks: ++` refusal).
    ///
    /// # Errors
    ///
    /// `CardError::TooLarge` carries the clamped box's cell count and the limit.
    pub fn validate(self) -> Result<Self, CardError> {
        let card = self.clamped();
        let cells = card.box_cells();
        if cells > MAX_CARD_CELLS {
            Err(CardError::TooLarge {
                cells,
                limit: MAX_CARD_CELLS,
            })
        } else {
            Ok(card)
        }
    }

    /// `Solid` or `Hollow`, as the card's tooltip says it.
    #[must_use]
    pub fn solid_word(self) -> &'static str {
        if self.solid { "Solid" } else { "Hollow" }
    }

    /// The shape's low corner for a Builder at `origin`
    /// (`ShapeCardItem.getMinCorner`, on the turned dimension).
    #[must_use]
    pub fn min_corner(self, origin: [i32; 3]) -> [i32; 3] {
        let dim = self.modifier.rotation.transform_dimension(self.dimension);
        let mut out = [0; 3];
        for axis in 0..3 {
            out[axis] = origin[axis] - dim[axis] / 2 + self.offset[axis];
        }
        out
    }

    /// The high corner, exclusive (`getMaxCorner`).
    #[must_use]
    pub fn max_corner(self, origin: [i32; 3]) -> [i32; 3] {
        let dim = self.modifier.rotation.transform_dimension(self.dimension);
        let min = self.min_corner(origin);
        [min[0] + dim[0], min[1] + dim[1], min[2] + dim[2]]
    }

    /// Whether `pos` is a cell of this shape for a Builder at `origin`: inside
    /// the walked box **and** inside the shape (the jar's `composeFormula`
    /// iterates the box and asks the formula).
    #[must_use]
    pub fn contains(self, origin: [i32; 3], pos: [i32; 3]) -> bool {
        let min = self.min_corner(origin);
        let max = self.max_corner(origin);
        if (0..3).any(|axis| pos[axis] < min[axis] || pos[axis] >= max[axis]) {
            return false;
        }
        if self.modifier == Modifier::default() {
            return self.inside(origin, pos);
        }
        // The jar's composition transform (`FormulaComposition.isInside`): turn
        // the relative coordinate about the card's offset, mirror it inside the
        // walked box, then ask the shape's own formula.
        let rel = [pos[0] - origin[0], pos[1] - origin[1], pos[2] - origin[2]];
        let [ox, oy, oz] = self.offset;
        let (mut tx, mut ty, mut tz) = (rel[0], rel[1], rel[2]);
        match self.modifier.rotation {
            Rotation::None => {}
            Rotation::X => {
                ty = (rel[2] - oz) + oy;
                tz = (rel[1] - oy) + oz;
            }
            Rotation::Y => {
                tx = (rel[2] - oz) + ox;
                tz = (rel[0] - ox) + oz;
            }
            Rotation::Z => {
                tx = (rel[1] - oy) + ox;
                ty = (rel[0] - ox) + oy;
            }
        }
        if self.modifier.flip_y {
            let p1 = min[1] - origin[1];
            let p2 = max[1] - origin[1];
            ty = p1 + p2 - 1 - ty;
        }
        self.inside(origin, [origin[0] + tx, origin[1] + ty, origin[2] + tz])
    }

    /// The shape's own test, with the box left out: the jar's formula, which
    /// for the cylinder ignores the height (so a hollow one-block-high
    /// cylinder has no shell — upstream's own behaviour, kept).
    fn inside(self, origin: [i32; 3], pos: [i32; 3]) -> bool {
        if self.solid {
            return inside_shape(self.shape, self.dimension, self.offset, origin, pos);
        }
        // Hollow: the jar's `correctFormula(false)` replaces the test with the
        // border test, which asks the *solid* test of the six neighbours.
        const NEIGHBOURS: [[i32; 3]; 6] = [
            [-1, 0, 0],
            [1, 0, 0],
            [0, -1, 0],
            [0, 1, 0],
            [0, 0, -1],
            [0, 0, 1],
        ];
        inside_shape(self.shape, self.dimension, self.offset, origin, pos)
            && NEIGHBOURS.iter().any(|step| {
                !inside_shape(
                    self.shape,
                    self.dimension,
                    self.offset,
                    origin,
                    [pos[0] + step[0], pos[1] + step[1], pos[2] + step[2]],
                )
            })
    }

    /// The cells of this shape for a Builder at `origin`, in the jar's
    /// `composeFormula` order (x slowest, then z, then y), stopping once `cap`
    /// cells have been kept. `truncated` says the shape holds more.
    #[must_use]
    pub fn cells(self, origin: [i32; 3], cap: u64) -> (Vec<[i32; 3]>, bool) {
        self.cells_within(origin, cap, u64::MAX)
    }

    /// [`Card::cells`] with a bound on how many candidates it may test, so a readout that only
    /// needs a count never walks a shape whose box holds tens of millions of cells.
    #[must_use]
    pub fn cells_within(self, origin: [i32; 3], cap: u64, work: u64) -> (Vec<[i32; 3]>, bool) {
        let min = self.min_corner(origin);
        let dim = self.modifier.rotation.transform_dimension(self.dimension);
        let mut out = Vec::new();
        let mut truncated = false;
        let mut looked = 0u64;
        'outer: for ox in 0..dim[0] {
            for oz in 0..dim[2] {
                for oy in 0..dim[1] {
                    if looked >= work {
                        truncated = true;
                        break 'outer;
                    }
                    looked += 1;
                    let pos = [min[0] + ox, min[1] + oy, min[2] + oz];
                    if !(self.contains(origin, pos)) {
                        continue;
                    }
                    if out.len() as u64 == cap {
                        truncated = true;
                        break 'outer;
                    }
                    out.push(pos);
                }
            }
        }
        (out, truncated)
    }

    /// How many cells this shape holds, counted up to `cap` (so a box answers
    /// in constant time and the others stop early). `truncated` says the shape
    /// holds more than `cap`.
    #[must_use]
    pub fn count(self, cap: u64) -> (u64, bool) {
        self.count_within(cap, u64::MAX)
    }

    /// [`Card::count`] with a bound on the candidates it may inspect, so a readout can answer in
    /// bounded time: it stops once `work` cells have been looked at and reports that there are
    /// more, exactly as it does past `cap` kept cells. A box is answered from its dimensions and
    /// inspects nothing.
    #[must_use]
    pub fn count_within(self, cap: u64, work: u64) -> (u64, bool) {
        if self.modifier == Modifier::default() && self.shape == Shape::Box {
            let volume = self.box_cells();
            if self.solid {
                return (volume.min(cap), volume > cap);
            }
            // Hollow box: the box minus the shell's interior.
            let interior = self
                .clamped()
                .dimension
                .map(|d| u64::from(u32::try_from(d - 2).unwrap_or(0)))
                .into_iter()
                .product::<u64>();
            let shell = volume - interior;
            return (shell.min(cap), shell > cap);
        }
        let (cells, truncated) = self.cells_within([0, 0, 0], cap, work);
        (cells.len() as u64, truncated)
    }
}

/// The number of cells a box of `dim` holds, saturating.
fn cells_of(dim: [i32; 3]) -> u64 {
    dim.into_iter()
        .map(|d| u64::from(u32::try_from(d).unwrap_or(0)))
        .product()
}

/// The jar's shape formulas (`Formulas.*.isInside`), in absolute coordinates:
/// `origin` is the Builder, `dim` the card's dimension and `offset` its offset.
fn inside_shape(
    shape: Shape,
    dim: [i32; 3],
    offset: [i32; 3],
    origin: [i32; 3],
    pos: [i32; 3],
) -> bool {
    let frame = Frame::new(dim, origin, offset);
    match shape {
        Shape::Box => {
            let min = frame.min();
            (0..3).all(|axis| pos[axis] >= min[axis] && pos[axis] < min[axis] + dim[axis])
        }
        Shape::Sphere => frame.sphere(pos, None),
        Shape::TopDome => frame.sphere(pos, Some(true)),
        Shape::BottomDome => frame.sphere(pos, Some(false)),
        Shape::Cylinder => frame.cylinder(pos, false),
        Shape::CappedCylinder => frame.cylinder(pos, true),
        Shape::Cone => frame.cone(pos),
        Shape::Prism => frame.prism(pos),
        Shape::Torus => frame.torus(pos),
        Shape::Heart => frame.heart(pos),
    }
}

/// One shape's box, centre and Builder, as the jar's formulas set them up
/// (`Formulas.*.setup`): `thisCoord` is the Builder's own position.
struct Frame {
    dim: [i32; 3],
    origin: [i32; 3],
    offset: [i32; 3],
    min: [i32; 3],
    centre: [f32; 3],
}

impl Frame {
    fn new(dim: [i32; 3], origin: [i32; 3], offset: [i32; 3]) -> Self {
        let mut min = [0; 3];
        let mut centre = [0.0; 3];
        for axis in 0..3 {
            // Java's `/` truncates towards zero, as Rust's does.
            min[axis] = origin[axis] - dim[axis] / 2 + offset[axis];
            // `xCoord + offset + ((dim % 2 != 0) ? 0.0f : -.5f)`
            centre[axis] = origin[axis] as f32 + offset[axis] as f32 + parity(dim[axis]);
        }
        Self {
            dim,
            origin,
            offset,
            min,
            centre,
        }
    }

    /// The box's low corner.
    fn min(&self) -> [i32; 3] {
        self.min
    }

    /// The position as the formulas' floats.
    fn local(&self, pos: [i32; 3]) -> [f32; 3] {
        [pos[0] as f32, pos[1] as f32, pos[2] as f32]
    }

    /// `((d + factor)²) / 4` per axis.
    fn ellipse(&self, factor: f32) -> [f32; 3] {
        self.dim
            .map(|d| (d as f32 + factor) * (d as f32 + factor) / 4.0)
    }

    /// `(int) ((dx + dy + dz + factor * 3) / 3)`: the sphere's `davg`.
    fn average3(&self, factor: f32) -> i32 {
        let total = self.dim[0] as f32 + self.dim[1] as f32 + self.dim[2] as f32 + factor * 3.0;
        (total / 3.0) as i32
    }

    /// `(int) ((dx + dz + factor * 2) / 2)`: the cylinder's and cone's `davg`.
    fn average2(&self, factor: f32) -> i32 {
        let total = self.dim[0] as f32 + self.dim[2] as f32 + factor * 2.0;
        (total / 2.0) as i32
    }

    /// `Formulas.FormulaSphere` and the two domes: the ellipse test over all
    /// three axes, `half` selecting the upper (`Some(true)`) or lower part.
    fn sphere(&self, pos: [i32; 3], half: Option<bool>) -> bool {
        let y = pos[1] as f32;
        if let Some(top) = half {
            if top && y < self.centre[1] {
                return false;
            }
            if !top && y > self.centre[1] {
                return false;
            }
        }
        let [x, y, z] = self.local(pos);
        let [dx2, dy2, dz2] = self.ellipse(1.8);
        let davg = self.average3(1.8);
        let squared = (x - self.centre[0]) * (x - self.centre[0]) / dx2
            + (y - self.centre[1]) * (y - self.centre[1]) / dy2
            + (z - self.centre[2]) * (z - self.centre[2]) / dz2;
        let distance = f64::from(squared).sqrt();
        ((distance * f64::from(davg / 2 + 1)) as i32) < davg / 2
    }

    /// `Formulas.FormulaCylinder` and `FormulaCappedCylinder`: the ellipse over
    /// x and z only, the capped one clipped to the card's height.
    fn cylinder(&self, pos: [i32; 3], capped: bool) -> bool {
        if capped && !(self.min[1]..self.min[1] + self.dim[1]).contains(&pos[1]) {
            return false;
        }
        let [x, _, z] = self.local(pos);
        let davg = self.average2(1.7);
        let [dx2, _, dz2] = self.ellipse(1.7);
        let squared = (x - self.centre[0]) * (x - self.centre[0]) / dx2
            + (z - self.centre[2]) * (z - self.centre[2]) / dz2;
        let distance = f64::from(squared).sqrt();
        ((distance * f64::from(davg / 2 + 1)) as i32) < davg / 2
    }

    /// `Formulas.FormulaCone`: the ellipse narrowed towards the top.
    fn cone(&self, pos: [i32; 3]) -> bool {
        let [x, y, z] = self.local(pos);
        let davg = self.average2(1.7);
        let [dx2, _, dz2] = self.ellipse(1.7);
        let squared = (x - self.centre[0]) * (x - self.centre[0]) / dx2
            + (z - self.centre[2]) * (z - self.centre[2]) / dz2;
        let distance = f64::from(squared).sqrt();
        let top = self.origin[1] + self.offset[1] + self.dim[1] / 2;
        let height = self.dim[1] as f32 + 0.5;
        let limit = (davg / 2 - 1) as f32 * (top as f32 - y) / height;
        // Java truncates the reach to an int before comparing it with the
        // float limit.
        let reach = (distance * f64::from(davg / 2 + 1)) as i32;
        reach as f32 <= limit
    }

    /// `Formulas.FormulaPrism`: a box narrowing towards the top.
    fn prism(&self, pos: [i32; 3]) -> bool {
        let min = self.min;
        let slice = pos[1] - min[1];
        if !(0..self.dim[1]).contains(&slice) {
            return false;
        }
        pos[0] >= min[0] + slice
            && pos[0] < min[0] + self.dim[0] - slice
            && pos[2] >= min[2] + slice
            && pos[2] < min[2] + self.dim[2] - slice
    }

    /// `Formulas.FormulaTorus`.
    fn torus(&self, pos: [i32; 3]) -> bool {
        let [x, y, z] = self.local(pos);
        let small = (self.dim[1] - 2) as f32 / 2.0;
        let big = (self.dim[0] - 2) as f32 / 2.0 - small;
        let squared = (x - self.centre[0]) * (x - self.centre[0])
            + (z - self.centre[2]) * (z - self.centre[2]);
        let ring = f64::from(big) - f64::from(squared).sqrt();
        let f = ring * ring + f64::from(y - self.centre[1]) * f64::from(y - self.centre[1])
            - f64::from(small) * f64::from(small);
        f < 0.0
    }

    /// `Formulas.FormulaHeart`, whose three constants push the surface
    /// off-centre (so it mirrors on no axis).
    fn heart(&self, pos: [i32; 3]) -> bool {
        let [x, y, z] = self.local(pos);
        let xx = f64::from(x - self.centre[0]) * 2.6 / f64::from(self.dim[0]) + 0.1;
        let zz = f64::from(y - self.centre[1]) * 2.4 / f64::from(self.dim[1]) + 0.2;
        let yy = f64::from(z - self.centre[2]) * 1.6 / f64::from(self.dim[2]) + 0.1;
        let f1 = (xx * xx + 2.25 * yy * yy + zz * zz - 1.0).powi(3);
        let f2 = xx * xx * zz * zz * zz;
        let f3 = (9.0 / 80.0) * yy * yy * zz * zz * zz;
        f1 - f2 - f3 < 0.0
    }
}

/// The jar's `(dim % 2 != 0) ? 0.0f : -.5f`.
fn parity(dim: i32) -> f32 {
    if dim % 2 == 0 { -0.5 } else { 0.0 }
}

/// The one-cell step of the Builder's walk: x fastest, then z, then y from the
/// top down, over the walked box, so every cell of the box is looked at exactly
/// once per pass.
///
/// `None` means the pass completed and the caller restarts at the top.
///
/// **Upstream defect fixed.** The jar's `nextLocationQuarry`
/// (`BuilderTileEntity.java:2198-2228`) steps `x` by one but snaps `x` and `z`
/// **down to the enclosing 16-block chunk** whenever it advances `z` or drops a
/// layer (`(x >> 4) << 4`), the remnant of its quarry chunk-loading. A box
/// whose low corner is in an earlier chunk than its high corner therefore has
/// whole columns that no pass ever looks at: for the default 5×5×5 box at a
/// Builder on a chunk boundary (`min.x = -2, max.x = 3`) the jar's walk reaches
/// **50 of its 125 cells**, and every later pass repeats the same miss, so the
/// other 75 are never built. This walk keeps the jar's top-down, x-fastest
/// order and reaches all 125 (see `the_walk_looks_at_every_cell_of_a_straddling_box`).
#[must_use]
pub fn next_cursor(cursor: [i32; 3], min: [i32; 3], max: [i32; 3]) -> Option<[i32; 3]> {
    let [x, y, z] = cursor;
    if x + 1 < max[0] {
        return Some([x + 1, y, z]);
    }
    if z + 1 < max[2] {
        return Some([min[0], y, z + 1]);
    }
    if y > min[1] {
        return Some([min[0], y - 1, min[2]]);
    }
    None
}

impl Scan {
    /// A scan of `card` for a Builder at `origin`, starting where the jar's
    /// `restartScan` starts a shaped build — the top of the low corner — on the
    /// topmost cell of the box (the jar's cursor sits one block above it, in
    /// the empty layer, and spends its first layer looking at nothing).
    #[must_use]
    pub fn start(origin: [i32; 3], card: Card) -> Self {
        let min = card.min_corner(origin);
        let max = card.max_corner(origin);
        Self {
            card,
            origin,
            min,
            max,
            cursor: Some([min[0], max[1] - 1, min[2]]),
        }
    }

    /// The scan resumed from a saved cursor (or not yet started). A cursor
    /// outside the walked box (a card that changed under a running pass) is
    /// dropped and the pass starts again at the top.
    #[must_use]
    pub fn resume(origin: [i32; 3], card: Card, cursor: Option<[i32; 3]>) -> Self {
        let mut scan = Self::start(origin, card);
        scan.cursor = match cursor {
            None => None,
            Some(cursor)
                if (0..3).all(|axis| {
                    cursor[axis] >= scan.min[axis] && cursor[axis] < scan.max[axis]
                }) =>
            {
                Some(cursor)
            }
            // A card that changed under a running pass: start the pass again.
            Some(_) => scan.cursor,
        };
        scan
    }

    /// Step to the next cell the walk looks at; `false` when the pass finished
    /// and the scan restarted (its cursor is then back at the start).
    pub fn advance(&mut self) -> bool {
        match self
            .cursor
            .and_then(|cursor| next_cursor(cursor, self.min, self.max))
        {
            Some(next) => {
                self.cursor = Some(next);
                true
            }
            None => {
                self.cursor = Some([self.min[0], self.max[1] - 1, self.min[2]]);
                false
            }
        }
    }

    /// Clip the walk's height to the world's build height, as the jar's
    /// `calculateBoxShaped` clips its box: `false` when the shape lies wholly
    /// outside it and there is nothing to build.
    pub fn clamp_y(&mut self, min_y: i32, max_y: i32) -> bool {
        self.min[1] = self.min[1].max(min_y);
        self.max[1] = self.max[1].min(max_y);
        if self.min[1] >= self.max[1] {
            return false;
        }
        if self
            .cursor
            .is_some_and(|cursor| cursor[1] < self.min[1] || cursor[1] >= self.max[1])
        {
            self.cursor = Some([self.min[0], self.max[1] - 1, self.min[2]]);
        }
        true
    }
}

/// The cells of `cells` that lie inside the world's build height, in order.
#[must_use]
pub fn clip_to_height(cells: &[[i32; 3]], min_y: i32, max_y: i32) -> Vec<[i32; 3]> {
    cells
        .iter()
        .copied()
        .filter(|pos| pos[1] >= min_y && pos[1] < max_y)
        .collect()
}

/// Whether `pos` lies inside the world's build height.
#[must_use]
pub fn in_height(pos: [i32; 3], min_y: i32, max_y: i32) -> bool {
    pos[1] >= min_y && pos[1] < max_y
}

impl RefusalReason {
    /// The words the readout and the action bar use.
    #[must_use]
    pub fn text(self) -> &'static str {
        match self {
            RefusalReason::NoMaterial => "no block to place (put one above or below the builder)",
            RefusalReason::OutOfWorld => "outside the world's build height",
            RefusalReason::Protected => "protected (outside the world border)",
            RefusalReason::NoRoom => {
                "no room for the drops (put a container above or below the builder)"
            }
            RefusalReason::Unloaded => "the chunk is not loaded",
            RefusalReason::NoPower => "not enough power",
            RefusalReason::NoLiquid => "cannot find liquid (put a tank above or below the builder)",
        }
    }
}

impl CardError {
    /// The cells the refused card's clamped box holds.
    #[must_use]
    pub fn cells(&self) -> u64 {
        match self {
            CardError::TooLarge { cells, .. } => *cells,
        }
    }

    /// The limit it passed.
    #[must_use]
    pub fn limit(&self) -> u64 {
        match self {
            CardError::TooLarge { limit, .. } => *limit,
        }
    }
}

impl std::fmt::Display for CardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CardError::TooLarge { cells, limit } => {
                write!(f, "shape of {cells} cells past the {limit}-cell card limit")
            }
        }
    }
}

/// `BuilderHeader` + the card + the progress, as `/rftoolsbuilder status` says
/// it. Byte-stable: the tests pin every branch.
#[must_use]
pub fn status_line(status: &Status) -> String {
    let [x, y, z] = status.at;
    let Some(card) = status.card else {
        return match &status.refused_card {
            Some(error) => format!(
                "Builder at {x} {y} {z}: card refused, shape of {} cells past the {}-cell card limit",
                error.cells(),
                error.limit()
            ),
            None => format!("Builder at {x} {y} {z}: no shape card"),
        };
    };
    let [dx, dy, dz] = card.dimension;
    let [ox, oy, oz] = card.offset;
    let count = status.cells.unwrap_or_else(|| {
        let (cells, more) = card.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES);
        CellCount { cells, more }
    });
    let (cells, truncated) = (count.cells, count.more);
    let cap = if truncated {
        format!(" (first {cells} cells)")
    } else {
        String::new()
    };
    // What the card is: a shape card builds, a quarry card breaks. The words are the jar's
    // own `hudLogEntry` plus its `(clearing)` line.
    let kind = match card.kind {
        CardKind::Shape => String::new(),
        CardKind::Quarry(quarry) => format!(
            ", quarry ({}{})",
            quarry.description(),
            if quarry.clearing { ", clearing" } else { "" }
        ),
        CardKind::Pump(pump) => format!(
            ", pump ({}{})",
            pump.description(),
            if pump.clearing { ", clearing" } else { "" }
        ),
        CardKind::Liquid => ", liquid (Place liquids)".to_owned(),
    };
    let replace = match card.kind {
        CardKind::Quarry(quarry) if !quarry.clearing => {
            format!(", replacing with {}", status.replace)
        }
        _ => String::new(),
    };
    let voids = if card.voiding.is_empty() {
        String::new()
    } else {
        format!(
            ", voiding {}{}",
            card.voiding.names(),
            if card.tag_matching { " (tags too)" } else { "" }
        )
    };
    let head = format!(
        "Builder at {x} {y} {z}: {} ({}) {dx}x{dy}x{dz} offset {ox} {oy} {oz}, {cells} cells{cap}{kind}{replace}{voids}",
        card.shape.description(),
        card.solid_word()
    );
    let action = match card.kind {
        CardKind::Quarry(_) => "quarried",
        CardKind::Pump(_) => "pumped",
        CardKind::Shape | CardKind::Liquid => "placed",
    };
    let voided = if status.progress.voided == 0 {
        String::new()
    } else {
        format!(", {} voided", status.progress.voided)
    };
    if let Some(refusal) = status.refusal {
        let [rx, ry, rz] = refusal.at;
        return format!(
            "{head}, {} {action}{voided}, refused at {rx} {ry} {rz}: {}",
            status.progress.placed,
            refusal.reason.text()
        );
    }
    if status.complete {
        if status.progress.skipped == 0 {
            return format!(
                "{head}, {} {action}{voided}, scan complete",
                status.progress.placed
            );
        }
        // A filtered-out (or otherwise skipped) cell is named in the finished readout too, so
        // `/rftoolsbuilder status` answers the exact `N placed, M skipped` the filter proves.
        return format!(
            "{head}, {} {action}{voided}, {} skipped, scan complete",
            status.progress.placed, status.progress.skipped
        );
    }
    if status.progress.placed == 0 && status.progress.skipped == 0 && status.progress.voided == 0 {
        return format!("{head}, not started");
    }
    format!(
        "{head}, {} {action}{voided}, {} skipped",
        status.progress.placed, status.progress.skipped
    )
}

// ---------------------------------------------------------------------------
// Chunk loading: the force-load branch of `BuilderTileEntity.chunkLoad`
// ---------------------------------------------------------------------------

/// The chunk `(x >> 4, z >> 4)` holding a block, with the jar's Java arithmetic
/// shift (negative coordinates round toward negative infinity, as `ChunkPos` does).
#[must_use]
pub fn chunk_of(pos: [i32; 3]) -> [i32; 2] {
    [pos[0] >> 4, pos[2] >> 4]
}

/// The ticket change a Builder must make on one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TicketPlan {
    /// The chunk whose ticket is released (the one held on the previous tick).
    pub release: Option<[i32; 2]>,
    /// The chunk whose ticket is acquired (the current cell's chunk).
    pub force: Option<[i32; 2]>,
}

/// The jar's `chunkLoad` decision when the Builder waits at an unloaded cell:
/// force that cell's chunk, releasing the chunk it held before. Holding the
/// same chunk again is a no-op — the jar's `cp.equals(forcedChunk)` early return.
#[must_use]
pub fn ticket_step(held: Option<[i32; 2]>, at: [i32; 3]) -> TicketPlan {
    let chunk = chunk_of(at);
    if held == Some(chunk) {
        TicketPlan::default()
    } else {
        TicketPlan {
            release: held,
            force: Some(chunk),
        }
    }
}

/// The release-only decision when a pass ends, a card is removed, or the block
/// is broken: the jar's `chunkUnload`.
#[must_use]
pub fn ticket_release(held: Option<[i32; 2]>) -> TicketPlan {
    TicketPlan {
        release: held,
        force: None,
    }
}

/// The ticket owner the jar uses is a UUID (`getOwnerUUID` in `chunkLoad` and
/// `chunkUnload`), not a block position. This port has no persistent owner UUID,
/// so it derives a stable one from the Builder's block: two Builders in one
/// chunk release separately, and the same Builder keeps its ticket across ticks.
#[must_use]
pub fn ticket_owner(pos: [i32; 3]) -> u128 {
    let coord = |value: i32| (value as u32) as u128;
    // `RFB` in the high 32 bits, then the three signed coordinates in 32 bits
    // each. Non-zero, stable, and unique per Builder position.
    (0x5246_4275u128 << 96) | (coord(pos[0]) << 64) | (coord(pos[1]) << 32) | coord(pos[2])
}

/// The words `/rftoolsbuilder forced` uses for one Builder's held chunks. It
/// starts with `Forced` so a status readout can never be mistaken for it.
#[must_use]
pub fn forced_line(at: [i32; 3], chunks: &[[i32; 2]]) -> String {
    let [x, y, z] = at;
    if chunks.is_empty() {
        format!("Forced chunks at {x} {y} {z}: none")
    } else {
        let list = chunks
            .iter()
            .map(|[cx, cz]| format!("[{cx},{cz}]"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("Forced chunks at {x} {y} {z}: {list}")
    }
}

// ---------------------------------------------------------------------------
// Energy: the Builder's FE buffer and the jar's per-cell cost (slice 6)
// ---------------------------------------------------------------------------

/// The jar's `ShapeCardType` cost supplier for one cell: a shaped build, a quarry break,
/// a void-card removal, or a filtered-out skip. The amounts are the config values above.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellCharge {
    /// `CARD_SHAPE`'s `builderRfPerOperation`.
    Build,
    /// `CARD_QUARRY*`'s `builderRfPerQuarry`.
    Quarry,
    /// `CARD_VOID`'s `builderRfPerOperation × voidShapeCardFactor`.
    Void,
    /// `builderRfPerSkipped` (a filter the port does not carry).
    Skipped,
}

/// One cell's base FE cost, from the jar's config suppliers.
#[must_use]
pub fn cell_cost(charge: CellCharge) -> u32 {
    match charge {
        CellCharge::Build => RF_PER_OPERATION,
        CellCharge::Quarry => RF_PER_QUARRY,
        CellCharge::Void => (RF_PER_OPERATION as f64 * VOID_RF_FACTOR) as u32,
        CellCharge::Skipped => RF_PER_SKIPPED,
    }
}

/// `BuilderTileEntity.getEnergyForBlock`'s infusion multiplier, `(3 - factor) / 3`. The jar
/// reads `DefaultInfusable.getInfusedFactor()`; with no infuser this package always pays the
/// full base cost. Factor 0 is 1 and a fully infused factor of 3 is free.
#[must_use]
pub fn infusion_multiplier(factor: f32) -> f32 {
    (3.0 - factor) / 3.0
}

/// A cell's cost at a given infusion factor. The port keeps the jar's formula but substitutes
/// [`INFUSION_FACTOR`] for its own un-infused builder, so the in-game cost is exact.
#[must_use]
pub fn adjusted_cost(cost: u32, factor: f32) -> u32 {
    (f64::from(cost) * f64::from(infusion_multiplier(factor))) as u32
}

/// What `/rftoolsbuilder energy <pos>` reports: the saved buffer and what this Builder has spent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EnergyStatus {
    /// The Builder's position.
    pub at: [i32; 3],
    /// FE currently in the buffer (the saved compound's `energy` int).
    pub stored: u32,
    /// FE spent by this Builder's passes so far (`energySpent`).
    pub spent: u64,
    /// FE spent specifically on filtered-out cells (`builderRfPerSkipped`), so an in-game
    /// assertion can read `M × 50` for the skipped cells apart from the build/quarry spend.
    pub filter_skipped: u64,
}

/// The parsed energy readout: stored/capacity, lifetime spend, every per-cell cost and the
/// infusion factor, so a charge-and-quarry assertion reads the numbers, not the absence of an
/// error. Byte-stable; the format is pinned by `the_energy_readout_prints_the_buffer_and_what_was_spent`.
#[must_use]
pub fn energy_line(status: &EnergyStatus) -> String {
    let [x, y, z] = status.at;
    format!(
        "Builder at {x} {y} {z}: energy {}/{} FE, spent {} FE, build {} FE/cell, quarry {} FE/cell, void {} FE/cell, skip {} FE/cell, infusion {}, filter skipped {} FE",
        status.stored,
        ENERGY_CAPACITY,
        status.spent,
        cell_cost(CellCharge::Build),
        cell_cost(CellCharge::Quarry),
        cell_cost(CellCharge::Void),
        cell_cost(CellCharge::Skipped),
        INFUSION_FACTOR as i32,
        status.filter_skipped,
    )
}

// ---------------------------------------------------------------------------
// The pump/liquid cards (slice 9): the Builder's fluid tank and the bucket rule
// ---------------------------------------------------------------------------

/// What a pump/liquid cell needs to know, mirroring `BuilderTileEntity.pumpBlock`
/// and `placeLiquidBlock` inputs (`FluidState.isSource`, `getDestroySpeed`,
/// `isEmptyOrReplacable` and what the tank holds).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FluidFacts<'a> {
    /// The cell is inside the Builder's own 3×3×3 box.
    pub near_builder: bool,
    /// The fluid block id at this cell when it is a **source** (`LiquidBlock` with
    /// `FluidState.isSource()`); `None` for air, a flowing fluid or a solid block.
    pub source: Option<&'static str>,
    /// `BlockState.getDestroySpeed`: below zero means the block cannot be broken.
    pub destroy_speed: f32,
    /// `isEmptyOrReplacable`: a liquid cell may be written here.
    pub replaceable: bool,
    /// What the Builder's tank holds, borrowed from the BE's saved compound.
    pub tank_fluid: Option<&'a str>,
    /// The tank's amount in millibuckets.
    pub tank_amount: u32,
}

/// The early returns of `pumpBlock`/`placeLiquidBlock`: the cell is left alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluidSkip {
    /// The Builder's own 3×3×3 box.
    NearBuilder,
    /// No fluid source here (`FluidState.isEmpty()` or `!isSource()`).
    NotSource,
    /// `getDestroySpeed < 0`: bedrock and the like.
    Unbreakable,
    /// The cell already holds a block that a liquid cannot replace.
    NotReplaceable,
}

/// A wait at a fluid cell: the jar's `waitOrSkip` suspension, with its message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluidWait {
    /// `checkFluidTank` refused a whole 1000 mB ("No room for liquid or no usable
    /// tank above or below!").
    NoRoom,
    /// `findAndConsumeLiquid` found no bucket ("Cannot find liquid!").
    NoLiquid,
}

/// What the tick does at one pump/liquid cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FluidStep {
    /// Leave the cell alone.
    Skip(FluidSkip),
    /// Remove the fluid source and add one bucket to the tank (`pumpBlock`).
    PumpIn,
    /// Drain one bucket from the tank and write its fluid block (`placeLiquidBlock`).
    Place,
    /// Wait here; nothing moves.
    Wait(FluidWait),
}

/// The jar's per-cell decision, in the order of its two handlers. The rule is
/// pure: filling/draining the tank and writing the block are the glue's.
#[must_use]
pub fn fluid_step(kind: CardKind, facts: FluidFacts<'_>) -> FluidStep {
    if facts.near_builder {
        return FluidStep::Skip(FluidSkip::NearBuilder);
    }
    match kind {
        CardKind::Pump(_) => {
            let Some(fluid) = facts.source else {
                return FluidStep::Skip(FluidSkip::NotSource);
            };
            if facts.destroy_speed < 0.0 {
                return FluidStep::Skip(FluidSkip::Unbreakable);
            }
            // `checkFluidTank`'s SIMULATE fill must take a whole bucket, and a
            // tank that already holds another fluid refuses this one.
            if facts.tank_amount + BUCKET_MB > FLUID_TANK_CAPACITY
                || facts.tank_fluid.is_some_and(|held| held != fluid)
            {
                return FluidStep::Wait(FluidWait::NoRoom);
            }
            FluidStep::PumpIn
        }
        CardKind::Liquid => {
            if !facts.replaceable {
                return FluidStep::Skip(FluidSkip::NotReplaceable);
            }
            if facts.tank_amount < BUCKET_MB || facts.tank_fluid.is_none() {
                return FluidStep::Wait(FluidWait::NoLiquid);
            }
            FluidStep::Place
        }
        CardKind::Shape | CardKind::Quarry(_) => FluidStep::Skip(FluidSkip::NotSource),
    }
}

/// The tank after one step: a pump adds a whole bucket, a liquid placement spends
/// one, and an empty tank becomes `None` (the jar's `FluidStack.EMPTY`).
#[must_use]
pub fn tank_after(step: FluidStep, fluid: Option<&str>, amount: u32) -> (Option<&str>, u32) {
    match step {
        FluidStep::PumpIn => (fluid, amount + BUCKET_MB),
        FluidStep::Place => (
            if amount <= BUCKET_MB { None } else { fluid },
            amount.saturating_sub(BUCKET_MB),
        ),
        FluidStep::Skip(_) | FluidStep::Wait(_) => (fluid, amount),
    }
}

/// `builderRfPerLiquid` per pumped/placed cell (the jar's pump cost supplier).
#[must_use]
pub fn fluid_cost() -> u32 {
    RF_PER_LIQUID
}

/// What `/rftoolsbuilder fluid <pos>` reports from the Builder's tank.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FluidStatus {
    /// The Builder's position.
    pub at: [i32; 3],
    /// The fluid id in the tank, or `None` when empty.
    pub fluid: Option<String>,
    /// The amount in millibuckets.
    pub amount: u32,
}

/// The parsed fluid readout. Byte-stable; pinned by
/// `the_fluid_readout_names_the_fluid_and_exact_millibuckets`.
#[must_use]
pub fn fluid_line(status: &FluidStatus) -> String {
    let [x, y, z] = status.at;
    format!(
        "Builder at {x} {y} {z}: fluid {} {} mB (capacity {} mB), bucket {} mB, {} FE/cell",
        status.fluid.as_deref().unwrap_or("none"),
        status.amount,
        FLUID_TANK_CAPACITY,
        BUCKET_MB,
        RF_PER_LIQUID,
    )
}

// ---------------------------------------------------------------------------
// The filter module (slice 7): filtered placement and the per-skip cost
// ---------------------------------------------------------------------------

/// The filter's mode, mirroring RFToolsBase's `FilterModuleData.blacklist`: an allow-list
/// (`false`) passes only the configured ids/tags, a deny-list (`true`) passes everything else.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterMode {
    /// `blacklist = false`: only configured blocks/tags pass; everything else is skipped.
    #[default]
    Allow,
    /// `blacklist = true`: configured blocks/tags are skipped; everything else passes.
    Deny,
}

impl FilterMode {
    /// The word `/rftoolsbuilder filter` reads and `/status` prints.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        }
    }

    /// Parse the jar's `blacklist` word, accepting the player-facing `allow`/`deny` too.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "allow" | "whitelist" | "false" => Some(Self::Allow),
            "deny" | "blacklist" | "true" => Some(Self::Deny),
            _ => None,
        }
    }
}

/// A filter configuration: one value (a block id such as `minecraft:stone`, or a `#tag`) and
/// a mode. The jar's `FilterModuleData` holds lists of item stacks and tags plus `blacklist`;
/// this port's `/rftoolsbuilder filter` authors one entry per card, which is enough to prove the
/// allow/deny and id/tag semantics (the multi-entry GUI is not carried — recorded in notes.md).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    /// `blacklist`: allow-list or deny-list.
    pub mode: FilterMode,
    /// The configured block id, or a `#`-prefixed block tag. Empty means no filter.
    pub value: String,
}

impl Filter {
    /// Whether the filter is absent: upstream's slot 1 being empty (`getCache` returns null),
    /// in which case no cell is filtered and the skip cost never fires.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }
}

/// Whether a cell's block passes the filter — the jar's `FilterModuleCache.test(cloneStack)`:
/// a `#tag` entry matches any block in that tag, a bare entry matches the block id, and the mode
/// inverts the match for a deny-list. An empty filter passes every block (no predicate in slot 1).
///
/// `in_tag` is supplied by the host: a pure closure here keeps this rule unit-testable without
/// the game (the tag table is the only game-dependent part).
#[must_use]
pub fn filter_match(filter: &Filter, block: &str, in_tag: impl Fn(&str, &str) -> bool) -> bool {
    if filter.is_empty() {
        return true;
    }
    let matched = match filter.value.strip_prefix('#') {
        Some(tag) => in_tag(block, tag),
        None => filter.value == block,
    };
    match filter.mode {
        FilterMode::Allow => matched,
        FilterMode::Deny => !matched,
    }
}

/// The charge for a cell the filter rejected: the jar's `builderRfPerSkipped` (50 FE), consumed
/// even though nothing was placed (`BuilderTileEntity` consumes it just before `skip(data)`).
#[must_use]
pub fn filter_skip_cost() -> u32 {
    cell_cost(CellCharge::Skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Builder at the origin, so a cell reads as its own coordinates.
    const AT: [i32; 3] = [0, 0, 0];

    fn card(shape: Shape, solid: bool, dim: [i32; 3]) -> Card {
        Card {
            shape,
            solid,
            dimension: dim,
            offset: [0, 0, 0],
            modifier: Modifier::default(),
            kind: CardKind::Shape,
            voiding: Voiding::empty(),
            tag_matching: false,
        }
    }

    fn count(shape: Shape, solid: bool, dim: [i32; 3]) -> u64 {
        card(shape, solid, dim).count(u64::MAX).0
    }

    // --- the shapes' exact cell counts ------------------------------------

    /// The tick's walk, exactly as `support::tick` drives it: at most `BLOCKS_PER_TICK` placements
    /// and `LOOKUPS_PER_TICK` cells a tick, one tick at a time, with a fake world that holds the
    /// scenario's own blocks (the Builder, the chest above it, the pedestal) and everything an
    /// earlier pass built. Returns (cells visited in shape, placed, ticks).
    fn walk_pass(
        origin: [i32; 3],
        card: Card,
        occupied: &mut std::collections::BTreeSet<[i32; 3]>,
        ticks: u32,
    ) -> (u64, u64, u32) {
        let mut scan = Scan::start(origin, card);
        let mut visited = 0u64;
        let mut placed = 0u64;
        let mut used = 0u32;
        for _ in 0..ticks {
            used += 1;
            let mut this_tick = 0u32;
            let mut steps = 0u32;
            while this_tick < BLOCKS_PER_TICK && steps < LOOKUPS_PER_TICK {
                let Some(cursor) = scan.cursor else { break };
                steps += 1;
                if card.contains(origin, cursor) {
                    visited += 1;
                    if !occupied.contains(&cursor) {
                        occupied.insert(cursor);
                        placed += 1;
                        this_tick += 1;
                    }
                }
                if !scan.advance() {
                    return (visited, placed, used);
                }
            }
        }
        (visited, placed, used)
    }

    #[test]
    fn a_second_pass_over_a_hollow_box_visits_and_builds_every_cell() {
        // The harness's sequence: a solid 3x3x3 pass on a Builder that has a chest above it and a
        // pedestal under the box's corner, then a hollow 5x5x5 pass on the same Builder. Every cell
        // of the shell must be visited once and built: a walk that under-visits shows up here as
        // fewer placed cells than the shape holds.
        let at = [0, 0, 0];
        let mut occupied: std::collections::BTreeSet<[i32; 3]> = std::collections::BTreeSet::new();
        occupied.insert(at); // the Builder itself
        occupied.insert([0, 1, 0]); // the chest above it
        occupied.insert([0, 0, -1]); // the pedestal under the box's corner

        let solid = Card {
            shape: Shape::Box,
            solid: true,
            dimension: [3, 3, 3],
            ..Card::default()
        };
        let (visited, placed, _) = walk_pass(at, solid, &mut occupied, 200);
        assert_eq!(visited, 27, "every cell of a solid 3x3x3 box");
        assert_eq!(
            placed, 24,
            "the Builder, the chest above it and the pedestal cannot take a block"
        );

        let hollow = Card {
            shape: Shape::Box,
            solid: false,
            dimension: [5, 5, 5],
            ..Card::default()
        };
        let (visited, placed, ticks) = walk_pass(at, hollow, &mut occupied, 400);
        assert_eq!(visited, 98, "every shell cell is looked at exactly once");
        assert_eq!(
            placed, 98,
            "and every shell cell is built (none of them is the chest, the Builder or the pedestal)"
        );
        assert!(ticks < 400, "the pass finishes inside its ticks");
    }

    #[test]
    fn a_box_holds_exactly_its_dimensions() {
        assert_eq!(count(Shape::Box, true, [3, 3, 3]), 27);
        assert_eq!(count(Shape::Box, true, [5, 5, 5]), 125);
        assert_eq!(count(Shape::Box, true, [1, 1, 1]), 1);
        let box3 = card(Shape::Box, true, [3, 3, 3]);
        assert_eq!(box3.min_corner(AT), [-1, -1, -1]);
        assert_eq!(box3.max_corner(AT), [2, 2, 2]);
        assert!(box3.contains(AT, [-1, -1, -1]));
        assert!(!box3.contains(AT, [2, 2, 2]));
    }

    #[test]
    fn a_hollow_box_is_its_shell() {
        assert_eq!(count(Shape::Box, false, [3, 3, 3]), 26);
        assert_eq!(count(Shape::Box, false, [5, 5, 5]), 98);
        let hollow = card(Shape::Box, false, [3, 3, 3]);
        assert!(hollow.contains(AT, [-1, -1, -1]), "a corner is shell");
        assert!(
            !hollow.contains(AT, [0, 0, 0]),
            "the middle of a 3×3×3 box is not"
        );
    }

    #[test]
    fn the_sphere_is_eighty_one_cells_at_five() {
        assert_eq!(count(Shape::Sphere, true, [5, 5, 5]), 81);
        // A sphere is never a full cube: at 3 the ellipse test keeps only the
        // centre, the six face neighbours and the twelve edges (19), not 27.
        assert_eq!(count(Shape::Sphere, true, [3, 3, 3]), 19);
    }

    #[test]
    fn the_domes_are_the_spheres_halves() {
        assert_eq!(count(Shape::TopDome, true, [5, 5, 5]), 51);
        assert_eq!(count(Shape::BottomDome, true, [5, 5, 5]), 51);
        let top = card(Shape::TopDome, true, [5, 5, 5]);
        assert!(top.contains(AT, [0, 2, 0]));
        assert!(!top.contains(AT, [0, -2, 0]));
    }

    #[test]
    fn the_cylinder_ignores_its_height_and_the_capped_one_clips_it() {
        assert_eq!(count(Shape::Cylinder, true, [5, 5, 5]), 105);
        assert_eq!(count(Shape::CappedCylinder, true, [5, 5, 5]), 105);
        assert_eq!(count(Shape::Cone, true, [5, 5, 5]), 21);
        assert_eq!(count(Shape::Prism, true, [5, 5, 5]), 35);
        assert_eq!(count(Shape::Torus, true, [5, 5, 5]), 19);
    }

    #[test]
    fn hollow_cylinders_keep_the_jars_heightless_border_test() {
        // `isBorder` asks `isInside` of the six neighbours, and the cylinder's
        // `isInside` ignores y, so the vertical neighbours always read as
        // inside: a hollow cylinder has no cap at all, only its rim, whatever
        // its height.
        assert_eq!(count(Shape::Cylinder, false, [5, 1, 5]), 12);
        assert_eq!(count(Shape::Cylinder, false, [5, 3, 5]), 36);
        // The capped cylinder's `isInside` does look at y, so it does cap.
        assert_eq!(count(Shape::CappedCylinder, false, [5, 1, 5]), 21);
    }

    #[test]
    fn the_sphere_mirrors_on_every_axis() {
        let sphere = card(Shape::Sphere, true, [7, 7, 7]);
        let (cells, _) = sphere.cells(AT, u64::MAX);
        assert!(cells.len() > 100, "a 7×7×7 sphere is not tiny");
        for [x, y, z] in cells {
            for mirror in [[-x, y, z], [x, -y, z], [x, y, -z]] {
                assert!(
                    sphere.contains(AT, mirror),
                    "sphere loses {mirror:?} while holding {}, {y}, {z}",
                    x
                );
            }
        }
    }

    #[test]
    fn the_heart_keeps_the_jars_uneven_formula() {
        // The heart's three constants (`+ .1`, `+ .2`, `+ .1`) push its surface
        // off-centre, so it mirrors on no axis; a port that dropped one would
        // gain a mirror here.
        let heart = card(Shape::Heart, true, [9, 9, 9]);
        let (cells, _) = heart.cells(AT, u64::MAX);
        assert_eq!(cells.len(), 247);
        let broken = |mirror: fn([i32; 3]) -> [i32; 3]| {
            cells
                .iter()
                .filter(|pos| !heart.contains(AT, mirror(**pos)))
                .count()
        };
        assert!(broken(|[x, y, z]| [-x, y, z]) > 0, "x is nudged by +.1");
        assert!(broken(|[x, y, z]| [x, -y, z]) > 0, "y is nudged by +.2");
        assert!(broken(|[x, y, z]| [x, y, -z]) > 0, "z is nudged by +.1");
    }

    #[test]
    fn cells_come_in_the_jars_x_z_y_order() {
        let (cells, _) = card(Shape::Box, true, [2, 2, 2]).cells(AT, u64::MAX);
        assert_eq!(cells.len(), 8);
        assert_eq!(cells[0], [-1, -1, -1]);
        assert_eq!(cells[1], [-1, 0, -1], "y varies fastest, as composeFormula");
        assert_eq!(cells[2], [-1, -1, 0], "then z");
        assert_eq!(cells[4], [0, -1, -1], "then x");
    }

    // --- the offset, the clamps and the caps -------------------------------

    #[test]
    fn the_offset_moves_the_shape_and_not_the_builder() {
        let moved = Card {
            offset: [10, 0, 0],
            ..card(Shape::Box, true, [3, 3, 3])
        };
        assert_eq!(moved.min_corner(AT), [9, -1, -1]);
        let (cells, _) = moved.cells(AT, u64::MAX);
        assert!(cells.iter().all(|pos| (9..12).contains(&pos[0])));
        assert_eq!(cells.len(), 27);
    }

    #[test]
    fn an_offset_leaving_the_world_places_nothing() {
        let below = Card {
            offset: [0, -200, 0],
            ..card(Shape::Box, true, [3, 3, 3])
        };
        let (cells, _) = below.cells([0, 100, 0], u64::MAX);
        assert_eq!(cells.len(), 27);
        assert!(
            clip_to_height(&cells, -64, 320).is_empty(),
            "every cell is under the world"
        );
        assert!(!in_height(cells[0], -64, 320));
        // A shape half in and half out keeps only what the world holds.
        let straddling = Card {
            offset: [0, -4, 0],
            ..card(Shape::Box, true, [5, 5, 5])
        };
        let (cells, _) = straddling.cells([0, -62, 0], u64::MAX);
        let kept = clip_to_height(&cells, -64, 320);
        assert_eq!(cells.len(), 125);
        assert!(kept.len() < cells.len());
        assert!(kept.iter().all(|pos| pos[1] >= -64));
    }

    #[test]
    fn every_card_is_clamped_to_the_jars_numbers() {
        let huge = Card {
            dimension: [1000, 4, 4],
            offset: [900, -900, 0],
            ..card(Shape::Box, true, [1, 1, 1])
        }
        .clamped();
        assert_eq!(huge.dimension, [512, 4, 4]);
        assert_eq!(huge.offset, [260, -260, 0]);
        assert_eq!(MAX_DIMENSION, 512);
        assert_eq!(MAX_OFFSET, 260);
    }

    #[test]
    fn a_card_past_the_limit_is_refused_with_the_jars_numbers() {
        let too_big = Card {
            dimension: [1000, 1000, 1000],
            ..card(Shape::Box, true, [1, 1, 1])
        };
        let error = too_big.validate().expect_err("512³ is past 50M");
        assert_eq!(
            error,
            CardError::TooLarge {
                cells: 134_217_728,
                limit: 50_000_000
            }
        );
        assert_eq!(MAX_CARD_CELLS, 50_000_000);
        assert_eq!(
            error.to_string(),
            "shape of 134217728 cells past the 50000000-cell card limit"
        );
        assert!(card(Shape::Box, true, [100, 100, 100]).validate().is_ok());
        assert!(
            card(Shape::Box, true, [400, 400, 400]).validate().is_err(),
            "64M cells is past the card limit"
        );
    }

    #[test]
    fn a_pass_keeps_at_most_the_space_chambers_cell_cap() {
        let big = card(Shape::Box, true, [200, 200, 200]);
        assert_eq!(big.box_cells(), 8_000_000);
        let (kept, truncated) = big.count(MAX_PASS_CELLS);
        assert_eq!(kept, MAX_PASS_CELLS);
        assert!(truncated);
        assert_eq!(MAX_PASS_CELLS, 128 * 128 * 128);
        let (cells, truncated) = big.cells(AT, 10);
        assert_eq!(cells.len(), 10);
        assert!(truncated);
    }

    // --- the card's parsing ------------------------------------------------

    #[test]
    fn a_card_parses_the_jars_shape_names() {
        assert_eq!(Shape::parse("box"), Some(Shape::Box));
        assert_eq!(Shape::parse("SHAPE_BOX"), Some(Shape::Box));
        assert_eq!(Shape::parse("Box"), Some(Shape::Box));
        assert_eq!(Shape::parse("top dome"), Some(Shape::TopDome));
        assert_eq!(Shape::parse("SHAPE_TOPDOME"), Some(Shape::TopDome));
        assert_eq!(Shape::parse("cappedcylinder"), Some(Shape::CappedCylinder));
        assert_eq!(
            Shape::parse("SHAPE_CAPPEDCYLINDER"),
            Some(Shape::CappedCylinder)
        );
        assert_eq!(Shape::parse("sphere"), Some(Shape::Sphere));
        assert_eq!(Shape::parse("heart"), Some(Shape::Heart));
        assert_eq!(Shape::parse("nonsense"), None);
        assert_eq!(Shape::parse("SHAPE_COMPOSITION"), None, "not carried");
        assert_eq!(Shape::parse("SHAPE_SCAN"), None, "not carried");
        assert_eq!(Shape::Box.id(), "SHAPE_BOX");
        assert_eq!(Shape::Box.description(), "Box");
        assert_eq!(Rotation::parse("0"), Some(Rotation::None));
        assert_eq!(Rotation::parse("y"), Some(Rotation::Y));
        assert_eq!(Rotation::parse("Q"), None);
        assert_eq!(Rotation::Y.transform_dimension([7, 3, 5]), [5, 3, 7]);
    }

    // --- the walk ----------------------------------------------------------

    #[test]
    fn the_walk_steps_x_then_z_then_down() {
        let min = [0, 0, 0];
        let max = [3, 3, 3];
        // x first.
        assert_eq!(next_cursor([0, 2, 0], min, max), Some([1, 2, 0]));
        // The row done: z first, x back to the box's own low corner.
        assert_eq!(next_cursor([2, 2, 0], min, max), Some([0, 2, 1]));
        // The layer done: down, x and z back to the low corner.
        assert_eq!(next_cursor([2, 2, 2], min, max), Some([0, 1, 0]));
        // The box done: the pass is over.
        assert_eq!(next_cursor([2, 0, 2], min, max), None);
    }

    #[test]
    fn the_walk_looks_at_every_cell_of_a_straddling_box() {
        // The default 5×5×5 box for a Builder on a chunk boundary: the jar's
        // chunk-snapping walk reaches only 50 of these 125 cells, on every
        // pass (notes.md, the defect this walk fixes).
        let card = card(Shape::Box, true, [5, 5, 5]);
        let mut scan = Scan::start(AT, card);
        let mut seen = Vec::new();
        loop {
            seen.push(scan.cursor.expect("a running scan has a cursor"));
            if !scan.advance() {
                break;
            }
        }
        let (cells, _) = card.cells(AT, u64::MAX);
        seen.retain(|pos| card.contains(AT, *pos));
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(cells.len(), 125);
        assert_eq!(seen.len(), 125, "every cell is looked at in one pass");
        for cell in &cells {
            assert!(seen.contains(cell), "{cell:?} is never looked at");
        }
        let start = Scan::start(AT, card);
        assert_eq!(scan.cursor, start.cursor, "the pass restarts at the top");
    }

    #[test]
    fn a_scan_starts_on_the_top_cell_of_the_low_corner() {
        let scan = Scan::start([10, 64, 10], card(Shape::Box, true, [3, 3, 3]));
        assert_eq!(scan.min, [9, 63, 9]);
        assert_eq!(scan.max, [12, 66, 12]);
        assert_eq!(scan.cursor, Some([9, 65, 9]));
    }

    #[test]
    fn a_resumed_cursor_outside_the_box_starts_the_pass_again() {
        let card = card(Shape::Box, true, [3, 3, 3]);
        let inside = Scan::resume(AT, card, Some([0, 0, 0]));
        assert_eq!(inside.cursor, Some([0, 0, 0]));
        let outside = Scan::resume(AT, card, Some([40, 40, 40]));
        assert_eq!(outside.cursor, Scan::start(AT, card).cursor);
        let lower = Scan::resume(AT, card, Some([0, -40, 0]));
        assert_eq!(lower.cursor, Scan::start(AT, card).cursor);
    }

    #[test]
    fn a_scan_is_clipped_to_the_worlds_build_height() {
        // Under the world: nothing left to build.
        let mut under = Scan::start([0, -70, 0], card(Shape::Box, true, [5, 5, 5]));
        assert_eq!(under.min[1], -72);
        assert!(!under.clamp_y(-64, 320));
        // Half under: only the part the world holds is walked.
        let mut half = Scan::start([0, -64, 0], card(Shape::Box, true, [5, 5, 5]));
        assert!(half.clamp_y(-64, 320));
        assert_eq!(half.min, [-2, -64, -2]);
        assert_eq!(half.max, [3, -61, 3]);
        let mut seen = Vec::new();
        loop {
            seen.push(half.cursor.expect("a running scan has a cursor"));
            if !half.advance() {
                break;
            }
        }
        seen.retain(|pos| card(Shape::Box, true, [5, 5, 5]).contains([0, -64, 0], *pos));
        assert!(!seen.is_empty());
        assert!(seen.iter().all(|pos| pos[1] >= -64), "{seen:?}");
    }

    // --- the turn and the mirror -------------------------------------------

    #[test]
    fn the_mirror_turns_the_cone_up_side_down() {
        let cone = card(Shape::Cone, true, [5, 5, 5]);
        let flipped = Card {
            modifier: Modifier {
                rotation: Rotation::None,
                flip_y: true,
            },
            ..cone
        };
        assert!(
            cone.contains(AT, [1, -2, 0]),
            "the cone's base is three wide"
        );
        assert!(
            !flipped.contains(AT, [1, -2, 0]),
            "the mirror puts the wide end at the top"
        );
        assert_eq!(flipped.count(u64::MAX).0, cone.count(u64::MAX).0);
    }

    #[test]
    fn a_turn_keeps_a_box_and_turns_a_heart() {
        // `FormulaComposition` clips to the *turned* bounds; a box fills its
        // turned box (5 × 3 × 7), so turning it changes nothing.
        let turned = Card {
            modifier: Modifier {
                rotation: Rotation::Y,
                flip_y: false,
            },
            ..card(Shape::Box, true, [7, 3, 5])
        };
        assert_eq!(turned.min_corner(AT), [-2, -1, -3]);
        assert_eq!(turned.max_corner(AT), [3, 2, 4]);
        assert_eq!(turned.count(u64::MAX).0, 105);
        assert!(!turned.contains(AT, [-3, 0, 0]), "outside the turned box");
        // A shape that is not symmetric does change under the turn.
        let heart = card(Shape::Heart, true, [9, 9, 9]);
        let spun = Card {
            modifier: Modifier {
                rotation: Rotation::X,
                flip_y: false,
            },
            ..heart
        };
        let (plain, _) = heart.cells(AT, u64::MAX);
        let (rotated, _) = spun.cells(AT, u64::MAX);
        assert!(!plain.is_empty() && !rotated.is_empty());
        assert_ne!(plain, rotated, "a turn must turn an uneven shape");
        assert!(rotated.iter().all(|pos| {
            (0..3).all(|axis| {
                pos[axis] >= spun.min_corner(AT)[axis] && pos[axis] < spun.max_corner(AT)[axis]
            })
        }));
    }

    // --- the readout -------------------------------------------------------

    #[test]
    fn the_status_line_is_byte_stable() {
        let at = [10, 64, 10];
        assert_eq!(
            status_line(&Status {
                at,
                card: None,
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
                replace: String::new(),
            }),
            "Builder at 10 64 10: no shape card"
        );
        let sphere = Card {
            shape: Shape::Sphere,
            ..Card::default()
        };
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
                replace: String::new(),
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, not started"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress {
                    placed: 12,
                    skipped: 40,
                    voided: 0,
                },
                refusal: None,
                complete: false,
                cells: None,
                replace: String::new(),
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, 12 placed, 40 skipped"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress {
                    placed: 81,
                    skipped: 0,
                    voided: 0,
                },
                refusal: None,
                complete: true,
                cells: None,
                replace: String::new(),
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, 81 placed, scan complete"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: Some(sphere),
                refused_card: None,
                progress: Progress {
                    placed: 12,
                    skipped: 3,
                    voided: 0,
                },
                refusal: Some(Refusal {
                    at: [10, 68, 10],
                    reason: RefusalReason::NoMaterial,
                }),
                complete: false,
                cells: None,
                replace: String::new(),
            }),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, 12 placed, \
             refused at 10 68 10: no block to place (put one above or below the builder)"
        );
        assert_eq!(
            status_line(&Status {
                at,
                card: None,
                refused_card: Some(CardError::TooLarge {
                    cells: 134_217_728,
                    limit: MAX_CARD_CELLS,
                }),
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
                replace: String::new(),
            }),
            "Builder at 10 64 10: card refused, shape of 134217728 cells past the 50000000-cell card limit"
        );
    }

    #[test]
    fn a_status_count_is_bounded_on_a_huge_sparse_shape() {
        // A 499x499x499 sphere is sparse enough that a shell of candidates is mostly outside the
        // shape. Counting all of them is ~124 million `contains` calls on the server thread, which
        // is what the review measured; the readout now stops after STATUS_CANDIDATES candidates.
        let sphere = card(Shape::Sphere, true, [499, 499, 499]);
        let (shown, more) = sphere.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES);
        assert!(more, "the answer says the shape holds more");
        assert!(shown <= STATUS_CANDIDATES, "it counted {shown} cells");
        let small = card(Shape::Sphere, true, [199, 199, 199]);
        let (whole, _) = small.count(MAX_PASS_CELLS);
        let (small_shown, small_more) = small.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES);
        assert!(
            small_more || small_shown <= whole,
            "never more than the whole shape"
        );
        // The work bound is what stops it: with a slightly larger budget the same shape counts
        // more cells, so the number is the budget's, not a constant.
        let (bigger, _) = sphere.count_within(MAX_PASS_CELLS, STATUS_CANDIDATES * 2);
        assert!(
            bigger > shown,
            "a bigger budget counts more: {bigger} vs {shown}"
        );
        // A box answers from its dimensions, exactly, and never inspects a cell: with no work
        // budget at all it still answers.
        let boxed = card(Shape::Box, true, [300, 300, 300]);
        let (cells, more) = boxed.count_within(MAX_PASS_CELLS, 0);
        assert_eq!(cells, MAX_PASS_CELLS);
        assert!(more);
    }

    #[test]
    fn the_readout_prints_the_count_it_was_given() {
        // A cached count (the pass computed it once) is printed as it stands, so a repeated
        // readout never re-walks the shape.
        let sphere = Card {
            shape: Shape::Sphere,
            ..Card::default()
        };
        let with_cache = |cells: u64, more: bool| {
            status_line(&Status {
                at: [10, 64, 10],
                card: Some(sphere),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: Some(CellCount { cells, more }),
                replace: String::new(),
            })
        };
        assert_eq!(
            with_cache(81, false),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 81 cells, not started"
        );
        assert_eq!(
            with_cache(1000, true),
            "Builder at 10 64 10: Sphere (Solid) 5x5x5 offset 0 0 0, 1000 cells (first 1000 cells), \
             not started"
        );
    }

    #[test]
    fn a_protected_cell_has_words_a_player_reads() {
        assert_eq!(
            RefusalReason::Protected.text(),
            "protected (outside the world border)"
        );
    }

    #[test]
    fn the_refusal_reasons_are_the_words_a_player_reads() {
        assert_eq!(
            RefusalReason::NoMaterial.text(),
            "no block to place (put one above or below the builder)"
        );
        assert_eq!(
            RefusalReason::OutOfWorld.text(),
            "outside the world's build height"
        );
        assert_eq!(BLOCKS_PER_TICK, 8);
        assert_eq!(LOOKUPS_PER_TICK, 800);
    }

    #[test]
    fn a_truncated_shape_says_so_in_the_readout() {
        let big = Card {
            dimension: [300, 300, 300],
            ..Card::default()
        };
        assert_eq!(
            status_line(&Status {
                at: [0, 64, 0],
                card: Some(big),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
                replace: String::new(),
            }),
            "Builder at 0 64 0: Box (Solid) 300x300x300 offset 0 0 0, 2097152 cells \
             (first 2097152 cells), not started"
        );
    }

    // --- the quarry card type ----------------------------------------------

    /// The six `ShapeCardType.CARD_QUARRY*` constants and their four fields.
    const QUARRY_TYPES: [(&str, Quarry); 6] = [
        (
            "quarry",
            Quarry {
                silk: false,
                fortune: false,
                clearing: false,
                voids: false,
            },
        ),
        (
            "quarry_silk",
            Quarry {
                silk: true,
                fortune: false,
                clearing: false,
                voids: false,
            },
        ),
        (
            "quarry_fortune",
            Quarry {
                silk: false,
                fortune: true,
                clearing: false,
                voids: false,
            },
        ),
        (
            "quarry_clear",
            Quarry {
                silk: false,
                fortune: false,
                clearing: true,
                voids: false,
            },
        ),
        (
            "quarry_clear_silk",
            Quarry {
                silk: true,
                fortune: false,
                clearing: true,
                voids: false,
            },
        ),
        (
            "quarry_clear_fortune",
            Quarry {
                silk: false,
                fortune: true,
                clearing: true,
                voids: false,
            },
        ),
    ];

    #[test]
    fn the_quarry_card_types_are_the_jars_six_constants() {
        assert_eq!(QUARRY_TYPES.len(), 6);
        for (suffix, quarry) in QUARRY_TYPES {
            assert_eq!(quarry.suffix(), suffix, "{quarry:?} writes its suffix");
            assert_eq!(Quarry::parse(suffix), Some(quarry), "{suffix} reads back");
            assert_eq!(
                CardKind::parse(suffix),
                Some(CardKind::Quarry(quarry)),
                "{suffix} is a card type"
            );
            assert_eq!(CardKind::Quarry(quarry).suffix(), suffix);
        }
        assert_eq!(
            Quarry::parse("shape_card_quarry_silk"),
            Some(QUARRY_TYPES[1].1),
            "the jar's item id reads too"
        );
        assert_eq!(Quarry::parse("quarry_nonsense"), None);
        // The words the hudd log prints (`hudLogEntry`), and the clearing line.
        assert_eq!(QUARRY_TYPES[0].1.description(), "Normal quarry");
        assert_eq!(QUARRY_TYPES[1].1.description(), "Silktouch quarry");
        assert_eq!(QUARRY_TYPES[2].1.description(), "Fortune quarry");
        assert_eq!(QUARRY_TYPES[3].1.description(), "Normal quarry");
        // A quarry is a card TYPE, not a shape: the jar has no quarry shape.
        assert_eq!(Shape::parse("quarry"), None);
        assert_eq!(CardKind::parse("shape"), Some(CardKind::Shape));
        assert_eq!(CardKind::Shape.suffix(), "shape");
        assert_eq!(CardKind::default(), CardKind::Shape);
        assert_eq!(Card::default().kind, CardKind::Shape, "a bare card builds");
    }

    #[test]
    fn a_quarry_leaves_what_it_must_not_break() {
        let normal = QUARRY_TYPES[0].1;
        let clear = QUARRY_TYPES[3].1;
        let facts = |near_builder, air, destroy_speed, is_replacement, voided| CellFacts {
            near_builder,
            air,
            destroy_speed,
            is_replacement,
            voided,
            filter_passes: true,
        };
        // The jar's order: the Builder's own 3x3x3, then air, then an unbreakable block,
        // then the replacement block (a non-clearing card only), then the void list.
        assert_eq!(
            quarry_step(normal, facts(true, false, 2.0, false, false)),
            QuarryStep::Skip(SkipReason::NearBuilder)
        );
        assert_eq!(
            quarry_step(normal, facts(false, true, 0.0, false, false)),
            QuarryStep::Skip(SkipReason::Empty)
        );
        assert_eq!(
            quarry_step(normal, facts(false, false, -1.0, false, true)),
            QuarryStep::Skip(SkipReason::Unbreakable),
            "a block that cannot be destroyed is skipped even when it is on the void list"
        );
        assert_eq!(
            quarry_step(normal, facts(false, false, 2.0, true, false)),
            QuarryStep::Skip(SkipReason::Replacement),
            "the quarry does not re-quarry its own replacement"
        );
        assert_eq!(
            quarry_step(clear, facts(false, false, 2.0, true, false)),
            QuarryStep::Harvest,
            "a clearing card takes the replacement block too"
        );
        assert_eq!(
            quarry_step(normal, facts(false, false, 2.0, false, true)),
            QuarryStep::Void
        );
        assert_eq!(
            quarry_step(clear, facts(false, false, 2.0, false, true)),
            QuarryStep::Void
        );
        assert_eq!(
            quarry_step(normal, facts(false, false, 0.6, false, false)),
            QuarryStep::Harvest
        );
        // Silk and fortune change the drops, never the decision.
        for (_, quarry) in QUARRY_TYPES {
            assert_eq!(
                quarry_step(quarry, facts(false, false, 1.5, false, false)),
                QuarryStep::Harvest,
                "{quarry:?}"
            );
        }
    }

    #[test]
    fn a_quarry_cell_the_filter_denies_is_skipped_before_any_drop() {
        let normal = QUARRY_TYPES[0].1;
        let facts = |filter_passes| CellFacts {
            near_builder: false,
            air: false,
            destroy_speed: 2.0,
            is_replacement: false,
            voided: false,
            filter_passes,
        };
        // `commonQuarryBlock` tests `filterCache` on the clone stack after its own early
        // guards (bytecode 298..351); a rejected block is `skip(data)` — it stays in place
        // and no drops are rolled. An accepted one harvests as before.
        assert_eq!(
            quarry_step(normal, facts(false)),
            QuarryStep::Skip(SkipReason::Filtered)
        );
        assert_eq!(quarry_step(normal, facts(true)), QuarryStep::Harvest);
        // The Builder's own box is checked before the filter.
        assert_eq!(
            quarry_step(
                normal,
                CellFacts {
                    near_builder: true,
                    ..facts(false)
                }
            ),
            QuarryStep::Skip(SkipReason::NearBuilder)
        );
    }

    #[test]
    fn the_quarry_filter_runs_before_the_void_path_and_the_drops() {
        let normal = QUARRY_TYPES[0].1;
        let facts = CellFacts {
            near_builder: false,
            air: false,
            destroy_speed: 2.0,
            is_replacement: false,
            voided: true,
            filter_passes: false,
        };
        // Bytecode order: filterCache.test at 298..351, cachedVoidableBlocks at 385, then the
        // drop roll. A denied block is skipped — never voided, and never handing out drops.
        assert_eq!(
            quarry_step(normal, facts),
            QuarryStep::Skip(SkipReason::Filtered)
        );
        assert_eq!(
            quarry_step(
                normal,
                CellFacts {
                    filter_passes: true,
                    ..facts
                }
            ),
            QuarryStep::Void,
            "the void list still applies once the filter accepts the block"
        );
    }

    #[test]
    fn the_proved_filter_rule_decides_the_quarry_path_by_id_and_tag() {
        let deny_stone = Filter {
            mode: FilterMode::Deny,
            value: "minecraft:stone".to_owned(),
        };
        let base = CellFacts {
            near_builder: false,
            air: false,
            destroy_speed: 2.0,
            is_replacement: false,
            voided: false,
            filter_passes: true,
        };
        let decide = |filter_passes| {
            quarry_step(
                QUARRY_TYPES[0].1,
                CellFacts {
                    filter_passes,
                    ..base
                },
            )
        };
        let id_passes = |block: &str| filter_match(&deny_stone, block, |_id, _tag| false);
        assert_eq!(
            decide(id_passes("minecraft:stone")),
            QuarryStep::Skip(SkipReason::Filtered)
        );
        assert_eq!(decide(id_passes("minecraft:dirt")), QuarryStep::Harvest);
        // A `#tag` value denies its members too: this needs a tag filter, not the id filter above.
        let deny_stones = Filter {
            mode: FilterMode::Deny,
            value: "#minecraft:stones".to_owned(),
        };
        let tag_passes = filter_match(&deny_stones, "minecraft:cobblestone", |_id, tag| {
            tag == "minecraft:stones"
        });
        assert_eq!(
            decide(tag_passes),
            QuarryStep::Skip(SkipReason::Filtered),
            "a denied tag rejects its members too"
        );
        assert_eq!(
            filter_skip_cost(),
            50,
            "the quarry skip charges the jar's builderRfPerSkipped, as the shaped path does"
        );
    }

    #[test]
    fn the_void_card_is_the_jars_seventh_card_type() {
        // `ShapeCardType.CARD_VOID` (branch 1.21_neo @ 409049466): resourceSuffix `void`,
        // quarry=false, clearing=false, fortune=false, SingleBlockHandler `voidBlock`, HUD
        // `"    Void mode"` (four leading spaces, stripped here as for every other hudLogEntry),
        // and BuilderConfiguration.voidShapeCardFactor = 0.5 (0..1,000,000).
        let void = Quarry::VOID;
        assert_eq!(void.suffix(), "void");
        assert_eq!(Quarry::parse("void"), Some(void));
        assert_eq!(
            Quarry::parse("shape_card_void"),
            Some(void),
            "the jar's item id reads too"
        );
        assert_eq!(CardKind::parse("void"), Some(CardKind::Quarry(void)));
        assert_eq!(CardKind::Quarry(void).suffix(), "void");
        assert_eq!(void.description(), "Void mode");
        assert_eq!(VOID_RF_FACTOR, 0.5, "builderRfPerQuarry x 0.5 = 150 RF");
        assert!(!void.clearing && !void.fortune && !void.silk);
        assert_eq!(Shape::parse("void"), None, "a card type, not a shape");
    }

    #[test]
    fn a_void_card_voids_every_breakable_cell() {
        let void = Quarry::VOID;
        let facts = |near_builder, air, destroy_speed, is_replacement| CellFacts {
            near_builder,
            air,
            destroy_speed,
            is_replacement,
            voided: false,
            filter_passes: true,
        };
        // The jar's voidBlock keeps the same early guards as the quarry: its own 3x3x3, air,
        // an unbreakable block and the replacement it left on a previous pass.
        assert_eq!(
            quarry_step(void, facts(true, false, 2.0, false)),
            QuarryStep::Skip(SkipReason::NearBuilder)
        );
        assert_eq!(
            quarry_step(void, facts(false, true, 0.0, false)),
            QuarryStep::Skip(SkipReason::Empty)
        );
        assert_eq!(
            quarry_step(void, facts(false, false, -1.0, false)),
            QuarryStep::Skip(SkipReason::Unbreakable)
        );
        assert_eq!(
            quarry_step(void, facts(false, false, 2.0, true)),
            QuarryStep::Skip(SkipReason::Replacement),
            "a second pass over its own replacement does not void again"
        );
        // Every other cell is voided — no per-material list is consulted for this type.
        assert_eq!(
            quarry_step(void, facts(false, false, 2.0, false)),
            QuarryStep::Void
        );
        // A normal quarry on the same facts harvests: the card type, not the rig, changes it.
        assert_eq!(
            quarry_step(QUARRY_TYPES[0].1, facts(false, false, 2.0, false)),
            QuarryStep::Harvest
        );
    }

    #[test]
    fn the_void_list_is_the_jars_materials_and_their_tags() {
        // `ShapeCardItem.getVoidedBlocks`: exact blocks per material, and the tag each
        // material expands to when the card's tag matching is on.
        let material = |id: &str| material_of(id);
        assert_eq!(material("minecraft:stone"), Some("stone"));
        assert_eq!(material("minecraft:cobblestone"), Some("cobble"));
        assert_eq!(material("minecraft:dirt"), Some("dirt"));
        assert_eq!(
            material("minecraft:grass_block"),
            Some("dirt"),
            "dirt's material covers grass too"
        );
        assert_eq!(material("minecraft:sand"), Some("sand"));
        assert_eq!(material("minecraft:gravel"), Some("gravel"));
        assert_eq!(material("minecraft:netherrack"), Some("netherrack"));
        assert_eq!(material("minecraft:end_stone"), Some("endstone"));
        assert_eq!(material("minecraft:andesite"), None, "not a material key");
        assert_eq!(
            material("minecraft:coarse_dirt"),
            None,
            "a tag member, not an id"
        );
        assert_eq!(tag_of("stone"), "c:stones");
        assert_eq!(tag_of("cobble"), "c:cobblestones");
        assert_eq!(tag_of("dirt"), "minecraft:dirt");
        assert_eq!(tag_of("sand"), "c:sands");
        assert_eq!(tag_of("gravel"), "c:gravels");
        assert_eq!(tag_of("netherrack"), "c:netherracks");
        assert_eq!(tag_of("endstone"), "c:end_stones");
        assert_eq!(VOID_MATERIALS.len(), 7);

        let void = Voiding::empty();
        assert!(void.is_empty());
        assert_eq!(void.names(), "");
        let void = void.with("gravel").with("stone").with("nonsense");
        assert!(
            !void.is_empty(),
            "an unknown key is ignored, a known one is kept"
        );
        assert!(void.contains("stone") && void.contains("gravel"));
        assert!(!void.contains("dirt"));
        assert_eq!(
            void.names(),
            "stone, gravel",
            "the jar's own material order, not the insertion order"
        );
        assert_eq!(void.without("stone").names(), "gravel");
        assert_eq!(Voiding::from_names(["dirt", "sand"]).names(), "dirt, sand");
        assert_eq!(
            Voiding::from_names(["nonsense", "dirt"]).names(),
            "dirt",
            "a jar card's unknown key reads as nothing"
        );
    }

    /// A quarry card of `quarry` over a 3x3x3 box at offset 6, as the in-game rig has it.
    fn quarry_card(quarry: Quarry) -> Card {
        Card {
            kind: CardKind::Quarry(quarry),
            dimension: [3, 3, 3],
            offset: [6, 0, 0],
            ..Card::default()
        }
    }

    #[test]
    fn a_quarry_readout_names_its_type_replacement_and_void_list() {
        let at = [10, 64, 10];
        let quarry = Card {
            voiding: Voiding::empty().with("dirt").with("gravel"),
            tag_matching: true,
            ..quarry_card(QUARRY_TYPES[0].1)
        };
        let line = |status: Status| status_line(&status);
        assert_eq!(
            line(Status {
                at,
                card: Some(quarry),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
                replace: "minecraft:dirt".to_owned(),
            }),
            "Builder at 10 64 10: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, \
             quarry (Normal quarry), replacing with minecraft:dirt, voiding dirt, gravel \
             (tags too), not started"
        );
        assert_eq!(
            line(Status {
                at,
                card: Some(quarry),
                refused_card: None,
                progress: Progress {
                    placed: 25,
                    skipped: 0,
                    voided: 2,
                },
                refusal: None,
                complete: true,
                cells: Some(CellCount {
                    cells: 27,
                    more: false
                }),
                replace: "minecraft:dirt".to_owned(),
            }),
            "Builder at 10 64 10: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, \
             quarry (Normal quarry), replacing with minecraft:dirt, voiding dirt, gravel \
             (tags too), 25 quarried, 2 voided, scan complete"
        );
        // A clearing card replaces nothing, so the readout does not claim it does; the
        // clearing flag is the jar's own line beside the hud text.
        assert_eq!(
            line(Status {
                at,
                card: Some(quarry_card(QUARRY_TYPES[3].1)),
                refused_card: None,
                progress: Progress {
                    placed: 27,
                    skipped: 0,
                    voided: 0,
                },
                refusal: None,
                complete: true,
                cells: None,
                replace: "minecraft:cobblestone".to_owned(),
            }),
            "Builder at 10 64 10: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, \
             quarry (Normal quarry, clearing), 27 quarried, scan complete"
        );
        // A shape card is untouched by any of this.
        assert_eq!(
            line(Status {
                at,
                card: Some(card(Shape::Box, true, [3, 3, 3])),
                refused_card: None,
                progress: Progress::default(),
                refusal: None,
                complete: false,
                cells: None,
                replace: "minecraft:cobblestone".to_owned(),
            }),
            "Builder at 10 64 10: Box (Solid) 3x3x3 offset 0 0 0, 27 cells, not started"
        );
        // A quarry waiting at a cell names it, with the quarry words.
        assert_eq!(
            line(Status {
                at,
                card: Some(quarry_card(QUARRY_TYPES[1].1)),
                refused_card: None,
                progress: Progress {
                    placed: 3,
                    skipped: 0,
                    voided: 0,
                },
                refusal: Some(Refusal {
                    at: [17, 64, 10],
                    reason: RefusalReason::NoRoom,
                }),
                complete: false,
                cells: None,
                replace: "minecraft:dirt".to_owned(),
            }),
            "Builder at 10 64 10: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, \
             quarry (Silktouch quarry), replacing with minecraft:dirt, 3 quarried, \
             refused at 17 64 10: no room for the drops (put a container above or below the builder)"
        );
    }

    #[test]
    fn a_void_readout_names_the_void_mode_and_counts_every_cell_as_voided() {
        let line = status_line(&Status {
            at: [22, 102, -7],
            card: Some(quarry_card(Quarry::VOID)),
            refused_card: None,
            progress: Progress {
                placed: 27,
                skipped: 0,
                voided: 27,
            },
            refusal: None,
            complete: true,
            cells: Some(CellCount {
                cells: 27,
                more: false,
            }),
            replace: "minecraft:dirt".to_owned(),
        });
        assert_eq!(
            line,
            "Builder at 22 102 -7: Box (Solid) 3x3x3 offset 6 0 0, 27 cells, \
             quarry (Void mode), replacing with minecraft:dirt, 27 quarried, 27 voided, scan complete"
        );
    }

    #[test]
    fn the_readout_counts_what_a_pass_acted_on() {
        // The 8-a-tick budget is the jar's `quarryBaseSpeed`, and the walk is the same one
        // the shape path uses; a quarry pass over a full 3x3x3 box acts on 27 cells.
        let card = Card {
            voiding: Voiding::empty().with("gravel"),
            ..quarry_card(QUARRY_TYPES[0].1)
        };
        let mut occupied: std::collections::BTreeSet<[i32; 3]> = std::collections::BTreeSet::new();
        let at = [0, 0, 0];
        let mut scan = Scan::start(at, card);
        let mut acted = 0;
        let mut ticks = 0;
        loop {
            ticks += 1;
            let mut this_tick = 0;
            while this_tick < BLOCKS_PER_TICK {
                let Some(cursor) = scan.cursor else { break };
                assert!(card.contains(at, cursor), "{cursor:?} is not in the box");
                // Every cell of this volume is stone but one, which is on the void list.
                let voided = cursor == [7, 0, 0];
                if !occupied.contains(&cursor) {
                    occupied.insert(cursor);
                    acted += 1;
                    this_tick += 1;
                    let _ = voided;
                }
                if !scan.advance() {
                    assert_eq!(acted, 27);
                    assert!(
                        ticks <= 4,
                        "27 cells at 8 a tick take 4 ticks, took {ticks}"
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn the_air_blocks_are_the_jars_is_air() {
        assert!(is_air_block("minecraft:air"));
        assert!(is_air_block("minecraft:cave_air"));
        assert!(is_air_block("minecraft:void_air"));
        assert!(!is_air_block("minecraft:stone"));
        assert!(!is_air_block("minecraft:short_grass"));
        assert_eq!(QUARRY_REPLACE, "minecraft:dirt");
        assert_eq!(QUARRY_TOOL, "minecraft:netherite_pickaxe");
        assert_eq!(FORTUNE_LEVEL, 3);
        assert_eq!(DONT_REMOVE_ME, "rftoolsbuilder:dontremoveme");
        assert_eq!(BUILDER_CELLS, 1);
    }

    // --- chunk loading: the force-load branch of `BuilderTileEntity.chunkLoad` ---

    #[test]
    fn the_chunk_of_a_cell_is_a_java_arithmetic_shift() {
        assert_eq!(chunk_of([31, 100, -1]), [1, -1]);
        assert_eq!(chunk_of([32, 100, 0]), [2, 0]);
        assert_eq!(chunk_of([-1, 100, -17]), [-1, -2]);
        assert_eq!(chunk_of([200, 100, 176]), [12, 11]);
    }

    #[test]
    fn a_wait_at_an_unloaded_cell_forces_its_chunk_and_releases_the_old_one() {
        assert_eq!(
            ticket_step(Some([1, 2]), [40, 100, 33]),
            TicketPlan {
                release: Some([1, 2]),
                force: Some([2, 2]),
            }
        );
        assert_eq!(
            ticket_step(None, [40, 100, 33]),
            TicketPlan {
                release: None,
                force: Some([2, 2]),
            }
        );
    }

    #[test]
    fn waiting_again_in_the_same_chunk_does_not_repeat_the_ticket() {
        assert_eq!(
            ticket_step(Some([2, 2]), [47, 100, 36]),
            TicketPlan::default()
        );
    }

    #[test]
    fn finishing_the_pass_releases_the_held_chunk() {
        assert_eq!(
            ticket_release(Some([2, 2])),
            TicketPlan {
                release: Some([2, 2]),
                force: None,
            }
        );
        assert_eq!(ticket_release(None), TicketPlan::default());
    }

    #[test]
    fn a_builders_ticket_owner_is_stable_and_position_specific() {
        assert_eq!(ticket_owner([20, 102, 1]), ticket_owner([20, 102, 1]));
        assert_ne!(ticket_owner([20, 102, 1]), ticket_owner([21, 102, 1]));
        assert_ne!(ticket_owner([20, 102, 1]), ticket_owner([20, 102, 2]));
        assert_ne!(ticket_owner([-1, 102, 1]), ticket_owner([1, 102, 1]));
        assert_ne!(ticket_owner([20, 102, 1]), 0);
    }

    #[test]
    fn a_forced_readout_names_only_the_chunks_a_builder_holds() {
        assert_eq!(forced_line([1, 2, 3], &[]), "Forced chunks at 1 2 3: none");
        assert_eq!(
            forced_line([1, 2, 3], &[[12, 11], [13, 11]]),
            "Forced chunks at 1 2 3: [12,11], [13,11]"
        );
    }

    // --- Slice 6: energy (the numbers are read from the jar, never guessed) ---

    #[test]
    fn the_jars_energy_constants_are_read_not_guessed() {
        assert_eq!(ENERGY_CAPACITY, 1_000_000);
        assert_eq!(ENERGY_MAX_INSERT, 20_000);
        assert_eq!(RF_PER_OPERATION, 500);
        assert_eq!(RF_PER_QUARRY, 300);
        assert_eq!(RF_PER_SKIPPED, 50);
        assert_eq!(INFUSION_SPEED_FACTOR, 20);
        assert_eq!(VOID_RF_FACTOR, 0.5);
    }

    #[test]
    fn a_void_card_costs_half_of_builder_rf_per_operation() {
        assert_eq!(cell_cost(CellCharge::Build), 500);
        assert_eq!(cell_cost(CellCharge::Quarry), 300);
        assert_eq!(cell_cost(CellCharge::Void), 250);
        // The jar's config comment attaches VOID_RF_FACTOR to "RF per operation"
        // (500), not to builderRfPerQuarry (300), and its (int) cast of 0.5 would
        // truncate a nonzero cost to 0 — both artefacts are recorded in notes.md.
        assert_ne!(cell_cost(CellCharge::Void), 150);
        assert_ne!(cell_cost(CellCharge::Void), 0);
        assert_eq!(cell_cost(CellCharge::Skipped), 50);
    }

    #[test]
    fn twenty_seven_quarry_cells_cost_exactly_eight_thousand_one_hundred_fe() {
        assert_eq!(27 * cell_cost(CellCharge::Quarry), 8100);
        assert_eq!(27 * cell_cost(CellCharge::Void), 6750);
    }

    #[test]
    fn an_uninfused_builder_pays_the_full_cell_cost() {
        assert_eq!(INFUSION_FACTOR, 0.0);
        assert_eq!(infusion_multiplier(0.0), 1.0);
        assert_eq!(infusion_multiplier(3.0), 0.0);
        assert_eq!(
            adjusted_cost(cell_cost(CellCharge::Quarry), INFUSION_FACTOR),
            300
        );
    }

    #[test]
    fn the_energy_readout_prints_the_buffer_and_what_was_spent() {
        let status = EnergyStatus {
            at: [22, 102, -7],
            stored: 991_900,
            spent: 8_100,
            filter_skipped: 0,
        };
        assert_eq!(
            energy_line(&status),
            "Builder at 22 102 -7: energy 991900/1000000 FE, spent 8100 FE, build 500 FE/cell, \
             quarry 300 FE/cell, void 250 FE/cell, skip 50 FE/cell, infusion 0, filter skipped 0 FE"
        );
    }

    // --- Slice 7: the filter module and filtered placement ---

    #[test]
    fn an_allow_filter_passes_only_the_configured_block() {
        let filter = Filter {
            mode: FilterMode::Allow,
            value: "minecraft:stone".to_owned(),
        };
        assert!(filter_match(&filter, "minecraft:stone", |_, _| false));
        assert!(!filter_match(&filter, "minecraft:dirt", |_, _| false));
        assert!(!filter_match(&filter, "minecraft:cobblestone", |_, _| {
            false
        }));
    }

    #[test]
    fn a_deny_filter_skips_only_the_configured_block() {
        let filter = Filter {
            mode: FilterMode::Deny,
            value: "minecraft:dirt".to_owned(),
        };
        assert!(!filter_match(&filter, "minecraft:dirt", |_, _| false));
        assert!(filter_match(&filter, "minecraft:stone", |_, _| false));
    }

    #[test]
    fn a_tag_filter_matches_only_blocks_in_the_tag() {
        let filter = Filter {
            mode: FilterMode::Allow,
            value: "#c:stones".to_owned(),
        };
        let in_stones = |block: &str, tag: &str| {
            tag == "c:stones" && matches!(block, "minecraft:stone" | "minecraft:cobblestone")
        };
        assert!(filter_match(&filter, "minecraft:stone", in_stones));
        assert!(filter_match(&filter, "minecraft:cobblestone", in_stones));
        assert!(!filter_match(&filter, "minecraft:dirt", in_stones));
        // A deny filter over the same tag rejects its members and keeps the rest.
        let deny = Filter {
            mode: FilterMode::Deny,
            value: "#c:stones".to_owned(),
        };
        assert!(!filter_match(&deny, "minecraft:stone", in_stones));
        assert!(filter_match(&deny, "minecraft:dirt", in_stones));
    }

    #[test]
    fn an_empty_filter_passes_every_block_in_both_modes() {
        for mode in [FilterMode::Allow, FilterMode::Deny] {
            let filter = Filter {
                mode,
                value: String::new(),
            };
            assert!(filter.is_empty());
            assert!(filter_match(&filter, "minecraft:stone", |_, _| false));
            assert!(filter_match(&filter, "minecraft:dirt", |_, _| false));
        }
    }

    #[test]
    fn a_filtered_out_cell_costs_the_jars_builder_rf_per_skipped() {
        assert_eq!(filter_skip_cost(), 50);
        assert_eq!(filter_skip_cost(), cell_cost(CellCharge::Skipped));
    }

    // ---- slice 9: the pump/liquid cards and the Builder's fluid tank ----

    #[test]
    fn the_pump_and_liquid_cards_are_the_jars_eight_and_ninth_card_types() {
        assert_eq!(BUCKET_MB, 1000); // checkFluidTank SIMULATE == 1000 / findAndConsumeLiquid 1000
        assert_eq!(RF_PER_LIQUID, 300); // BuilderConfiguration.builderRfPerLiquid
        assert_eq!(FLUID_TANK_CAPACITY, 64_000);
        assert_eq!(Pump::PUMP.suffix(), "pump");
        assert_eq!(Pump::PUMP_CLEAR.suffix(), "pump_clear");
        assert_eq!(Pump::PUMP.description(), "Pump");
        assert_eq!(Pump::PUMP_CLEAR.description(), "Pump");
        assert_eq!(CardKind::Liquid.suffix(), "liquid");
        assert_eq!(CardKind::Liquid.description(), "Place liquids");
        assert_eq!(CardKind::parse("pump"), Some(CardKind::Pump(Pump::PUMP)));
        assert_eq!(
            CardKind::parse("pump_clear"),
            Some(CardKind::Pump(Pump::PUMP_CLEAR))
        );
        assert_eq!(CardKind::parse("liquid"), Some(CardKind::Liquid));
        assert_eq!(fluid_cost(), 300);
    }

    #[test]
    fn twenty_seven_source_cells_fill_the_tank_with_exactly_one_bucket_each() {
        let mut amount = 0u32;
        let mut fluid = None;
        let mut pumped = 0u32;
        for _ in 0..27 {
            let facts = FluidFacts {
                near_builder: false,
                source: Some("minecraft:water"),
                destroy_speed: 1.0,
                replaceable: false,
                tank_fluid: fluid,
                tank_amount: amount,
            };
            assert_eq!(
                fluid_step(CardKind::Pump(Pump::PUMP), facts),
                FluidStep::PumpIn
            );
            // The glue writes the pumped fluid's id into the tank with the amount.
            (fluid, amount) = tank_after(FluidStep::PumpIn, Some("minecraft:water"), amount);
            pumped += 1;
        }
        assert_eq!(pumped, 27);
        assert_eq!(fluid, Some("minecraft:water"));
        assert_eq!(amount, 27_000); // the artefact's exact millibuckets for 27 source cells
    }

    #[test]
    fn a_pump_takes_a_whole_bucket_or_nothing_and_waits_when_the_tank_is_full() {
        let facts = FluidFacts {
            near_builder: false,
            source: Some("minecraft:water"),
            destroy_speed: 1.0,
            replaceable: false,
            tank_fluid: Some("minecraft:water"),
            tank_amount: 64_000 - 500,
        };
        assert_eq!(
            fluid_step(CardKind::Pump(Pump::PUMP), facts),
            FluidStep::Wait(FluidWait::NoRoom)
        );
        assert_eq!(
            tank_after(
                FluidStep::Wait(FluidWait::NoRoom),
                facts.tank_fluid,
                facts.tank_amount
            )
            .1,
            63_500
        );
        // A different fluid cannot go into a tank that already holds one.
        let other = FluidFacts {
            tank_fluid: Some("minecraft:lava"),
            ..facts
        };
        assert_eq!(
            fluid_step(CardKind::Pump(Pump::PUMP), other),
            FluidStep::Wait(FluidWait::NoRoom)
        );
        // The jar's early guards: own box and a non-source (flowing fluid or air).
        for skip in [
            FluidFacts {
                near_builder: true,
                ..facts
            },
            FluidFacts {
                source: None,
                ..facts
            },
        ] {
            assert!(matches!(
                fluid_step(CardKind::Pump(Pump::PUMP), skip),
                FluidStep::Skip(_)
            ));
        }
        // The jar's destroy-speed guard screens a block that cannot be broken.
        assert_eq!(
            fluid_step(
                CardKind::Pump(Pump::PUMP),
                FluidFacts {
                    destroy_speed: -1.0,
                    ..facts
                }
            ),
            FluidStep::Skip(FluidSkip::Unbreakable)
        );
    }

    #[test]
    fn a_liquid_card_spends_exactly_one_bucket_per_cell_and_waits_when_empty() {
        let facts = FluidFacts {
            near_builder: false,
            source: None,
            destroy_speed: 1.0,
            replaceable: true,
            tank_fluid: Some("minecraft:water"),
            tank_amount: 27_000,
        };
        let mut amount = facts.tank_amount;
        let mut placed = 0u32;
        for _ in 0..27 {
            assert_eq!(fluid_step(CardKind::Liquid, facts), FluidStep::Place);
            (_, amount) = tank_after(FluidStep::Place, facts.tank_fluid, amount);
            placed += 1;
        }
        assert_eq!(placed, 27);
        assert_eq!(amount, 0);
        assert_eq!(
            fluid_step(
                CardKind::Liquid,
                FluidFacts {
                    tank_amount: 0,
                    ..facts
                }
            ),
            FluidStep::Wait(FluidWait::NoLiquid)
        );
        assert!(matches!(
            fluid_step(
                CardKind::Liquid,
                FluidFacts {
                    replaceable: false,
                    ..facts
                }
            ),
            FluidStep::Skip(FluidSkip::NotReplaceable)
        ));
    }

    #[test]
    fn the_fluid_readout_names_the_fluid_and_exact_millibuckets() {
        assert_eq!(
            fluid_line(&FluidStatus {
                at: [42, 102, -18],
                fluid: Some("minecraft:water".to_owned()),
                amount: 27_000,
            }),
            "Builder at 42 102 -18: fluid minecraft:water 27000 mB (capacity 64000 mB), bucket 1000 mB, 300 FE/cell"
        );
        assert_eq!(
            fluid_line(&FluidStatus {
                at: [0, 0, 0],
                fluid: None,
                amount: 0,
            }),
            "Builder at 0 0 0: fluid none 0 mB (capacity 64000 mB), bucket 1000 mB, 300 FE/cell"
        );
    }
}
