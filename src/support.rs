//! The game glue of the Builder: its block entity, the card slot, the card's
//! item Part, the tick that places cells, and the readout commands.
//!
//! The rules — the nine shape formulas, the solid/hollow cells, the clamps, the
//! walk and the readout text — live in [`crate::runtime`], which has no
//! `mod_api` types. This module only reads the game, calls the runtime and
//! writes the result back.
//!
//! A Builder's state is its block entity's saved compound (`cursor`, `placed`,
//! `skipped`, `done`, `refusal`) and its card is the container's one slot, so
//! both survive a relaunch and a broken Builder drops its card with the block.

use std::sync::OnceLock;

use mod_api::{
    self, BlockEntityEvent, BlockEntityPart, BlockEntityRef, BlockEntityTickEvent,
    BlockEntityTicksEvent, BlockKey, BlockPart, BlockState, BlockUpdateFlags, Capability, Command,
    CommandArgumentValue, Component, Compound, ENERGY, Flow, Hand, Host, HostBlockStates, HostSide,
    HostTransaction, ITEMS, Identifier, InteractionResult, ItemPart, ItemResource, ItemStack,
    LevelRef, LevelSide, ModContainer, ModEnergyBuffer, ModFluidTanks, PlayerRef,
    RegisterCommandsEvent, Registrar, RegistrationContext, ResourceHandler, RightClickBlockEvent,
    RunModCommandEvent, ServerHost, SoundSource, Tag, Text, TicketController, TicketOwner,
    UseOnEvent,
};

use crate::runtime::{
    self, BLOCKS_PER_TICK, BUILDER_CELLS, Card, CardKind, CellCharge, CellFacts, DONT_REMOVE_ME,
    ENERGY_CAPACITY, ENERGY_MAX_INSERT, EnergyStatus, FLUID_TANK_CAPACITY, FORTUNE_LEVEL, Filter,
    FilterMode, FluidFacts, FluidStatus, FluidStep, FluidWait, INFUSION_FACTOR, LOOKUPS_PER_TICK,
    Progress, QUARRY_REPLACE, QUARRY_TOOL, Quarry, QuarryStep, Refusal, RefusalReason, Scan, Shape,
    Status, Voiding, adjusted_cost, cell_cost, energy_line, filter_match, filter_skip_cost,
    fluid_cost, fluid_line, fluid_step, tank_after,
};

/// The compound key type, taken from `Tag::String`'s own payload: the API does not re-export the
/// UTF-16 key type and a path dependency on `horizon-core` is refused by the launcher's build
/// (it stages only the API crate), so the same conversion a mod's NBT string uses is written here.
macro_rules! key {
    ($name:expr) => {{
        let mod_api::Tag::String(key) = mod_api::Tag::String(($name).into()) else {
            unreachable!("a string tag always holds a compound key");
        };
        key
    }};
}

/// The block this slice implements, exactly as the content declares it.
const BLOCK: &str = runtime::BUILDER;
/// The block-entity type this mod declares for that block.
const BE_TYPE: &str = "rftoolsbuilder:builder_block_entity";
/// The one shape card this slice carries.
const CARD_ITEM: &str = runtime::SHAPE_CARD;
/// The item component that carries a card's shape (`component_types`).
pub(crate) const COMPONENT: &str = runtime::COMPONENT;
/// The card slot of the Builder's container (`BuilderTileEntity.SLOT_TAB`).
const SLOT_CARD: usize = 0;
/// The Builder's filter slot (`BuilderTileEntity.SLOT_FILTER`). The jar carries a separate
/// filter-module item here; this port's filter rides the card's component (recorded deviation),
/// but the slot is declared so the container has the jar's two slots.
const SLOT_FILTER: usize = 1;
/// The container's slot count: the card and the filter slot.
const CARD_SLOTS: usize = SLOT_FILTER + 1;

// --- card component keys (upstream's `ShapeCardData` field names) ----------
/// `ShapeCardData.shape`: the jar's shape enum name.
const K_SHAPE: &str = "shape";
/// `ShapeCardData.solid`: solid or hollow.
const K_SOLID: &str = "solid";
/// `ShapeCardDimensions.dimension`: the box the shape is fitted to.
const K_DIMENSION: &str = "dimension";
/// `ShapeCardDimensions.offset`: where the shape's centre sits.
const K_OFFSET: &str = "offset";
/// `ShapeModifierData.rotation`: the turn.
const K_ROTATION: &str = "rotation";
/// `ShapeModifierData.flipY`: the mirror.
const K_FLIP_Y: &str = "flipY";
/// `ShapeCardDimensions.mode`: the corner-selection step.
const K_MODE: &str = "mode";
/// `ShapeCardDimensions.selected`: the Builder the card is being authored for.
const K_SELECTED: &str = "selected";
/// The dimension id that goes with `selected`.
const K_SELECTED_DIMENSION: &str = "selectedDimension";
/// `ShapeCardDimensions.corner1`: the first corner picked.
const K_CORNER1: &str = "corner1";
/// The card's type (`ShapeCardType`): the jar reads it from the item id, and this port
/// carries one card item, so it rides the component. No upstream data field exists for it.
const K_TYPE: &str = "type";
/// `ShapeCardData.tagMatching`: expand the void list through each material's block tag.
const K_TAG_MATCHING: &str = "tagMatching";
/// `ShapeCardData.voiding`: the material keys the card's quarry collects nothing for.
const K_VOIDING: &str = "voiding";
/// The filter's configured value (a block id or a `#tag`) — `/rftoolsbuilder filter`.
const K_FILTER_VALUE: &str = "filterValue";
/// The filter's mode (`blacklist`): `allow` or `deny`.
const K_FILTER_MODE: &str = "filterMode";

/// The game's block-entity tank save: `fluids` is a list of `{Fluid, Amount}`
/// compounds, one per declared tank (`NativeFluidTank` read/writer at this pin).
const K_FLUIDS: &str = "fluids";
const K_FLUID_ID: &str = "Fluid";
const K_FLUID_AMOUNT: &str = "Amount";
const EMPTY_FLUID: &str = "minecraft:empty";

// --- block-entity keys -----------------------------------------------------
/// The walking cursor; absent while there is no pass to resume.
const K_CURSOR: &str = "cursor";
/// Cells a block was placed into.
const K_PLACED: &str = "placed";
/// Shape cells looked at that needed nothing.
const K_SKIPPED: &str = "skipped";
/// The pass finished.
const K_DONE: &str = "done";
/// Why the scan stopped (`RefusalReason` as a byte).
const K_REFUSAL: &str = "refusal";
/// Where it stopped.
const K_REFUSAL_POS: &str = "refusalPos";
/// The card the current pass belongs to (its shape, size, offset and turn):
/// a different card starts a new pass.
const K_CARD: &str = "card";
/// The pass's cell count, cached when the pass starts so a repeated readout costs nothing.
const K_CELLS: &str = "cells";
/// Whether the shape holds more cells than the cached count.
const K_CELLS_MORE: &str = "cellsMore";
/// The installed card's item id, in the block entity's saved data. The card lives here (not only in
/// the container slot): a click-time slot write did not reach the next command's read at this pin,
/// so the saved data is the one place every path agrees on.
const K_CARD_ITEM: &str = "cardItem";
/// The installed card's data (its shape, size, offset, turn and corner selection).
const K_CARD_DATA: &str = "cardData";
/// The item id the Builder last mirrored into its container slot: a slot holding anything else was
/// changed from outside and wins.
const K_CARD_SHOWN: &str = "cardShown";
/// `BuilderConfiguration.quarryReplace`: what this Builder's quarry leaves behind, as a block
/// id. The jar keeps it in a server config; the port keeps it per Builder (deviation,
/// `notes.md`), defaulting to the jar's own value.
const K_REPLACE: &str = "quarryReplace";
/// Quarry cells whose block was removed without its drops being collected.
const K_VOIDED: &str = "voided";
/// The chunk `[x, z]` a Builder currently force-loads with its ticket (the jar's
/// `forcedChunk` field), so the ticket can be released when the pass ends.
const K_FORCED_CHUNK: &str = "forcedChunk";
/// The Builder's FE, as the jar's `GenericEnergyStorage` writes it: an int `energy` in the
/// block entity's saved compound, which the `ModEnergyBuffer` capability also reads.
const K_ENERGY: &str = "energy";
/// FE this Builder's passes have spent, so `/rftoolsbuilder energy` can report exact consumption.
const K_ENERGY_SPENT: &str = "energySpent";
/// FE spent specifically on filtered-out cells (`builderRfPerSkipped`).
const K_FILTER_SPENT: &str = "filterSpent";

/// The card's refusal codes in the saved compound.
const REFUSAL_NONE: i8 = 0;
/// `RefusalReason::NoMaterial`.
const REFUSAL_NO_MATERIAL: i8 = 1;
/// `RefusalReason::OutOfWorld`.
const REFUSAL_OUT_OF_WORLD: i8 = 2;
/// `RefusalReason::Protected`.
const REFUSAL_PROTECTED: i8 = 3;
/// `RefusalReason::NoRoom`.
const REFUSAL_NO_ROOM: i8 = 4;
/// `RefusalReason::Unloaded`.
const REFUSAL_UNLOADED: i8 = 5;
/// `RefusalReason::NoPower`.
const REFUSAL_NO_POWER: i8 = 6;
const REFUSAL_NO_LIQUID: i8 = 7;

/// The block key the hooks compare the loaded block against. A `BlockKey` only
/// names itself through a host, so it is asked for once, at registration.
static BLOCK_KEY: OnceLock<BlockKey> = OnceLock::new();

fn block_key() -> BlockKey {
    *BLOCK_KEY
        .get()
        .expect("registration runs before every hook")
}

/// The ticket controller the Builder force-loads quarry chunks with, registered
/// once at startup (`RegisterTicketControllersEvent`) and `Copy`, so a `OnceLock`
/// holds it for every later tick (the jar's `ModSetup.ticketController`).
static TICKET_CONTROLLER: OnceLock<TicketController> = OnceLock::new();

fn ticket_controller() -> TicketController {
    *TICKET_CONTROLLER
        .get()
        .expect("registration runs before every hook")
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Declare the block-entity type, attach its Part, declare the Builder's card
/// slot and attach the card item's Part.
pub(crate) fn register(registrar: &mut dyn Registrar, _cx: &RegistrationContext) {
    let block = registrar.block_key(BLOCK);
    let _ = BLOCK_KEY.set(block);
    // The jar's `RegisterTicketControllersEvent`: register once and keep the handle for
    // `chunk.set_mod_forced`. The id is `rftoolsbuilder:quarry` (the SDK namespaces it).
    let controller = registrar.register_ticket_controller("quarry", true);
    let _ = TICKET_CONTROLLER.set(controller);
    let kind = registrar
        .block_entity_type(BE_TYPE)
        .blocks(&[BLOCK])
        .build();
    registrar
        .block_entity_part(&kind, Builder)
        .expect("one Part per block-entity type");
    // The card is both put in and taken out by hand, as upstream's card slot
    // is `in().out()`.
    let slots: Vec<usize> = vec![SLOT_CARD, SLOT_FILTER];
    // `..Default::default()` keeps a field a later game build adds from breaking
    // this literal (`cargo horizon-mod check` asks for it).
    #[expect(
        clippy::needless_update,
        reason = "the update is for a field a later game build may add"
    )]
    let container = ModContainer {
        slots: CARD_SLOTS,
        faces: std::array::from_fn(|_| slots.clone()),
        inputs: slots.clone(),
        outputs: slots,
        menu: None,
        ..ModContainer::default()
    };
    registrar.block_container(&block, container);
    // The jar's `GenericEnergyStorage(1_000_000, 20_000)`: a buffer that takes FE from any
    // side but gives none out (the Builder has no energy output). Its amount lives in the
    // saved compound's `energy` int, so a reload keeps it and the tick reads it there (the
    // capability is deliberately unavailable to a block entity during its own tick).
    #[expect(
        clippy::needless_update,
        reason = "the update is for a field a later game build may add"
    )]
    let battery = ModEnergyBuffer {
        capacity: ENERGY_CAPACITY,
        max_insert: ENERGY_MAX_INSERT,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
        ..ModEnergyBuffer::default()
    };
    registrar.block_energy(&block, battery);
    // The jar pumps fluid into (and places it from) a tank above or below the Builder. This
    // package has no tank block, so the Builder declares one 64-bucket tank as the stand-in;
    // its contents live in the saved compound under `fluids` and the BE tick reads them there.
    let mut tank = ModFluidTanks::default();
    tank.tanks = vec![i32::try_from(FLUID_TANK_CAPACITY).expect("64,000 mB fits an int")];
    tank.faces = std::array::from_fn(|_| vec![0]);
    tank.inputs = vec![0];
    tank.outputs = vec![0];
    registrar.block_fluids(&block, tank);
    let card = registrar.item_key(CARD_ITEM);
    registrar.item_part(card, ShapeCardItem);
}

/// The Builder's Part: it ticks, and everything else is per-tick work.
struct Builder;

#[mod_api::horizon_part]
impl BlockPart for Builder {}

#[mod_api::horizon_part]
impl BlockEntityPart for Builder {
    /// The Builder gets a block entity with empty saved data; `Items` is the
    /// container's and the scan state rides the compound.
    fn block_entity(
        &self,
        _level: LevelSide<'_>,
        _event: &mut BlockEntityEvent<'_>,
    ) -> Option<Compound> {
        Some(Compound::new())
    }

    /// The Builder ticks on the server (`EntityBlock.getTicker`).
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: walk the card's shape and place what fits.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        tick(host, event);
    }
}

// ---------------------------------------------------------------------------
// The card's component
// ---------------------------------------------------------------------------

/// The `rftoolsbuilder:shape_card` component: a namespaced NBT component whose
/// fields mirror upstream's `ShapeCardData` (`shape`, `solid`, `dimension`,
/// `offset`, `rotation`, `flipY`). A stack without it is the jar's default card.
struct CardTag;

impl Component for CardTag {
    type Value = Tag;

    fn id(&self) -> &str {
        COMPONENT
    }

    /// Only a compound is a card; anything else reads as the default card.
    fn decode(&self, value: &Tag) -> Option<Tag> {
        matches!(value, Tag::Compound(_)).then(|| value.clone())
    }

    fn encode(&self, value: Tag) -> Tag {
        value
    }
}

/// What a card stack carries: the shape and the corner-selection step.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct CardData {
    card: Card,
    selection: Selection,
    /// The filter the jar keeps in the filter module's data component (here on the card).
    filter: Filter,
}

/// The corner-selection state upstream keeps in `ShapeCardDimensions`
/// (`mode`, `corner1`, `selected`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Selection {
    /// 0 none, 1 the first corner next, 2 the second.
    mode: u8,
    /// The Builder the card was bound to, and its dimension.
    target: Option<(String, [i32; 3])>,
    /// The first corner picked.
    corner1: Option<[i32; 3]>,
}

impl CardData {
    /// The data a stack carries; a stack with no (or a broken) component is the
    /// jar's default card, as `getOrDefault` reads it.
    fn read(stack: &ItemStack) -> Self {
        let Some(Tag::Compound(data)) = stack.get(&CardTag) else {
            return Self::default();
        };
        Self {
            card: card_of(&data),
            selection: selection_of(&data),
            filter: filter_of(&data),
        }
    }

    /// Writes the data into the stack's component patch; whether the stack took
    /// it.
    fn write(&self, stack: &mut ItemStack) -> bool {
        let mut data = Compound::new();
        put_card(&mut data, self.card);
        put_selection(&mut data, &self.selection);
        put_filter(&mut data, &self.filter);
        stack.set(&CardTag, Tag::Compound(data)).is_ok()
    }
}

/// The card a data compound holds, each field falling back to the jar's
/// default on its own (`ShapeCardData.DEFAULT`).
fn card_of(data: &Compound) -> Card {
    let default = Card::default();
    Card {
        shape: string(data, K_SHAPE)
            .and_then(|name| Shape::parse(&name))
            .unwrap_or(default.shape),
        solid: data
            .get(&key!(K_SOLID))
            .and_then(|tag| match tag {
                Tag::Byte(flag) => Some(*flag != 0),
                _ => None,
            })
            .unwrap_or(default.solid),
        dimension: ints(data, K_DIMENSION).unwrap_or(default.dimension),
        offset: ints(data, K_OFFSET).unwrap_or(default.offset),
        modifier: runtime::Modifier {
            rotation: string(data, K_ROTATION)
                .and_then(|code| runtime::Rotation::parse(&code))
                .unwrap_or(default.modifier.rotation),
            flip_y: data
                .get(&key!(K_FLIP_Y))
                .and_then(|tag| match tag {
                    Tag::Byte(flag) => Some(*flag != 0),
                    _ => None,
                })
                .unwrap_or(default.modifier.flip_y),
        },
        // A card with no type is the jar's `shape_card_def` (`CARD_SHAPE`); a type this
        // slice does not carry (`void`, `pump`, …) reads as a shape card rather than
        // silently growing a behaviour.
        kind: string(data, K_TYPE)
            .and_then(|name| CardKind::parse(&name))
            .unwrap_or(default.kind),
        voiding: match data.get(&key!(K_VOIDING)) {
            Some(Tag::List(values)) => {
                Voiding::from_names(values.iter().filter_map(|tag| match tag {
                    Tag::String(text) => Some(text.to_string_lossy()),
                    _ => None,
                }))
            }
            _ => default.voiding,
        },
        tag_matching: data
            .get(&key!(K_TAG_MATCHING))
            .and_then(|tag| match tag {
                Tag::Byte(flag) => Some(*flag != 0),
                _ => None,
            })
            .unwrap_or(default.tag_matching),
    }
}

/// Writes a card into a data compound.
fn put_card(data: &mut Compound, card: Card) {
    data.insert(key!(K_SHAPE), Tag::String(card.shape.id().into()));
    data.insert(key!(K_SOLID), Tag::Byte(i8::from(card.solid)));
    put_ints(data, K_DIMENSION, card.dimension);
    put_ints(data, K_OFFSET, card.offset);
    data.insert(
        key!(K_ROTATION),
        Tag::String(card.modifier.rotation.id().into()),
    );
    data.insert(key!(K_FLIP_Y), Tag::Byte(i8::from(card.modifier.flip_y)));
    data.insert(key!(K_TYPE), Tag::String(card.kind.suffix().into()));
    data.insert(key!(K_TAG_MATCHING), Tag::Byte(i8::from(card.tag_matching)));
    let voiding: Vec<Tag> = card
        .voiding
        .iter()
        .map(|name| Tag::String(name.into()))
        .collect();
    data.insert(key!(K_VOIDING), Tag::List(voiding));
}

/// The filter a card's data compound carries: the `blacklist` mode and the single configured
/// value (a block id or a `#tag`). Absent fields are the jar's "no filter in slot 1".
fn filter_of(data: &Compound) -> Filter {
    Filter {
        mode: string(data, K_FILTER_MODE)
            .and_then(|mode| FilterMode::parse(&mode))
            .unwrap_or_default(),
        value: string(data, K_FILTER_VALUE).unwrap_or_default(),
    }
}

/// Write a filter's fields into a card's data compound.
fn put_filter(data: &mut Compound, filter: &Filter) {
    data.insert(
        key!(K_FILTER_VALUE),
        Tag::String(filter.value.as_str().into()),
    );
    data.insert(
        key!(K_FILTER_MODE),
        Tag::String(filter.mode.as_str().into()),
    );
}

/// The card in the Builder's slot, as its saved compound holds it: the item id and the card's own
/// data, which is everything the stack's component carried.
fn saved_card(data: &Compound) -> Option<(String, CardData)> {
    let item = string(data, K_CARD_ITEM)?;
    let Some(Tag::Compound(card)) = data.get(&key!(K_CARD_DATA)) else {
        return None;
    };
    Some((
        item,
        CardData {
            card: card_of(card),
            selection: selection_of(card),
            filter: filter_of(card),
        },
    ))
}

/// Write the card slot into the saved compound, or empty it with `None`.
fn write_saved_card(data: &mut Compound, card: Option<(&str, &CardData)>) {
    match card {
        Some((item, payload)) => {
            data.insert(key!(K_CARD_ITEM), Tag::String(item.into()));
            let mut fields = Compound::new();
            put_card(&mut fields, payload.card);
            put_selection(&mut fields, &payload.selection);
            put_filter(&mut fields, &payload.filter);
            data.insert(key!(K_CARD_DATA), Tag::Compound(fields));
        }
        None => {
            data.remove(&key!(K_CARD_ITEM));
            data.remove(&key!(K_CARD_DATA));
        }
    }
}

/// The stack a saved card makes: the item with the card's data set as its component. `None` when
/// the id is not an item of this world or the stack refuses the component.
fn card_stack(host: &dyn Host, card: &(String, CardData)) -> Option<ItemStack> {
    let key = host.item_key(&Identifier::parse_namespaced(&card.0)?)?;
    let mut stack = ItemStack::of(key);
    card.1.write(&mut stack).then_some(stack)
}

/// The selection a data compound holds.
fn selection_of(data: &Compound) -> Selection {
    let target = match (string(data, K_SELECTED_DIMENSION), ints(data, K_SELECTED)) {
        (Some(dimension), Some(pos)) => Some((dimension, pos)),
        _ => None,
    };
    Selection {
        mode: data
            .get(&key!(K_MODE))
            .and_then(|tag| match tag {
                Tag::Byte(mode) => u8::try_from(*mode).ok(),
                _ => None,
            })
            .unwrap_or(0)
            .min(2),
        target,
        corner1: ints(data, K_CORNER1),
    }
}

/// Writes a selection into a data compound, dropping the keys it does not use
/// (`replace_data`'s job for a block entity; a component compound is ours).
fn put_selection(data: &mut Compound, selection: &Selection) {
    data.insert(
        key!(K_MODE),
        Tag::Byte(i8::try_from(selection.mode).unwrap_or(0)),
    );
    match &selection.target {
        Some((dimension, pos)) => {
            data.insert(
                key!(K_SELECTED_DIMENSION),
                Tag::String(dimension.as_str().into()),
            );
            put_ints(data, K_SELECTED, *pos);
        }
        None => {
            data.remove(&key!(K_SELECTED_DIMENSION));
            data.remove(&key!(K_SELECTED));
        }
    }
    match selection.corner1 {
        Some(pos) => put_ints(data, K_CORNER1, pos),
        None => {
            data.remove(&key!(K_CORNER1));
        }
    }
}

/// A compound's string field.
fn string(data: &Compound, key: &str) -> Option<String> {
    match data.get(&key!(key)) {
        Some(Tag::String(text)) => Some(text.to_string_lossy()),
        _ => None,
    }
}

/// A compound's three-integer field, as upstream's `BlockPos.CODEC` writes it
/// (a list of three ints; an int array is accepted too).
fn ints(data: &Compound, key: &str) -> Option<[i32; 3]> {
    let read = |tag: &Tag| match tag {
        Tag::Int(value) => Some(*value),
        _ => None,
    };
    match data.get(&key!(key)) {
        Some(Tag::List(values)) if values.len() == 3 => {
            Some([read(&values[0])?, read(&values[1])?, read(&values[2])?])
        }
        Some(Tag::IntArray(values)) if values.len() == 3 => Some([values[0], values[1], values[2]]),
        _ => None,
    }
}

/// Writes a three-integer field.
fn put_ints(data: &mut Compound, key: &str, value: [i32; 3]) {
    data.insert(
        key!(key),
        Tag::List(vec![
            Tag::Int(value[0]),
            Tag::Int(value[1]),
            Tag::Int(value[2]),
        ]),
    );
}

/// Reads a two-integer field (a chunk's `[x, z]`).
fn ints2(nbt: &Compound, key: &str) -> Option<[i32; 2]> {
    let as_int = |tag: &Tag| match tag {
        Tag::Int(value) => Some(*value),
        _ => None,
    };
    match nbt.get(&key!(key)) {
        Some(Tag::List(values)) if values.len() == 2 => {
            Some([as_int(&values[0])?, as_int(&values[1])?])
        }
        Some(Tag::IntArray(values)) if values.len() == 2 => Some([values[0], values[1]]),
        _ => None,
    }
}

/// Writes a two-integer field.
fn put_ints2(data: &mut Compound, key: &str, value: [i32; 2]) {
    data.insert(
        key!(key),
        Tag::List(vec![Tag::Int(value[0]), Tag::Int(value[1])]),
    );
}

// ---------------------------------------------------------------------------
// A Builder's saved state
// ---------------------------------------------------------------------------

/// A Builder's scan state, kept in its saved compound.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct State {
    /// The card this pass belongs to; a different one starts a fresh pass.
    card: Option<String>,
    /// Where the walk looks next.
    cursor: Option<[i32; 3]>,
    /// How far the pass has got.
    progress: Progress,
    /// Whether the pass finished.
    done: bool,
    /// Where and why it stopped.
    refusal: Option<Refusal>,
    /// The pass's cell count, cached when the pass started.
    cells: Option<runtime::CellCount>,
    /// The chunk currently held by the Builder's force-load ticket.
    forced_chunk: Option<[i32; 2]>,
    /// FE this Builder's passes have spent, as the jar's `GenericEnergyStorage` consumption
    /// accumulated (the port keeps a lifetime total for the energy readout).
    energy_spent: u64,
    /// FE spent on filtered-out cells, read by `/rftoolsbuilder energy` as `M × 50`.
    filter_skipped: u64,
}

impl State {
    /// The state a saved compound holds.
    fn read(nbt: &Compound) -> Self {
        let number = |key: &str| match nbt.get(&key!(key)) {
            Some(Tag::Long(value)) => u64::try_from(*value).ok(),
            Some(Tag::Int(value)) => u64::try_from(*value).ok(),
            _ => None,
        };
        let refusal = match nbt.get(&key!(K_REFUSAL)) {
            Some(Tag::Byte(code)) if *code == REFUSAL_NO_MATERIAL => {
                Some(RefusalReason::NoMaterial)
            }
            Some(Tag::Byte(code)) if *code == REFUSAL_OUT_OF_WORLD => {
                Some(RefusalReason::OutOfWorld)
            }
            Some(Tag::Byte(code)) if *code == REFUSAL_PROTECTED => Some(RefusalReason::Protected),
            Some(Tag::Byte(code)) if *code == REFUSAL_NO_ROOM => Some(RefusalReason::NoRoom),
            Some(Tag::Byte(code)) if *code == REFUSAL_UNLOADED => Some(RefusalReason::Unloaded),
            Some(Tag::Byte(code)) if *code == REFUSAL_NO_POWER => Some(RefusalReason::NoPower),
            Some(Tag::Byte(code)) if *code == REFUSAL_NO_LIQUID => Some(RefusalReason::NoLiquid),
            _ => None,
        }
        .map(|reason| Refusal {
            at: ints(nbt, K_REFUSAL_POS).unwrap_or([0, 0, 0]),
            reason,
        });
        Self {
            card: string(nbt, K_CARD),
            cursor: ints(nbt, K_CURSOR),
            progress: Progress {
                placed: number(K_PLACED).unwrap_or(0),
                skipped: number(K_SKIPPED).unwrap_or(0),
                voided: number(K_VOIDED).unwrap_or(0),
            },
            done: matches!(nbt.get(&key!(K_DONE)), Some(Tag::Byte(1))),
            refusal,
            cells: number(K_CELLS).map(|cells| runtime::CellCount {
                cells,
                more: matches!(nbt.get(&key!(K_CELLS_MORE)), Some(Tag::Byte(1))),
            }),
            forced_chunk: ints2(nbt, K_FORCED_CHUNK),
            energy_spent: number(K_ENERGY_SPENT).unwrap_or(0),
            filter_skipped: number(K_FILTER_SPENT).unwrap_or(0),
        }
    }

    /// Writes the state into a block entity's compound, dropping the keys it no
    /// longer uses (a key a `merge_data` cannot delete; this writes the whole
    /// compound the block entity keeps).
    fn write(&self, nbt: &mut Compound) {
        match &self.card {
            Some(card) => {
                nbt.insert(key!(K_CARD), Tag::String(card.as_str().into()));
            }
            None => {
                nbt.remove(&key!(K_CARD));
            }
        }
        match self.cursor {
            Some(cursor) => put_ints(nbt, K_CURSOR, cursor),
            None => {
                nbt.remove(&key!(K_CURSOR));
            }
        }
        nbt.insert(
            key!(K_PLACED),
            Tag::Long(i64::try_from(self.progress.placed).unwrap_or(i64::MAX)),
        );
        nbt.insert(
            key!(K_SKIPPED),
            Tag::Long(i64::try_from(self.progress.skipped).unwrap_or(i64::MAX)),
        );
        nbt.insert(
            key!(K_VOIDED),
            Tag::Long(i64::try_from(self.progress.voided).unwrap_or(i64::MAX)),
        );
        nbt.insert(key!(K_DONE), Tag::Byte(i8::from(self.done)));
        match self.cells {
            Some(cells) => {
                nbt.insert(
                    key!(K_CELLS),
                    Tag::Long(i64::try_from(cells.cells).unwrap_or(i64::MAX)),
                );
                nbt.insert(key!(K_CELLS_MORE), Tag::Byte(i8::from(cells.more)));
            }
            None => {
                nbt.remove(&key!(K_CELLS));
                nbt.remove(&key!(K_CELLS_MORE));
            }
        }
        match self.refusal {
            Some(refusal) => {
                let code = match refusal.reason {
                    RefusalReason::NoMaterial => REFUSAL_NO_MATERIAL,
                    RefusalReason::OutOfWorld => REFUSAL_OUT_OF_WORLD,
                    RefusalReason::Protected => REFUSAL_PROTECTED,
                    RefusalReason::NoRoom => REFUSAL_NO_ROOM,
                    RefusalReason::Unloaded => REFUSAL_UNLOADED,
                    RefusalReason::NoPower => REFUSAL_NO_POWER,
                    RefusalReason::NoLiquid => REFUSAL_NO_LIQUID,
                };
                nbt.insert(key!(K_REFUSAL), Tag::Byte(code));
                put_ints(nbt, K_REFUSAL_POS, refusal.at);
            }
            None => {
                nbt.insert(key!(K_REFUSAL), Tag::Byte(REFUSAL_NONE));
                nbt.remove(&key!(K_REFUSAL_POS));
            }
        }
        match self.forced_chunk {
            Some(chunk) => put_ints2(nbt, K_FORCED_CHUNK, chunk),
            None => {
                nbt.remove(&key!(K_FORCED_CHUNK));
            }
        }
        nbt.insert(
            key!(K_ENERGY_SPENT),
            Tag::Long(i64::try_from(self.energy_spent).unwrap_or(i64::MAX)),
        );
        nbt.insert(
            key!(K_FILTER_SPENT),
            Tag::Long(i64::try_from(self.filter_skipped).unwrap_or(i64::MAX)),
        );
    }
}

/// The Builder's saved FE (0 when the compound has none), clamped to the jar's buffer capacity.
fn energy_of(nbt: &Compound) -> u32 {
    match nbt.get(&key!(K_ENERGY)) {
        Some(Tag::Int(value)) => (*value).clamp(0, ENERGY_CAPACITY) as u32,
        _ => 0,
    }
}

/// Write the FE back into the saved compound, the key the capability reads too.
fn set_energy(nbt: &mut Compound, amount: u32) {
    nbt.insert(key!(K_ENERGY), Tag::Int(amount as i32));
}

/// What the Builder's fluid tank holds, read from the `fluids` compound the game's
/// tank backend reads: a list of `{Fluid, Amount}` entries, one per declared tank.
fn tank_of(nbt: &Compound) -> (Option<String>, u32) {
    let Some(Tag::List(list)) = nbt.get(&key!(K_FLUIDS)) else {
        return (None, 0);
    };
    let Some(Tag::Compound(entry)) = list.first() else {
        return (None, 0);
    };
    let fluid = match entry.get(&key!(K_FLUID_ID)) {
        Some(Tag::String(id)) if id.to_string_lossy() != EMPTY_FLUID => Some(id.to_string_lossy()),
        _ => None,
    };
    let amount = match entry.get(&key!(K_FLUID_AMOUNT)) {
        Some(Tag::Int(value)) => (*value).clamp(0, FLUID_TANK_CAPACITY as i32) as u32,
        _ => 0,
    };
    (fluid, amount)
}

/// Write the tank back in the game's own shape, so the capability and the BE tick agree.
fn set_tank(nbt: &mut Compound, fluid: Option<&str>, amount: u32) {
    let mut entry = Compound::new();
    entry.insert(
        key!(K_FLUID_ID),
        Tag::String(fluid.unwrap_or(EMPTY_FLUID).into()),
    );
    entry.insert(
        key!(K_FLUID_AMOUNT),
        Tag::Int(amount.min(FLUID_TANK_CAPACITY) as i32),
    );
    nbt.insert(key!(K_FLUIDS), Tag::List(vec![Tag::Compound(entry)]));
}

/// Charge one cell's cost to this Builder's buffer. Nothing is written and `false` is returned
/// when the buffer cannot pay, so a stalled cell never acts for free (`GenericEnergyStorage`'s
/// `consumeEnergy` after the jar's "Not enough power!" check).
fn spend(nbt: &mut Compound, spent: &mut u64, base: u32) -> bool {
    let cost = adjusted_cost(base, INFUSION_FACTOR);
    let held = energy_of(nbt);
    if held < cost {
        return false;
    }
    set_energy(nbt, held - cost);
    *spent += u64::from(cost);
    true
}

/// Charge a filtered-out cell the jar's `builderRfPerSkipped`. Upstream calls
/// `consumeEnergy(min(energyStored, builderRfPerSkipped))` and never suspends on a skip, so this
/// drains what the buffer holds (at most 50 FE) and records it apart from the build/quarry spend.
fn spend_skip(nbt: &mut Compound, spent: &mut u64, filter_skipped: &mut u64) {
    let paid = energy_of(nbt).min(filter_skip_cost());
    if paid == 0 {
        return;
    }
    set_energy(nbt, energy_of(nbt) - paid);
    *spent += u64::from(paid);
    *filter_skipped += u64::from(paid);
}

// ---------------------------------------------------------------------------
// The tick
// ---------------------------------------------------------------------------

/// Bring the Builder's card slot into agreement: fold a container change back into the saved data,
/// then mirror the saved card into the container slot if it holds something else.
///
/// The card is kept in the saved compound because that is the one place a click and a command
/// agree on at this pin: a `set_item_slot` made by the click is visible to the click's own read and
/// invisible to the next `/rftoolsbuilder status`, which reported "no shape card" for an installed
/// card (measured 2026-11-19). `event.entity.items` is the game's container view, so it is refreshed
/// here for the menu, a hopper or `ctl container`.
fn sync_card(
    host: &mut dyn ServerHost,
    event: &mut BlockEntityTickEvent<'_>,
) -> Option<(String, CardData)> {
    let shown = string(&event.entity.nbt, K_CARD_SHOWN);
    let held = event
        .entity
        .items
        .first()
        .and_then(Option::as_ref)
        .map(|stack| (stack.item().id(&*host), CardData::read(stack)));
    let mut saved = saved_card(&event.entity.nbt);
    let mut changed = false;
    // An outside change: the container holds a card other than the one mirrored last tick.
    if held.as_ref().map(|(item, _)| item) != shown.as_ref() {
        saved = held.clone();
        changed = true;
    }
    let want = saved.as_ref().and_then(|card| card_stack(&*host, card));
    let current = event.entity.items.first().and_then(Option::as_ref).cloned();
    let same = match (&current, &want) {
        (None, None) => true,
        (Some(have), Some(want)) => have == want,
        _ => false,
    };
    if !same {
        match want.clone() {
            Some(stack) => event.entity.items = vec![Some(stack)],
            None => event.entity.items = vec![None],
        }
        // The filter slot (SLOT_FILTER) stays empty in this port — the filter rides the card
        // component — but the container must still expose both of the jar's slots.
        event.entity.items.resize(CARD_SLOTS, None);
        changed = true;
    }
    let shown_now = saved.as_ref().map(|(item, _)| item.clone());
    if string(&event.entity.nbt, K_CARD_SHOWN) != shown_now {
        match shown_now {
            Some(item) => {
                event
                    .entity
                    .nbt
                    .insert(key!(K_CARD_SHOWN), Tag::String(item.as_str().into()));
            }
            None => {
                event.entity.nbt.remove(&key!(K_CARD_SHOWN));
            }
        }
        changed = true;
    }
    if changed {
        *event.reply = mod_api::BlockEntityTickReply::Changed;
    }
    saved
}

/// Apply one chunk-ticket plan. The ticket's owner is the jar's owner UUID,
/// derived from this Builder's position, so two Builders in one chunk release
/// separately (`TicketOwner::Entity` vs `forceChunk(level, uuid, …)`). The
/// `Entity` variant is used rather than `Block` because a source scanner reads
/// a `Block` variant as an API-type change (a known `fits` false positive).
fn apply_ticket_plan(host: &mut dyn ServerHost, builder: [i32; 3], plan: runtime::TicketPlan) {
    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let controller = ticket_controller();
    let owner = TicketOwner::Entity(runtime::ticket_owner(builder));
    if let Some(chunk) = plan.release
        && let Some(handle) = level.chunk_ref(host, chunk)
    {
        handle.set_mod_forced(host, controller, owner, false, false);
    }
    if let Some(chunk) = plan.force
        && let Some(handle) = level.chunk_ref(host, chunk)
    {
        // The jar's second `forceChunk` passes `false` (bytecode: two `iconst_0`),
        // so the shipped 7.0.8 jar never actually adds the ticket — an upstream
        // defect. The port adopts the behaviour the config and the unload path
        // mean: force the chunk (`true, false`, no natural spawning).
        handle.set_mod_forced(host, controller, owner, true, false);
    }
}

/// One server tick of a Builder: walk the shape and place a block into each
/// cell that can take one, at the jar's [`BLOCKS_PER_TICK`].
///
/// The Builder waits at a cell it has no block for (upstream's `waitMode`, its
/// default), reports `Unchanged` when there was nothing to do, and completes
/// one pass per card (upstream restarts the pass while the machine is enabled;
/// the port does one pass and says so — notes.md).
fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    // The card slot lives in the saved data and is mirrored into the container every tick; a
    // container slot holding something other than what the Builder last mirrored was changed from
    // outside (a click that has not been read back yet) and wins.
    let mirrored = sync_card(host, event);
    let card = mirrored.as_ref().map(|(_, payload)| payload.card);
    let filter = mirrored
        .as_ref()
        .map(|(_, payload)| payload.filter.clone())
        .unwrap_or_default();
    let before = State::read(&event.entity.nbt);
    let mut state = before.clone();

    let Some(card) = card else {
        // No card: release any ticket the last pass held, then no scan.
        if before.forced_chunk.is_some() {
            apply_ticket_plan(host, pos, runtime::ticket_release(before.forced_chunk));
            state.forced_chunk = None;
        }
        if before != State::default() {
            state = State::default();
            state.write(&mut event.entity.nbt);
            *event.reply = mod_api::BlockEntityTickReply::Changed;
        }
        return;
    };
    let Ok(card) = card.validate() else {
        // A refused card releases the last pass's ticket (the jar's `chunkUnload`).
        if before.forced_chunk.is_some() {
            apply_ticket_plan(host, pos, runtime::ticket_release(before.forced_chunk));
            state.forced_chunk = None;
        }
        if before != State::default() {
            state = State::default();
            state.write(&mut event.entity.nbt);
            *event.reply = mod_api::BlockEntityTickReply::Changed;
        }
        return;
    };

    let Some(level) = host.level(host.dimension()) else {
        return;
    };
    let Some((min_y, height)) = level.height(host) else {
        return;
    };

    // The Builder's replacement block (`BuilderConfiguration.quarryReplace`), defaulted to the
    // jar's own value when the compound does not carry one.
    let replace = string(&event.entity.nbt, K_REPLACE).unwrap_or_else(|| QUARRY_REPLACE.to_owned());

    // One pass per card (notes.md, deviation 1): a card that is not the one
    // this pass belongs to starts a fresh pass at the top of the low corner,
    // and a finished pass stays finished until the card changes.
    let mut fingerprint = fingerprint(card, &replace);
    // Editing the filter is a new job, like editing the card: the pass restarts so an already
    // finished Builder rebuilds under the new allow/deny rule.
    fingerprint.push_str(&format!("|{}-{}", filter.value, filter.mode.as_str()));
    if state.card.as_deref() != Some(fingerprint.as_str()) {
        // A new card (or a changed replacement) is the jar's `restartScan`, which starts by
        // releasing the chunk the previous pass held.
        if let Some(held) = before.forced_chunk {
            apply_ticket_plan(host, pos, runtime::ticket_release(Some(held)));
        }
        // One bounded count per pass, kept in the block entity: a repeated readout is free, and
        // even the first one never walks a shape whose box holds tens of millions of cells.
        let (cells, more) = card.count_within(runtime::MAX_PASS_CELLS, runtime::STATUS_CANDIDATES);
        state = State {
            card: Some(fingerprint),
            cells: Some(runtime::CellCount { cells, more }),
            ..State::default()
        };
        state.write(&mut event.entity.nbt);
        *event.reply = mod_api::BlockEntityTickReply::Changed;
    } else if state.done {
        return;
    }
    let mut scan = match state.cursor {
        Some(cursor) => Scan::resume(pos, card, Some(cursor)),
        None => Scan::start(pos, card),
    };
    if !scan.clamp_y(min_y, min_y.saturating_add(height)) {
        state.done = true;
        state.cursor = None;
        state.refusal = Some(Refusal {
            at: scan.min,
            reason: RefusalReason::OutOfWorld,
        });
        state.write(&mut event.entity.nbt);
        *event.reply = mod_api::BlockEntityTickReply::Changed;
        return;
    }

    let mut acted = 0;
    let mut steps = 0;
    while acted < BLOCKS_PER_TICK && steps < LOOKUPS_PER_TICK {
        let Some(cursor) = scan.cursor else {
            break;
        };
        steps += 1;
        if cell_protected(host, level, cursor) {
            // The cell belongs to a player who may not be built in (outside the border): it is
            // skipped and no material is taken for it.
            state.progress.skipped += 1;
        } else if !card.contains(pos, cursor) {
            // Not a cell of the shape: the walk looks past it.
        } else if let CardKind::Quarry(quarry) = card.kind {
            if !level.is_loaded(host, cursor) {
                // The jar's `chunkLoad`: force this cell's chunk, releasing the chunk held
                // before, then wait for it to come in (a fresh chunk loads over the next
                // ticks, so the pass resumes on a later tick).
                let plan = runtime::ticket_step(state.forced_chunk, cursor);
                if plan.release.is_some() || plan.force.is_some() {
                    apply_ticket_plan(host, pos, plan);
                    state.forced_chunk = plan.force;
                }
                state.refusal = Some(Refusal {
                    at: cursor,
                    reason: RefusalReason::Unloaded,
                });
                break;
            }
            match quarry_cell(
                host,
                level,
                pos,
                cursor,
                quarry,
                &card,
                &filter,
                &replace,
                &mut event.entity.nbt,
                &mut state.energy_spent,
                &mut state.filter_skipped,
            ) {
                QuarryCell::Skip => state.progress.skipped += 1,
                QuarryCell::Void => {
                    acted += 1;
                    state.progress.placed += 1;
                    state.progress.voided += 1;
                    state.refusal = None;
                }
                QuarryCell::Harvested => {
                    acted += 1;
                    state.progress.placed += 1;
                    state.refusal = None;
                }
                QuarryCell::NoRoom => {
                    state.refusal = Some(Refusal {
                        at: cursor,
                        reason: RefusalReason::NoRoom,
                    });
                    // Stay on this cell: upstream's wait mode, and the block is not broken, so
                    // nothing is lost while the container is full.
                    state.cursor = Some(cursor);
                    state.write(&mut event.entity.nbt);
                    *event.reply = mod_api::BlockEntityTickReply::Changed;
                    return;
                }
                QuarryCell::Unreadable => {
                    state.refusal = Some(Refusal {
                        at: cursor,
                        reason: RefusalReason::Unloaded,
                    });
                    break;
                }
                QuarryCell::NoPower => {
                    // The jar's "Not enough power!" suspend: nothing was broken or collected,
                    // so stay on this cell and wait for FE (a charged rig resumes the pass).
                    state.refusal = Some(Refusal {
                        at: cursor,
                        reason: RefusalReason::NoPower,
                    });
                    state.cursor = Some(cursor);
                    state.write(&mut event.entity.nbt);
                    *event.reply = mod_api::BlockEntityTickReply::Changed;
                    return;
                }
            }
        } else if matches!(card.kind, CardKind::Pump(_) | CardKind::Liquid) {
            if !level.is_loaded(host, cursor) {
                state.refusal = Some(Refusal {
                    at: cursor,
                    reason: RefusalReason::Unloaded,
                });
                break;
            }
            match fluid_cell(
                host,
                level,
                pos,
                cursor,
                card.kind,
                &mut event.entity.nbt,
                &mut state.energy_spent,
            ) {
                FluidCell::Skipped => state.progress.skipped += 1,
                FluidCell::Pumped | FluidCell::Placed => {
                    acted += 1;
                    state.progress.placed += 1;
                    state.refusal = None;
                }
                FluidCell::Wait(wait) => {
                    state.refusal = Some(Refusal {
                        at: cursor,
                        reason: match wait {
                            FluidWait::NoRoom => RefusalReason::NoRoom,
                            FluidWait::NoLiquid => RefusalReason::NoLiquid,
                        },
                    });
                    state.cursor = Some(cursor);
                    state.write(&mut event.entity.nbt);
                    *event.reply = mod_api::BlockEntityTickReply::Changed;
                    return;
                }
                FluidCell::NoPower => {
                    state.refusal = Some(Refusal {
                        at: cursor,
                        reason: RefusalReason::NoPower,
                    });
                    state.cursor = Some(cursor);
                    state.write(&mut event.entity.nbt);
                    *event.reply = mod_api::BlockEntityTickReply::Changed;
                    return;
                }
            }
        } else {
            if !level.is_loaded(host, cursor) {
                // Nothing can be read or written there; look again next tick.
                state.refusal = Some(Refusal {
                    at: cursor,
                    reason: RefusalReason::NoMaterial,
                });
                break;
            }
            // The filter module: upstream tests `filterCache.test(Block.getCloneItemStack(…))`
            // and, when it is false, consumes `builderRfPerSkipped` and calls `skip(data)` — the
            // cell is never built. A block's id is its clone item for the blocks this package
            // carries; a `#tag` value is checked against the host's block tag table.
            let passes = if filter.is_empty() {
                true
            } else {
                match level.block(host, cursor) {
                    Some(block) => {
                        let id = block.id(&*host);
                        filter_match(&filter, &id, |_id, tag| {
                            host.block_in_tag(block, tag).unwrap_or(false)
                        })
                    }
                    None => true,
                }
            };
            if !passes {
                spend_skip(
                    &mut event.entity.nbt,
                    &mut state.energy_spent,
                    &mut state.filter_skipped,
                );
                state.progress.skipped += 1;
                state.refusal = None;
            } else if is_replaceable(host, level, cursor) {
                match place(
                    host,
                    level,
                    pos,
                    cursor,
                    &mut event.entity.nbt,
                    &mut state.energy_spent,
                ) {
                    Placed::Yes => {
                        acted += 1;
                        state.progress.placed += 1;
                        state.refusal = None;
                    }
                    Placed::NoMaterial => {
                        state.refusal = Some(Refusal {
                            at: cursor,
                            reason: RefusalReason::NoMaterial,
                        });
                        // Stay on this cell: upstream's wait mode.
                        state.cursor = Some(cursor);
                        state.write(&mut event.entity.nbt);
                        *event.reply = mod_api::BlockEntityTickReply::Changed;
                        return;
                    }
                    Placed::Rejected => state.progress.skipped += 1,
                    Placed::NoPower => {
                        state.refusal = Some(Refusal {
                            at: cursor,
                            reason: RefusalReason::NoPower,
                        });
                        state.cursor = Some(cursor);
                        state.write(&mut event.entity.nbt);
                        *event.reply = mod_api::BlockEntityTickReply::Changed;
                        return;
                    }
                }
            } else {
                state.progress.skipped += 1;
            }
        }
        if !scan.advance() {
            state.done = true;
            state.cursor = None;
            break;
        }
    }
    if !state.done {
        state.cursor = scan.cursor;
    }
    if state.done && state.forced_chunk.is_some() {
        // The pass no longer needs the chunk: release the ticket so a later rig in the
        // same chunk must force it for itself. This applies the jar's `chunkUnload` when
        // the work finishes (the jar only releases it on restartScan or block removal).
        apply_ticket_plan(
            host,
            pos,
            runtime::ticket_release(state.forced_chunk.take()),
        );
    }
    if state != before {
        state.write(&mut event.entity.nbt);
        *event.reply = mod_api::BlockEntityTickReply::Changed;
    }
}

/// What one quarry cell did.
enum QuarryCell {
    /// The block must not be broken here (air, the replacement, unbreakable, the Builder's own
    /// box, the `dontremoveme` tag).
    Skip,
    /// The block was removed and its drops collected.
    Harvested,
    /// The block was removed and nothing was collected (the card's void list).
    Void,
    /// The drops do not fit in the container: wait here, break nothing.
    NoRoom,
    /// The cell's block could not be read.
    Unreadable,
    /// The buffer cannot pay for this cell: nothing was broken or collected.
    NoPower,
}

/// One quarry cell: the jar's `commonQuarryBlock`, then its `clearOrDirtBlock`.
///
/// **Upstream defect fixed.** The jar rolls and inserts a block's drops *before*
/// `clearOrDirtBlock` reads the `rftoolsbuilder:dontremoveme` tag, and only then returns
/// without removing the block (`BuilderTileEntity.java:1246` inserts, `:1109-1114` returns
/// on the tag), so a tagged block hands out its drops on every pass while staying in place.
/// The port skips a tagged block before anything is rolled.
#[expect(
    clippy::too_many_arguments,
    reason = "each argument is a distinct game input or output the quarry rule reads; a grouping struct used once would add noise"
)]
fn quarry_cell(
    host: &mut dyn ServerHost,
    level: LevelRef,
    builder: [i32; 3],
    cursor: [i32; 3],
    quarry: Quarry,
    card: &Card,
    filter: &Filter,
    replace: &str,
    nbt: &mut Compound,
    spent: &mut u64,
    filter_skipped: &mut u64,
) -> QuarryCell {
    let Some(block_state) = level.block_state(host, cursor) else {
        return QuarryCell::Unreadable;
    };
    let Some(block) = level.block(host, cursor) else {
        return QuarryCell::Unreadable;
    };
    let id = block.id(&*host);
    let near_builder = (0..3).all(|axis| (cursor[axis] - builder[axis]).abs() <= BUILDER_CELLS);
    let facts = CellFacts {
        near_builder,
        air: runtime::is_air_block(&id),
        destroy_speed: host
            .block_states()
            .destroy_speed(block_state)
            .unwrap_or(-1.0),
        is_replacement: id == replace,
        voided: is_voided(&*host, card, block, &id),
        // The same `filter_match` the shaped path proved, applied to the quarry's clone stack
        // (here the block id and, for a `#tag` value, the host's block tag table).
        filter_passes: filter_match(filter, &id, |_id, tag| {
            host.block_in_tag(block, tag).unwrap_or(false)
        }),
    };
    let step = runtime::quarry_step(quarry, facts);
    if step == QuarryStep::Skip(runtime::SkipReason::Empty) {
        return QuarryCell::Skip;
    }
    // The tag upstream leaves in place; checked before any drop is rolled (the defect above).
    if host.block_in_tag(block, DONT_REMOVE_ME).unwrap_or(false) {
        return QuarryCell::Skip;
    }
    // `commonQuarryBlock` consumes `min(energyStored, builderRfPerSkipped)` on a filtered-out
    // cell and calls `skip(data)`: the block stays, no drops are inserted, and the cell is never
    // charged the quarry cost. `spend_skip` drains at most 50 FE without suspending, as upstream.
    if step == QuarryStep::Skip(runtime::SkipReason::Filtered) {
        spend_skip(nbt, spent, filter_skipped);
        return QuarryCell::Skip;
    }
    // `CARD_VOID` costs builderRfPerOperation × voidShapeCardFactor; the other quarry types
    // cost builderRfPerQuarry. Check before the drops are inserted so an unpowered cell never
    // hands out an item while its block stays in place.
    let base = if quarry.voids {
        cell_cost(CellCharge::Void)
    } else {
        cell_cost(CellCharge::Quarry)
    };
    let cost = adjusted_cost(base, INFUSION_FACTOR);
    if energy_of(nbt) < cost {
        return QuarryCell::NoPower;
    }
    let mut collected = false;
    if step == QuarryStep::Harvest {
        if !insert_drops(host, level, builder, cursor, quarry) {
            return QuarryCell::NoRoom;
        }
        collected = true;
    }
    let _ = spend(nbt, spent, base);
    if !clear_or_replace(host, level, cursor, block_state, quarry.clearing, replace) {
        // The write was refused: put the FE back rather than charge for a cell that did not act.
        *spent = spent.saturating_sub(u64::from(cost));
        set_energy(nbt, energy_of(nbt) + cost);
        return QuarryCell::Skip;
    }
    if collected {
        QuarryCell::Harvested
    } else {
        QuarryCell::Void
    }
}

/// What one pump/liquid cell did.
enum FluidCell {
    /// The jar's early skip: no source here, the Builder's own box, or unbreakable.
    Skipped,
    /// A source was removed and a whole bucket went into the Builder's tank.
    Pumped,
    /// A bucket was drained and its fluid block written into the cell.
    Placed,
    /// `waitOrSkip`: no whole bucket in the tank, or no room for one more.
    Wait(FluidWait),
    /// The buffer cannot pay for this cell: nothing moved.
    NoPower,
}

/// One pump/liquid cell: the jar's `pumpBlock` and `placeLiquidBlock`, with the Builder's
/// own tank standing in for the external tank above or below it. The millibucket rule and
/// the early guards are the pure [`fluid_step`]; this function only reads the world and
/// writes the tank, the block and the FE.
fn fluid_cell(
    host: &mut dyn ServerHost,
    level: LevelRef,
    builder: [i32; 3],
    at: [i32; 3],
    kind: CardKind,
    nbt: &mut Compound,
    spent: &mut u64,
) -> FluidCell {
    let (held_fluid, held_amount) = tank_of(nbt);
    // `FluidState.isSource()`: this package's source blocks are the vanilla water and lava
    // source blocks; a flowing fluid's block id is not one of these.
    let source: Option<&'static str> = match level.block(host, at).map(|block| block.id(&*host)) {
        Some(id) if id == "minecraft:water" => Some("minecraft:water"),
        Some(id) if id == "minecraft:lava" => Some("minecraft:lava"),
        _ => None,
    };
    let destroy_speed = level
        .block_state(host, at)
        .and_then(|state| host.block_states().destroy_speed(state))
        .unwrap_or(0.0);
    let facts = FluidFacts {
        near_builder: (0..3).all(|axis| (at[axis] - builder[axis]).abs() <= BUILDER_CELLS),
        source,
        destroy_speed,
        replaceable: is_replaceable(host, level, at),
        tank_fluid: held_fluid.as_deref(),
        tank_amount: held_amount,
    };
    let step = fluid_step(kind, facts);
    match step {
        FluidStep::Skip(_) => return FluidCell::Skipped,
        FluidStep::Wait(wait) => return FluidCell::Wait(wait),
        FluidStep::PumpIn | FluidStep::Place => {}
    }
    if energy_of(nbt) < fluid_cost() {
        return FluidCell::NoPower;
    }
    let (fluid, amount) = tank_after(step, held_fluid.as_deref(), held_amount);
    let written = match step {
        FluidStep::PumpIn => {
            // The pumped fluid's id is the *source* at this cell, not what the tank
            // held before (which is empty on the first bucket).
            let Some(id) = source else {
                return FluidCell::Skipped;
            };
            set_tank(nbt, Some(id), amount);
            // The jar's `FluidTools.pickupFluidBlock` leaves the source cell as air.
            host.block_state_id("minecraft:air")
        }
        FluidStep::Place => {
            // The fluid to write is what the tank held *before* the spend: on the
            // last bucket `tank_after` clears the id because the tank is now empty.
            let Some(id) = held_fluid.as_deref() else {
                return FluidCell::Wait(FluidWait::NoLiquid);
            };
            set_tank(nbt, fluid, amount);
            host.block_state_id(id)
        }
        FluidStep::Skip(_) | FluidStep::Wait(_) => unreachable!("handled above"),
    };
    let Some(state) = written else {
        return FluidCell::Skipped;
    };
    let flags = BlockUpdateFlags::CLIENTS | BlockUpdateFlags::NEIGHBORS;
    if !level.set_block(host, at, state, flags).unwrap_or(false) {
        // The write was refused: put the tank and the FE back, as the quarry does.
        set_tank(nbt, held_fluid.as_deref(), held_amount);
        return FluidCell::Skipped;
    }
    let _ = spend(nbt, spent, fluid_cost());
    if step == FluidStep::PumpIn {
        FluidCell::Pumped
    } else {
        FluidCell::Placed
    }
}

/// Whether the card's void list covers the block at this cell: its material by its own id, or,
/// with the card's `tagMatching` on, a member of that material's block tag
/// (`ShapeCardItem.getVoidedBlocks`).
fn is_voided(host: &dyn Host, card: &Card, block: BlockKey, id: &str) -> bool {
    if let Some(material) = runtime::material_of(id)
        && card.voiding.contains(material)
    {
        return true;
    }
    if !card.tag_matching {
        return false;
    }
    runtime::VOID_MATERIALS.iter().any(|material| {
        card.voiding.contains(material)
            && host
                .block_in_tag(block, runtime::tag_of(material))
                .unwrap_or(false)
    })
}

/// Rolls the block's drops at `at` — silk touch, fortune III or plain, with the jar's super
/// harvesting tool — and puts them into the container above the Builder, else below it.
/// Returns whether they all fit; nothing is taken out of the world here.
fn insert_drops(
    host: &mut dyn ServerHost,
    level: LevelRef,
    builder: [i32; 3],
    at: [i32; 3],
    quarry: Quarry,
) -> bool {
    let tool = quarry_tool(host, quarry);
    let Ok(Some(drops)) = level.block_drops(host, at, tool.as_ref(), None) else {
        return false;
    };
    for stack in &drops {
        let resource = ItemResource::of(stack);
        let mut left = stack.count();
        for offset in [1, -1] {
            if left == 0 {
                break;
            }
            let container = [builder[0], builder[1] + offset, builder[2]];
            if !level.is_loaded(host, container) {
                continue;
            }
            let Some(handler) = level.capability(host, &ITEMS, container, None) else {
                continue;
            };
            let went = host
                .transaction(|host, tx| {
                    Ok::<u32, ()>(handler.insert_any(host, tx, &resource, left))
                })
                .unwrap_or(0);
            left = left.saturating_sub(went);
        }
        if left > 0 {
            return false;
        }
    }
    true
}

/// The stack a quarry harvests with: a netherite pickaxe (`SuperHarvestingTool` is
/// `Tiers.NETHERITE` over `mineable/pickaxe`) carrying silk touch when the card asks for it and
/// fortune III when it asks for that (`getHarvesterTool`).
fn quarry_tool(host: &mut dyn ServerHost, quarry: Quarry) -> Option<ItemStack> {
    let key = host.item_key(&Identifier::parse_namespaced(QUARRY_TOOL)?)?;
    let mut stack = ItemStack::of(key);
    let mut levels = Compound::new();
    if quarry.silk {
        levels.insert(key!("minecraft:silk_touch"), Tag::Int(1));
    }
    if quarry.fortune {
        levels.insert(
            key!("minecraft:fortune"),
            Tag::Int(i32::try_from(FORTUNE_LEVEL).unwrap_or(1)),
        );
    }
    if levels.is_empty() {
        return Some(stack);
    }
    let mut patch = Compound::new();
    patch.insert(key!("minecraft:enchantments"), Tag::Compound(levels));
    stack.set_components(patch).ok()?;
    Some(stack)
}

/// `clearOrDirtBlock`: the block's break sound, then air when the card is clearing or the
/// Builder's replacement block (`Block.UPDATE_CLIENTS`, as upstream writes it — no neighbour
/// update).
fn clear_or_replace(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    state: BlockState,
    clearing: bool,
    replace: &str,
) -> bool {
    if let Some(sound) = host.block_states().sound(state) {
        level.play_sound(
            host,
            sound.break_sound.as_ref(),
            [
                f64::from(pos[0]) + 0.5,
                f64::from(pos[1]) + 0.5,
                f64::from(pos[2]) + 0.5,
            ],
            SoundSource::Blocks,
            sound.volume,
            sound.pitch,
        );
    }
    let id = if clearing { "minecraft:air" } else { replace };
    let Some(block) = host.block_key(id) else {
        return false;
    };
    let Some(target) = host.block_state_default(block) else {
        return false;
    };
    level
        .set_block(host, pos, target, BlockUpdateFlags::CLIENTS)
        .unwrap_or(false)
}

/// What identifies the card a pass belongs to: its shape, solidity, size,
/// offset and turn, plus what changes a quarry's work (its type, its void list and the
/// Builder's replacement block), so re-writing any of them starts a new pass.
fn fingerprint(card: Card, replace: &str) -> String {
    let [dx, dy, dz] = card.dimension;
    let [ox, oy, oz] = card.offset;
    format!(
        "{}-{}-{dx}x{dy}x{dz}-{ox},{oy},{oz}-{}{}-{}-{}-{}-{replace}",
        card.shape.id(),
        card.solid,
        card.modifier.rotation.id(),
        if card.modifier.flip_y { "-flip" } else { "" },
        card.kind.suffix(),
        card.voiding.names(),
        card.tag_matching
    )
}

/// Whether `pos` can take a block: `BlockState.canBeReplaced()` (air, grass,
/// water, snow layers), as the jar's `isEmptyOrReplacable`.
fn is_replaceable(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    level
        .block_state(host, pos)
        .is_some_and(|state| host.block_states().replaceable(state).unwrap_or(false))
}

/// What one placement did.
enum Placed {
    /// A block went in.
    Yes,
    /// Nothing placeable is in reach: the Builder waits here.
    NoMaterial,
    /// A block was in reach but the cell took it not.
    Rejected,
    /// The buffer cannot pay for this cell: no block was taken.
    NoPower,
}

/// Places one block into `at`, taking it from the container above the Builder
/// or, failing that, below it (upstream's `createTakeableItem` order). A shaped cell costs
/// `builderRfPerOperation`, charged only after the block was actually written.
fn place(
    host: &mut dyn ServerHost,
    level: LevelRef,
    builder: [i32; 3],
    at: [i32; 3],
    nbt: &mut Compound,
    spent: &mut u64,
) -> Placed {
    let Some(Material {
        handler,
        index,
        resource,
        block,
    }) = find_material(host, level, builder)
    else {
        return Placed::NoMaterial;
    };
    let Some(state) = host.block_state_default(block) else {
        return Placed::Rejected;
    };
    // The jar's `buildBlock` consumes RF after placing; wait before taking a block if the
    // buffer cannot pay, so an unpowered Builder never hands out material for free.
    if energy_of(nbt) < adjusted_cost(cell_cost(CellCharge::Build), INFUSION_FACTOR) {
        return Placed::NoPower;
    }
    let taken = host
        .transaction(|host, tx| Ok::<u32, ()>(handler.extract(host, tx, index, &resource, 1)))
        .unwrap_or(0);
    if taken != 1 {
        return Placed::NoMaterial;
    }
    let flags = BlockUpdateFlags::CLIENTS | BlockUpdateFlags::NEIGHBORS;
    if !level.set_block(host, at, state, flags).unwrap_or(false) {
        // Put the block back rather than lose it, as upstream's
        // `takeAndReplace` does.
        let _ = host
            .transaction(|host, tx| Ok::<u32, ()>(handler.insert(host, tx, index, &resource, 1)));
        return Placed::Rejected;
    }
    // Upstream places through a fake player and plays the block's place sound
    // unless the Builder is silent; there is no fake player here, so the sound
    // is the part worth keeping.
    if let Some(sound) = host.block_states().sound(state) {
        level.play_sound(
            host,
            sound.place.as_ref(),
            [
                f64::from(at[0]) + 0.5,
                f64::from(at[1]) + 0.5,
                f64::from(at[2]) + 0.5,
            ],
            SoundSource::Blocks,
            sound.volume,
            sound.pitch,
        );
    }
    let _ = spend(nbt, spent, cell_cost(CellCharge::Build));
    Placed::Yes
}

/// A block the Builder can place, with the handler it comes from.
struct Material {
    handler: Capability<dyn ResourceHandler<ItemResource>>,
    index: usize,
    resource: ItemResource,
    block: BlockKey,
}

/// The first block item in the container above the Builder, else in the one
/// below it: the jar's `createTakeableItem(Direction.UP)` then `DOWN`.
fn find_material(
    host: &mut dyn ServerHost,
    level: LevelRef,
    builder: [i32; 3],
) -> Option<Material> {
    for offset in [1, -1] {
        let at = [builder[0], builder[1] + offset, builder[2]];
        if !level.is_loaded(host, at) {
            continue;
        }
        let Some(handler) = level.capability(host, &ITEMS, at, None) else {
            continue;
        };
        let size = handler.size(host);
        for index in 0..size {
            let Some(resource) = handler.resource(host, index) else {
                continue;
            };
            let Some(block) = host.item_block(resource.item()) else {
                continue;
            };
            return Some(Material {
                handler,
                index,
                resource,
                block,
            });
        }
    }
    None
}

// ---------------------------------------------------------------------------
// The player's right click on a Builder
// ---------------------------------------------------------------------------

/// Right click: a card in hand goes into the card slot, an empty hand reads the
/// Builder out, and a sneaking empty hand takes the card back. Any other stack
/// is left alone, so a block can still be placed against the Builder.
pub(crate) fn on_right_click_block(
    host: HostSide<'_>,
    event: &mut RightClickBlockEvent<'_>,
) -> Flow {
    let click = event.click;
    if click.hand != Hand::Main {
        return Flow::Continue;
    }
    let HostSide::Server(server) = host else {
        return Flow::Continue;
    };
    if !is_builder(&*server, click.pos) {
        return Flow::Continue;
    }
    let stack = click.stack.cloned();
    let card_held = stack
        .as_ref()
        .is_some_and(|stack| stack.item().id(&*server) == CARD_ITEM);
    if !card_held && stack.is_some() {
        return Flow::Continue;
    }
    if card_held {
        install(server, click.pos, click.player, stack);
    } else {
        let sneaking = server
            .player(click.player)
            .and_then(|player| player.is_secondary_use_active(&*server))
            .unwrap_or(false);
        if sneaking {
            take_card(server, click.pos, click.player);
        } else {
            let line = status_at(&*server, click.pos);
            action_bar(server, click.player, &line);
        }
    }
    *event.cancellation_result = InteractionResult::Success;
    Flow::Cancel
}

/// The block at `pos` is a Builder.
fn is_builder(host: &dyn Host, pos: [i32; 3]) -> bool {
    host.level(host.dimension())
        .and_then(|level| level.block(host, pos))
        == Some(block_key())
}

/// Puts the held card into the Builder's card slot (only into an empty one) and
/// takes it from the player's hand.
fn install(host: &mut dyn ServerHost, pos: [i32; 3], uuid: u128, stack: Option<ItemStack>) {
    let Some(stack) = stack else {
        return;
    };
    let Some(entity) = entity_at(host, pos) else {
        return;
    };
    let mut data = entity.data(&*host).unwrap_or_default();
    // The slot holds one card, as upstream's does; clicking with another swaps them and the old
    // one goes back to the player rather than vanishing.
    if let Some(old) = saved_card(&data) {
        let Some(old_stack) = card_stack(&*host, &old) else {
            return;
        };
        let returned = host
            .player(uuid)
            .is_some_and(|player| player.give_stack(host, old_stack).is_ok());
        if !returned {
            action_bar(
                host,
                uuid,
                "Builder: the card in it could not be given back",
            );
            return;
        }
    }
    let Some(item) = stack.item().id(&*host).into() else {
        return;
    };
    let payload = CardData::read(&stack);
    write_saved_card(&mut data, Some((&item, &payload)));
    // Outside a transaction: a `replace_data` inside one is rolled back with it at this pin
    // (measured on `integratedcrafting`, 2026-11-19), which is why the card is written here and the
    // hand is charged after it.
    if entity.replace_data(host, data).is_err() {
        action_bar(host, uuid, "Builder: the card could not be put in");
        return;
    }
    let _ = entity.set_changed(host);
    take_from_hand(host, uuid, &stack, 1);
    // The container slot follows on the next tick (`sync_card`); the action bar is right now.
    let line = status_at(host, pos);
    action_bar(host, uuid, &line);
}

/// Takes the card out of the Builder into the player's inventory.
fn take_card(host: &mut dyn ServerHost, pos: [i32; 3], uuid: u128) {
    let Some(entity) = entity_at(host, pos) else {
        return;
    };
    let mut data = entity.data(&*host).unwrap_or_default();
    let Some(card) = saved_card(&data) else {
        action_bar(host, uuid, "Builder: no card to take");
        return;
    };
    let Some(stack) = card_stack(&*host, &card) else {
        action_bar(host, uuid, "Builder: the card could not be read");
        return;
    };
    write_saved_card(&mut data, None);
    if entity.replace_data(host, data.clone()).is_err() {
        return;
    }
    let given = match host.player(uuid) {
        Some(player) => player.give_stack(host, stack).is_ok(),
        None => false,
    };
    if !given {
        // The card stays in the Builder rather than vanishing.
        write_saved_card(&mut data, Some((&card.0, &card.1)));
        let _ = entity.replace_data(host, data);
        action_bar(host, uuid, "Builder: the card could not be given back");
        return;
    }
    let _ = entity.set_changed(host);
    // The card is out: the pass it was driving goes with it.
    let mut cleared = Compound::new();
    State::default().write(&mut cleared);
    let _ = entity.merge_data(host, cleared);
    action_bar(host, uuid, "Builder: card taken");
}

/// The mod block entity at `pos`, when one stands there.
fn entity_at(host: &dyn Host, pos: [i32; 3]) -> Option<BlockEntityRef> {
    let level = host.level(host.dimension())?;
    host.block_entity(level, pos)
}

/// Answers in the acting player's action bar.
fn action_bar(host: &mut dyn ServerHost, uuid: u128, line: &str) {
    if let Some(player) = host.player(uuid) {
        let _ = player.action_bar(host, Text::literal(line));
    }
}

/// Takes `taken` items off the acting player's held stack.
fn take_from_hand(host: &mut dyn ServerHost, uuid: u128, held: &ItemStack, taken: u32) {
    let Some(player) = host.player(uuid) else {
        return;
    };
    let Some(slot) = player.selected_slot(host) else {
        return;
    };
    let remainder = if held.count() > taken {
        let mut rest = held.clone();
        let _ = rest.set_count(held.count() - taken);
        Some(rest)
    } else {
        None
    };
    let _ = player.set_inventory_slot(host, u32::try_from(slot).unwrap_or(u32::MAX), remainder);
}

// ---------------------------------------------------------------------------
// The card's item Part: authoring a shape by its two corners
// ---------------------------------------------------------------------------

/// The card item's Part: sneak-right-click a Builder, then click two corners,
/// as upstream's `ShapeCardItem.useOn` does.
struct ShapeCardItem;

#[mod_api::horizon_part]
impl ItemPart for ShapeCardItem {
    fn use_on(&self, host: HostSide<'_>, event: &mut UseOnEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(server) = host else {
            // The server answers; the client's own answer would only swing.
            return None;
        };
        authorize(server, event).then_some(InteractionResult::Success)
    }
}

/// The corner-selection step of a card's right click on a block. Returns
/// whether the click was used.
fn authorize(host: &mut dyn ServerHost, event: &UseOnEvent<'_>) -> bool {
    if event.hand != Hand::Main {
        return false;
    }
    let Some(player) = host.player(event.player) else {
        return false;
    };
    let Some(level) = host.level(host.dimension()) else {
        return false;
    };
    let dimension = level.dimension_id().to_owned();
    let Some(mut data) = held_card(host, &player) else {
        return false;
    };
    let pos = event.pos;
    let mut message = None;
    match data.selection.mode {
        0 => {
            let sneaking = player.is_secondary_use_active(&*host).unwrap_or(false);
            if sneaking && is_builder(&*host, pos) {
                data.selection.mode = 1;
                data.selection.target = Some((dimension, pos));
                data.selection.corner1 = None;
                message = Some("Now select the first corner");
            } else if sneaking {
                message = Some("You can only do this on a builder!");
            } else {
                // A plain click on a block is the card's own use: used, silent,
                // as upstream's `return InteractionResult.SUCCESS`.
                return true;
            }
        }
        1 | 2 => match selection_guard(&data.selection, &dimension, pos) {
            Guard::NoBuilder => message = Some("There is no Builder selected!"),
            Guard::OtherDimension => message = Some("The Builder is in another dimension!"),
            Guard::Cleared => {
                data.selection = Selection::default();
                message = Some("Cleared area selection mode!");
            }
            Guard::Ok => {
                if data.selection.mode == 1 {
                    data.selection.corner1 = Some(pos);
                    data.selection.mode = 2;
                    message = Some("Now select the second corner");
                } else if let Some(corner1) = data.selection.corner1 {
                    if let Some((_, builder)) = data.selection.target {
                        data.card = from_corners(corner1, pos, builder);
                        data.selection = Selection::default();
                        message = Some("New settings copied to the shape card!");
                    } else {
                        data.selection = Selection::default();
                        message = Some("Cleared area selection mode!");
                    }
                }
            }
        },
        _ => {}
    }
    let written = write_held_card(host, &player, &data);
    if let Some(message) = message
        && written
    {
        action_bar(host, event.player, message);
    }
    true
}

/// What the corner-selection guards say about a click.
enum Guard {
    /// The card is bound to a Builder and this click picks a corner.
    Ok,
    /// The card names no Builder.
    NoBuilder,
    /// The Builder is in another dimension.
    OtherDimension,
    /// The click was the bound Builder itself: the selection is dropped.
    Cleared,
}

/// The two guards upstream asks before a corner is taken.
fn selection_guard(selection: &Selection, dimension: &str, pos: [i32; 3]) -> Guard {
    let Some((target_dimension, builder)) = &selection.target else {
        return Guard::NoBuilder;
    };
    if target_dimension != dimension {
        return Guard::OtherDimension;
    }
    if *builder == pos {
        return Guard::Cleared;
    }
    Guard::Ok
}

/// The card a two-corner click produces: the box the corners span, centred
/// between them and offset from the Builder, solid
/// (`ShapeCardItem.useOn`'s arithmetic).
fn from_corners(corner1: [i32; 3], corner2: [i32; 3], builder: [i32; 3]) -> Card {
    let mut dimension = [0; 3];
    let mut offset = [0; 3];
    for axis in 0..3 {
        // `(int) Math.ceil((c1 + pos) / 2.0f)`
        let centre = (f32::from(i16::try_from(corner1[axis] + corner2[axis]).unwrap_or(0)) / 2.0)
            .ceil() as i32;
        dimension[axis] = (corner1[axis] - corner2[axis]).abs() + 1;
        offset[axis] = centre - builder[axis];
    }
    Card {
        shape: Shape::Box,
        solid: true,
        dimension,
        offset,
        modifier: runtime::Modifier::default(),
        ..Card::default()
    }
}

/// The card the acting player holds, if the held stack is a card.
fn held_card(host: &dyn ServerHost, player: &PlayerRef) -> Option<CardData> {
    let slot = player.selected_slot(host)?;
    let held = player
        .inventory_slots(host)?
        .into_iter()
        .find(|(at, _)| *at == u32::try_from(slot).unwrap_or(u32::MAX))?
        .1;
    if held.item().id(host) != CARD_ITEM {
        return None;
    }
    Some(CardData::read(&held))
}

/// Writes the card back into the acting player's held slot.
fn write_held_card(host: &mut dyn ServerHost, player: &PlayerRef, data: &CardData) -> bool {
    let Some(slot) = player.selected_slot(host) else {
        return false;
    };
    let slot = u32::try_from(slot).unwrap_or(u32::MAX);
    let Some(held) = player
        .inventory_slots(host)
        .and_then(|slots| slots.into_iter().find(|(at, _)| *at == slot))
        .map(|(_, stack)| stack)
    else {
        return false;
    };
    let mut held = held;
    if !data.write(&mut held) {
        return false;
    }
    player.set_inventory_slot(host, slot, Some(held)).is_ok()
}

// ---------------------------------------------------------------------------
// The readout commands
// ---------------------------------------------------------------------------

/// `/rftoolsbuilder status|shape|offset|reset` (`register_commands`).
pub(crate) fn on_register_commands(event: &mut RegisterCommandsEvent<'_>) {
    event
        .commands
        .push(Command::new("rftoolsbuilder status <pos:block_pos>"));
    event
        .commands
        .push(Command::new("rftoolsbuilder forced <pos:block_pos>"));
    // The Builder's fluid tank: the pump/liquid cards' exact millibuckets, read back in game.
    event
        .commands
        .push(Command::new("rftoolsbuilder fluid <pos:block_pos>"));

    // The energy readout and the charger (a harness addition, as `/status` is: with no GUI
    // and no cable in this package, a player otherwise cannot fill the buffer).
    event
        .commands
        .push(Command::new("rftoolsbuilder energy <pos:block_pos>"));
    event.commands.push(Command::new(
        "rftoolsbuilder charge <pos:block_pos> <amount:integer>",
    ));
    event.commands.push(Command::new(
        "rftoolsbuilder shape <shape:word> <dx:integer> <dy:integer> <dz:integer>",
    ));
    event.commands.push(Command::new(
        "rftoolsbuilder shape <shape:word> <dx:integer> <dy:integer> <dz:integer> <solid:word>",
    ));
    event.commands.push(Command::new(
        "rftoolsbuilder offset <x:integer> <y:integer> <z:integer>",
    ));
    event.commands.push(Command::new("rftoolsbuilder reset"));
    event
        .commands
        .push(Command::new("rftoolsbuilder type <type:word>"));
    event
        .commands
        .push(Command::new("rftoolsbuilder void <material:word>"));
    event
        .commands
        .push(Command::new("rftoolsbuilder tagmatching <mode:word>"));
    event.commands.push(Command::new(
        "rftoolsbuilder filter <value:resource:minecraft:block> <mode:word>",
    ));
    // The resource argument cannot spell "none" (it must name a block), so clearing the filter
    // is its own usage. `word` is not usable for a value: brigadier's word parser rejects `:`.
    event
        .commands
        .push(Command::new("rftoolsbuilder filter none"));
    event.commands.push(Command::new(
        "rftoolsbuilder replace <pos:block_pos> <block:resource:minecraft:block>",
    ));
}

/// Their bodies (`run_mod_command`).
pub(crate) fn on_run_mod_command(
    host: &mut dyn ServerHost,
    event: &mut RunModCommandEvent<'_>,
) -> Flow {
    let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
    let [root, sub, ..] = literals.as_slice() else {
        return Flow::Continue;
    };
    if *root != "rftoolsbuilder" {
        return Flow::Continue;
    }
    match *sub {
        "status" => {
            let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
                feedback(host, "usage: /rftoolsbuilder status <x> <y> <z>");
                return Flow::Cancel;
            };
            let pos = *pos;
            if !is_builder(host, pos) {
                // A generic refusal: naming the block there would turn a Builder readout into a
                // probe for whatever a player points it at (review 1276-rftoolsbuilder).
                feedback(host, "not a builder");
                return Flow::Cancel;
            }
            let line = status_at(host, pos);
            feedback(host, &line);
            Flow::Cancel
        }
        "forced" => {
            let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
                feedback(host, "usage: /rftoolsbuilder forced <x> <y> <z>");
                return Flow::Cancel;
            };
            let line = forced_at(host, *pos);
            feedback(host, &line);
            Flow::Cancel
        }
        "fluid" => {
            let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
                feedback(host, "usage: /rftoolsbuilder fluid <x> <y> <z>");
                return Flow::Cancel;
            };
            let line = fluid_at(host, *pos);
            feedback(host, &line);
            Flow::Cancel
        }
        "energy" => {
            let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
                feedback(host, "usage: /rftoolsbuilder energy <x> <y> <z>");
                return Flow::Cancel;
            };
            let line = energy_at(host, *pos);
            feedback(host, &line);
            Flow::Cancel
        }
        "charge" => {
            let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
                feedback(host, "usage: /rftoolsbuilder charge <x> <y> <z> <amount>");
                return Flow::Cancel;
            };
            let Some(CommandArgumentValue::Integer(amount)) = event.command.argument("amount")
            else {
                feedback(host, "usage: /rftoolsbuilder charge <x> <y> <z> <amount>");
                return Flow::Cancel;
            };
            let line = charge_block(host, *pos, *amount);
            feedback(host, &line);
            Flow::Cancel
        }
        "shape" => {
            let line = write_shape(host, event);
            feedback(host, &line);
            Flow::Cancel
        }
        "offset" => {
            let line = write_offset(host, event);
            feedback(host, &line);
            Flow::Cancel
        }
        "reset" => {
            let line = with_held_card(host, |data| {
                *data = CardData::default();
                "Card reset to the jar's default: Box (Solid) 5x5x5 offset 0 0 0".to_owned()
            });
            feedback(
                host,
                &line.unwrap_or_else(|| "Hold a shape card in your main hand".into()),
            );
            Flow::Cancel
        }
        "type" => {
            let line = write_type(host, event);
            feedback(host, &line);
            Flow::Cancel
        }
        "void" => {
            let line = write_void(host, event);
            feedback(host, &line);
            Flow::Cancel
        }
        "tagmatching" => {
            let line = write_tag_matching(host, event);
            feedback(host, &line);
            Flow::Cancel
        }
        "filter" => {
            let line = write_filter(host, event);
            feedback(host, &line);
            Flow::Cancel
        }
        "replace" => {
            let line = write_replace(host, event);
            feedback(host, &line);
            Flow::Cancel
        }
        _ => Flow::Continue,
    }
}

/// `/rftoolsbuilder type <type>`: the card's `ShapeCardType` (the jar's item id picks it, and
/// this port carries one card item, so the type rides the card).
fn write_type(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> String {
    let Some(CommandArgumentValue::String(name)) = event.command.argument("type") else {
        return "usage: /rftoolsbuilder type <shape|quarry|quarry_silk|quarry_fortune|quarry_clear|quarry_clear_silk|quarry_clear_fortune|void|pump|pump_clear|liquid>".to_owned();
    };
    let Some(kind) = CardKind::parse(name) else {
        return format!(
            "unknown card type '{name}'; try one of: shape, {}",
            Quarry::ALL
                .iter()
                .map(|quarry| quarry.suffix())
                .collect::<Vec<_>>()
                .join(", ")
        );
    };
    with_held_card(host, |data| {
        data.card.kind = kind;
        match kind {
            CardKind::Shape => "Card type set to shape (places blocks)".to_owned(),
            CardKind::Quarry(quarry) => format!(
                "Card type set to {} ({}{})",
                quarry.suffix(),
                quarry.description(),
                if quarry.clearing { ", clearing" } else { "" }
            ),
            CardKind::Pump(pump) => format!(
                "Card type set to {} ({}{})",
                pump.suffix(),
                pump.description(),
                if pump.clearing { ", clearing" } else { "" }
            ),
            CardKind::Liquid => "Card type set to liquid (Place liquids)".to_owned(),
        }
    })
    .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned())
}

/// `/rftoolsbuilder void <material>`: add a material to the card's void list (`none` clears
/// it, `all` adds every material the jar knows).
fn write_void(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> String {
    let Some(CommandArgumentValue::String(material)) = event.command.argument("material") else {
        return "usage: /rftoolsbuilder void <stone|cobble|dirt|sand|gravel|netherrack|endstone|none|all>".to_owned();
    };
    let material = material.trim().to_ascii_lowercase();
    if material == "none" {
        return with_held_card(host, |data| {
            data.card.voiding = Voiding::empty();
            "Card void list cleared".to_owned()
        })
        .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned());
    }
    if material == "all" {
        return with_held_card(host, |data| {
            data.card.voiding = runtime::VOID_MATERIALS
                .iter()
                .fold(Voiding::empty(), |void, material| void.with(material));
            "Card voids every material: stone, cobble, dirt, sand, gravel, netherrack, endstone"
                .to_owned()
        })
        .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned());
    }
    if !runtime::VOID_MATERIALS.contains(&material.as_str()) {
        return format!(
            "unknown material '{material}'; try one of: {}",
            runtime::VOID_MATERIALS.join(", ")
        );
    }
    with_held_card(host, |data| {
        data.card.voiding = data.card.voiding.with(&material);
        format!("Card voids {material}")
    })
    .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned())
}

/// `/rftoolsbuilder tagmatching <on|off>`: `ShapeCardData.tagMatching`.
fn write_tag_matching(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> String {
    let mode = match event.command.argument("mode") {
        Some(CommandArgumentValue::String(word)) if word.eq_ignore_ascii_case("on") => true,
        Some(CommandArgumentValue::String(word)) if word.eq_ignore_ascii_case("off") => false,
        _ => return "usage: /rftoolsbuilder tagmatching <on|off>".to_owned(),
    };
    with_held_card(host, |data| {
        data.card.tag_matching = mode;
        format!("Card tag matching {}", if mode { "on" } else { "off" })
    })
    .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned())
}

/// `/rftoolsbuilder filter <block|tag> <allow|deny>`: author the filter module's data on the held
/// card. A value starting with `#` names a block tag, anything else a block id. `allow` is the
/// jar's `blacklist = false` (only the value passes), `deny` its `blacklist = true` (the value is
/// skipped). `none` clears the filter (the jar's empty slot 1).
fn write_filter(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> String {
    // `/rftoolsbuilder filter none` (no `value` argument) clears the filter. A real value is a
    // resource id (`minecraft:stone`) or a `#tag`; `word` cannot carry the `:`.
    let value = match event.command.argument("value") {
        None => {
            return with_held_card(host, |data| {
                data.filter = Filter::default();
                "Card filter cleared".to_owned()
            })
            .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned());
        }
        Some(CommandArgumentValue::Id(id)) => id.to_string(),
        _ => return "usage: /rftoolsbuilder filter <block|tag> <allow|deny>".to_owned(),
    };
    let Some(CommandArgumentValue::String(word)) = event.command.argument("mode") else {
        return "usage: /rftoolsbuilder filter <block|tag> <allow|deny>".to_owned();
    };
    let Some(mode) = FilterMode::parse(word) else {
        return format!("unknown filter mode '{word}'; try allow or deny");
    };
    with_held_card(host, |data| {
        if value.eq_ignore_ascii_case("none") {
            data.filter = Filter::default();
            "Card filter cleared".to_owned()
        } else {
            data.filter = Filter {
                mode,
                value: value.clone(),
            };
            format!("Card filters {} blocks ({})", value, mode.as_str())
        }
    })
    .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned())
}

/// `/rftoolsbuilder replace <pos> <block>`: the Builder's replacement block
/// (`BuilderConfiguration.quarryReplace`, kept per Builder — `notes.md`).
fn write_replace(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> String {
    let Some(CommandArgumentValue::BlockPos(pos)) = event.command.argument("pos") else {
        return "usage: /rftoolsbuilder replace <x> <y> <z> <block>".to_owned();
    };
    let Some(CommandArgumentValue::Id(block)) = event.command.argument("block") else {
        return "usage: /rftoolsbuilder replace <x> <y> <z> <block>".to_owned();
    };
    let pos = *pos;
    if !is_builder(host, pos) {
        return "not a builder".to_owned();
    }
    let Some(entity) = entity_at(host, pos) else {
        return "not a builder".to_owned();
    };
    let mut data = entity.data(&*host).unwrap_or_default();
    data.insert(
        key!(K_REPLACE),
        Tag::String(block.to_string().as_str().into()),
    );
    if entity.replace_data(host, data).is_err() {
        return "the Builder could not take the replacement block".to_owned();
    }
    let _ = entity.set_changed(host);
    format!("Builder replacement set to {block}")
}

/// `/rftoolsbuilder shape <shape> <dx> <dy> <dz> [solid|hollow]`.
fn write_shape(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> String {
    let Some(CommandArgumentValue::String(name)) = event.command.argument("shape") else {
        return "usage: /rftoolsbuilder shape <shape> <dx> <dy> <dz> [solid|hollow]".to_owned();
    };
    let Some(shape) = Shape::parse(name) else {
        return format!(
            "unknown shape '{name}'; try one of: {}",
            Shape::ALL
                .iter()
                .map(|shape| shape.id())
                .collect::<Vec<_>>()
                .join(", ")
        );
    };
    let size = ["dx", "dy", "dz"].map(|name| match event.command.argument(name) {
        Some(CommandArgumentValue::Integer(value)) => Some(*value),
        _ => None,
    });
    let [Some(dx), Some(dy), Some(dz)] = size else {
        return "usage: /rftoolsbuilder shape <shape> <dx> <dy> <dz> [solid|hollow]".to_owned();
    };
    let solid = match event.command.argument("solid") {
        None => true,
        Some(CommandArgumentValue::String(word)) if word.eq_ignore_ascii_case("solid") => true,
        Some(CommandArgumentValue::String(word)) if word.eq_ignore_ascii_case("hollow") => false,
        Some(_) => return "the last argument is 'solid' or 'hollow'".to_owned(),
    };
    let card = Card {
        shape,
        solid,
        dimension: [dx, dy, dz],
        ..Card::default()
    };
    if let Err(error) = card.validate() {
        return error.to_string();
    }
    with_held_card(host, |data| {
        data.card = card;
        format!(
            "Card set to {} ({}) {dx}x{dy}x{dz}",
            shape.description(),
            card.solid_word()
        )
    })
    .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned())
}

/// `/rftoolsbuilder offset <x> <y> <z>`.
fn write_offset(host: &mut dyn ServerHost, event: &RunModCommandEvent<'_>) -> String {
    let offset = ["x", "y", "z"].map(|name| match event.command.argument(name) {
        Some(CommandArgumentValue::Integer(value)) => Some(*value),
        _ => None,
    });
    let [Some(x), Some(y), Some(z)] = offset else {
        return "usage: /rftoolsbuilder offset <x> <y> <z>".to_owned();
    };
    with_held_card(host, |data| {
        data.card.offset = [x, y, z];
        format!("Card offset set to {x} {y} {z}")
    })
    .unwrap_or_else(|| "Hold a shape card in your main hand".to_owned())
}

/// Applies `edit` to the card the acting player holds.
fn with_held_card(
    host: &mut dyn ServerHost,
    edit: impl FnOnce(&mut CardData) -> String,
) -> Option<String> {
    let player = host.players().acting_player()?;
    let mut data = held_card(host, &player)?;
    let line = edit(&mut data);
    write_held_card(host, &player, &data).then_some(line)
}

/// The command's answer, in the player's chat.
fn feedback(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// Whether `pos` is protected from this Builder: outside the world border, or outside the
/// world's bounds. The pin has no `ServerLevel.mayInteract` helper, so the check is the part of it
/// the API can answer; the difference is recorded in `notes.md`.
fn cell_protected(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    if level.in_world_bounds(&*host, pos) == Some(false) {
        return true;
    }
    let Some([min_x, min_z, max_x, max_z]) = level.world_border(&*host) else {
        return false;
    };
    let x = f64::from(pos[0]) + 0.5;
    let z = f64::from(pos[2]) + 0.5;
    !(min_x..=max_x).contains(&x) || !(min_z..=max_z).contains(&z)
}

/// Reads a Builder out as `/rftoolsbuilder status` and the action bar say it.
fn status_at(host: &dyn Host, pos: [i32; 3]) -> String {
    let Some(entity) = entity_at(host, pos) else {
        return format!(
            "Builder at {} {} {}: no block entity",
            pos[0], pos[1], pos[2]
        );
    };
    let state = State::read(&entity.data(host).unwrap_or_default());
    // The card comes from the saved data, which every path agrees on; the container slot is this
    // tick's mirror of it (`sync_card`).
    let card = saved_card(&entity.data(host).unwrap_or_default()).map(|(_, payload)| payload.card);
    let (card, refused_card) = match card {
        Some(card) => match card.validate() {
            Ok(valid) => (Some(valid), None),
            Err(error) => (None, Some(error)),
        },
        None => (None, None),
    };
    runtime::status_line(&Status {
        at: pos,
        card,
        refused_card,
        progress: state.progress,
        refusal: state.refusal,
        complete: state.done,
        cells: state.cells,
        replace: string(&entity.data(host).unwrap_or_default(), K_REPLACE)
            .unwrap_or_else(|| QUARRY_REPLACE.to_owned()),
    })
}

/// `/rftoolsbuilder forced`: the chunks this Builder's ticket actually holds, as
/// the game reports them (`LevelRef::mod_forced_chunks`) — the authoritative
/// proof of the ticket, not just the saved copy in the block entity.
fn forced_at(host: &mut dyn ServerHost, pos: [i32; 3]) -> String {
    let owner = TicketOwner::Entity(runtime::ticket_owner(pos));
    let chunks: Vec<[i32; 2]> = host
        .level(host.dimension())
        .and_then(|level| level.mod_forced_chunks(host, ticket_controller()))
        .unwrap_or_default()
        .into_iter()
        .filter(|(ticket_owner, _, _)| *ticket_owner == owner)
        .map(|(_, chunk, _)| chunk)
        .collect();
    runtime::forced_line(pos, &chunks)
}

/// `/rftoolsbuilder fluid <pos>`: the Builder's tank, read from the `fluids` compound the
/// game's tank capability reads — an exact, parseable millibucket answer.
fn fluid_at(host: &dyn Host, pos: [i32; 3]) -> String {
    if !is_builder(host, pos) {
        return "not a builder".to_owned();
    }
    let saved = entity_at(host, pos)
        .and_then(|entity| entity.data(host))
        .unwrap_or_default();
    let (fluid, amount) = tank_of(&saved);
    fluid_line(&FluidStatus {
        at: pos,
        fluid,
        amount,
    })
}

/// `/rftoolsbuilder energy <pos>`: this Builder's FE buffer and lifetime spend, read from its
/// saved compound (the same `energy` int the capability reads) — an exact, parseable answer.
fn energy_at(host: &dyn Host, pos: [i32; 3]) -> String {
    if !is_builder(host, pos) {
        return "not a builder".to_owned();
    }
    let saved = entity_at(host, pos)
        .and_then(|entity| entity.data(host))
        .unwrap_or_default();
    let state = State::read(&saved);
    energy_line(&EnergyStatus {
        at: pos,
        stored: energy_of(&saved),
        spent: state.energy_spent,
        filter_skipped: state.filter_skipped,
    })
}

/// `/rftoolsbuilder charge <pos> <amount>`: bring a Builder's buffer **to** `amount` FE the way
/// a cable would, through the block's `ENERGY` capability and as many inserts as its
/// `max_insert` needs. A buffer already at or above the target is left alone (the jar's
/// `GenericEnergyStorage` gives nothing out). Prints the before/after from the saved compound.
fn charge_block(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> String {
    if !is_builder(host, pos) {
        return "not a builder".to_owned();
    }
    let Some(level) = host.level(host.dimension()) else {
        return format!("no level for Builder at {},{},{}", pos[0], pos[1], pos[2]);
    };
    let stored = |host: &dyn ServerHost| {
        host.block_entity(level, pos)
            .and_then(|entity| entity.data(host))
            .map_or(0, |saved| energy_of(&saved))
    };
    let before = stored(&*host);
    let wanted = u32::try_from(amount.max(0))
        .unwrap_or(u32::MAX)
        .saturating_sub(before);
    let mut inserted = 0u32;
    // One insert moves at most `ENERGY_MAX_INSERT`, so fill in bounded passes; a full buffer
    // (or one with no capability) inserts nothing and ends the loop.
    for _ in 0..64 {
        if inserted >= wanted {
            break;
        }
        let missing = wanted - inserted;
        let took = host
            .transaction(|host, tx| -> Result<u32, ()> {
                let Some(buffer) = level.capability(&mut *host, &ENERGY, pos, None) else {
                    return Ok(0);
                };
                Ok(buffer.insert(&mut *host, tx, missing))
            })
            .unwrap_or(0);
        if took == 0 {
            break;
        }
        inserted += took;
    }
    let after = stored(&*host);
    format!(
        "charged Builder at {} {} {}: {} -> {} FE (inserted {})",
        pos[0], pos[1], pos[2], before, after, inserted
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_component_round_trips_a_card() {
        let card = Card {
            shape: Shape::Sphere,
            solid: false,
            dimension: [7, 3, 5],
            offset: [2, -1, 3],
            modifier: runtime::Modifier {
                rotation: runtime::Rotation::Y,
                flip_y: true,
            },
            kind: CardKind::Quarry(Quarry {
                silk: true,
                fortune: false,
                clearing: true,
                voids: false,
            }),
            voiding: Voiding::empty().with("gravel").with("dirt"),
            tag_matching: true,
        };
        let mut data = Compound::new();
        put_card(&mut data, card);
        assert_eq!(card_of(&data), card);
        assert_eq!(string(&data, K_SHAPE).as_deref(), Some("SHAPE_SPHERE"));
        assert_eq!(ints(&data, K_DIMENSION), Some([7, 3, 5]));
        assert_eq!(ints(&data, K_OFFSET), Some([2, -1, 3]));
    }

    #[test]
    fn a_missing_or_broken_component_is_the_jars_default_card() {
        assert_eq!(card_of(&Compound::new()), Card::default());
        let mut broken = Compound::new();
        broken.insert(key!(K_SHAPE), Tag::String("SHAPE_COMPOSITION".into()));
        broken.insert(key!(K_DIMENSION), Tag::String("nope".into()));
        let card = card_of(&broken);
        assert_eq!(card.shape, Shape::Box, "an uncarried shape falls back");
        assert_eq!(card.dimension, [5, 5, 5], "a broken box falls back");
        assert!(card.solid, "the jar's default is solid");
    }

    #[test]
    fn the_selection_round_trips_and_drops_what_it_does_not_use() {
        let selection = Selection {
            mode: 2,
            target: Some(("minecraft:overworld".to_owned(), [1, 2, 3])),
            corner1: Some([4, 5, 6]),
        };
        let mut data = Compound::new();
        put_selection(&mut data, &selection);
        assert_eq!(selection_of(&data), selection);
        put_selection(&mut data, &Selection::default());
        assert!(!data.contains_key(&key!(K_SELECTED)));
        assert!(!data.contains_key(&key!(K_CORNER1)));
        assert_eq!(selection_of(&data), Selection::default());
    }

    #[test]
    fn the_two_corner_click_is_the_jars_box() {
        // Corners (0,64,0) and (4,68,4) on a Builder at (10,64,10): a 5×5×5
        // box whose centre sits at (2,66,2), so the offset is (-8,2,-8).
        let card = from_corners([0, 64, 0], [4, 68, 4], [10, 64, 10]);
        assert_eq!(card.shape, Shape::Box);
        assert!(card.solid);
        assert_eq!(card.dimension, [5, 5, 5]);
        assert_eq!(card.offset, [-8, 2, -8]);
        // A single block is 1×1×1 at the clicked corner.
        let one = from_corners([3, 70, 3], [3, 70, 3], [0, 64, 0]);
        assert_eq!(one.dimension, [1, 1, 1]);
        assert_eq!(one.offset, [3, 6, 3]);
    }

    #[test]
    fn the_corner_guards_are_the_jars_three_refusals() {
        let target = Selection {
            mode: 1,
            target: Some(("minecraft:overworld".to_owned(), [1, 2, 3])),
            corner1: None,
        };
        assert!(matches!(
            selection_guard(&target, "minecraft:overworld", [9, 9, 9]),
            Guard::Ok
        ));
        assert!(matches!(
            selection_guard(&target, "minecraft:the_nether", [9, 9, 9]),
            Guard::OtherDimension
        ));
        assert!(matches!(
            selection_guard(&target, "minecraft:overworld", [1, 2, 3]),
            Guard::Cleared
        ));
        assert!(matches!(
            selection_guard(&Selection::default(), "minecraft:overworld", [9, 9, 9]),
            Guard::NoBuilder
        ));
    }

    #[test]
    fn the_installed_card_round_trips_through_the_saved_data() {
        // The card lives in the block entity's saved compound (the container slot did not carry a
        // click-time write through to the next command at this pin), so the saved form must hold
        // everything the stack's component held: the item and the card's data.
        let payload = CardData {
            card: Card {
                shape: Shape::Sphere,
                solid: false,
                dimension: [7, 3, 5],
                offset: [2, -1, 3],
                modifier: runtime::Modifier {
                    rotation: runtime::Rotation::Y,
                    flip_y: true,
                },
                kind: CardKind::Quarry(Quarry {
                    silk: false,
                    fortune: true,
                    clearing: false,
                    voids: false,
                }),
                voiding: Voiding::empty().with("stone"),
                tag_matching: false,
            },
            selection: Selection {
                mode: 2,
                target: Some(("minecraft:overworld".to_owned(), [10, 64, 10])),
                corner1: Some([1, 2, 3]),
            },
            filter: Filter {
                mode: FilterMode::Deny,
                value: "#c:stones".to_owned(),
            },
        };
        let mut data = Compound::new();
        write_saved_card(&mut data, Some(("rftoolsbuilder:shape_card_def", &payload)));
        let (item, read) = saved_card(&data).expect("the saved card comes back");
        assert_eq!(item, "rftoolsbuilder:shape_card_def");
        assert_eq!(
            read, payload,
            "shape, size, offset, turn and selection survive"
        );
        // An emptied slot leaves nothing behind.
        write_saved_card(&mut data, None);
        assert!(saved_card(&data).is_none());
    }

    #[test]
    fn a_builders_state_round_trips_through_its_compound() {
        let state = State {
            card: Some("SHAPE_BOX-true-5x5x5-0,0,0-0".to_owned()),
            cursor: Some([4, 65, -2]),
            progress: Progress {
                placed: 12,
                skipped: 40,
                voided: 2,
            },
            done: false,
            cells: Some(runtime::CellCount {
                cells: 125,
                more: false,
            }),
            refusal: Some(Refusal {
                at: [4, 66, -2],
                reason: RefusalReason::NoMaterial,
            }),
            forced_chunk: Some([3, -1]),
            energy_spent: 8_100,
            filter_skipped: 300,
        };
        let mut nbt = Compound::new();
        state.write(&mut nbt);
        assert_eq!(State::read(&nbt), state);
        // A finished pass drops the cursor and the refusal, and says so.
        let done = State {
            cursor: None,
            done: true,
            refusal: None,
            ..state.clone()
        };
        done.write(&mut nbt);
        assert_eq!(State::read(&nbt), done);
        assert!(!nbt.contains_key(&key!(K_CURSOR)));
        assert!(!nbt.contains_key(&key!(K_REFUSAL_POS)));
        assert_eq!(nbt.get(&key!(K_REFUSAL)), Some(&Tag::Byte(REFUSAL_NONE)));
    }

    #[test]
    fn a_buffer_pays_only_what_it_holds() {
        let mut nbt = Compound::new();
        set_energy(&mut nbt, 8_100);
        assert_eq!(energy_of(&nbt), 8_100);
        let mut spent = 0;
        assert!(spend(&mut nbt, &mut spent, cell_cost(CellCharge::Quarry)));
        assert_eq!(energy_of(&nbt), 7_800);
        assert_eq!(spent, 300);
        // An empty buffer cannot pay a cell and leaves both the buffer and the spend untouched.
        set_energy(&mut nbt, 50);
        assert!(!spend(&mut nbt, &mut spent, cell_cost(CellCharge::Quarry)));
        assert_eq!(energy_of(&nbt), 50);
        assert_eq!(spent, 300);
        // The jar's max, not an overflowing int, is what a saved compound reports.
        set_energy(&mut nbt, i32::MAX as u32);
        assert_eq!(energy_of(&nbt), ENERGY_CAPACITY as u32);
    }

    #[test]
    fn twenty_seven_quarry_cells_spend_the_exact_buffer_difference() {
        let mut nbt = Compound::new();
        set_energy(&mut nbt, 20_000);
        let mut spent = 0;
        for _ in 0..27 {
            assert!(spend(&mut nbt, &mut spent, cell_cost(CellCharge::Quarry)));
        }
        assert_eq!(spent, 8_100);
        assert_eq!(energy_of(&nbt), 11_900);
        // A void card over the same 27 cells spends builderRfPerOperation × 0.5, not the
        // jar's void-list or truncating-supplier readings.
        set_energy(&mut nbt, 20_000);
        spent = 0;
        for _ in 0..27 {
            assert!(spend(&mut nbt, &mut spent, cell_cost(CellCharge::Void)));
        }
        assert_eq!(spent, 6_750);
    }

    #[test]
    fn a_pass_belongs_to_one_card() {
        let card = Card {
            shape: Shape::Sphere,
            ..Card::default()
        };
        assert_eq!(
            fingerprint(card, "minecraft:dirt"),
            fingerprint(card, "minecraft:dirt")
        );
        assert_eq!(
            fingerprint(card, "minecraft:dirt"),
            "SHAPE_SPHERE-true-5x5x5-0,0,0-0-shape--false-minecraft:dirt",
            "the note the saved compound holds"
        );
        // What a quarry's pass depends on beyond its geometry: the type, the void list, the
        // tag toggle and the Builder's replacement block.
        assert_ne!(
            fingerprint(card, "minecraft:dirt"),
            fingerprint(card, "minecraft:cobblestone"),
            "the replacement block is part of the pass"
        );
        for other in [
            Card {
                dimension: [7, 7, 7],
                ..card
            },
            Card {
                offset: [1, 0, 0],
                ..card
            },
            Card {
                solid: false,
                ..card
            },
            Card {
                shape: Shape::Box,
                ..card
            },
            Card {
                modifier: runtime::Modifier {
                    rotation: runtime::Rotation::Y,
                    flip_y: false,
                },
                ..card
            },
            Card {
                kind: CardKind::Quarry(Quarry {
                    silk: false,
                    fortune: false,
                    clearing: false,
                    voids: false,
                }),
                ..card
            },
            Card {
                voiding: Voiding::empty().with("stone"),
                ..card
            },
            Card {
                tag_matching: true,
                ..card
            },
        ] {
            assert_ne!(
                fingerprint(card, "minecraft:dirt"),
                fingerprint(other, "minecraft:dirt"),
                "{other:?}"
            );
        }
    }

    #[test]
    fn the_card_component_round_trips_the_quarry_type_and_void_list() {
        // A card that loses its type or its void list when it is moved is the defect this
        // pins: the component, the saved data and the stack a Builder hands back all carry
        // them.
        let cards = [
            Card {
                kind: CardKind::Quarry(Quarry {
                    silk: false,
                    fortune: false,
                    clearing: false,
                    voids: false,
                }),
                voiding: Voiding::empty().with("gravel").with("dirt"),
                tag_matching: true,
                ..Card::default()
            },
            Card {
                kind: CardKind::Quarry(Quarry {
                    silk: true,
                    fortune: false,
                    clearing: true,
                    voids: false,
                }),
                ..Card::default()
            },
            Card {
                kind: CardKind::Quarry(Quarry {
                    silk: false,
                    fortune: true,
                    clearing: true,
                    voids: false,
                }),
                voiding: Voiding::empty().with("endstone"),
                ..Card::default()
            },
        ];
        for card in cards {
            let mut data = Compound::new();
            put_card(&mut data, card);
            assert_eq!(card_of(&data), card, "{card:?} survives its component");
            // The jar's own field names and shapes: a string type and a string list.
            assert_eq!(
                string(&data, K_TYPE).as_deref(),
                Some(card.kind.suffix()),
                "the type is the card type's resource suffix"
            );
            match data.get(&key!(K_VOIDING)) {
                Some(Tag::List(values)) => {
                    let names: Vec<String> = values
                        .iter()
                        .filter_map(|tag| match tag {
                            Tag::String(text) => Some(text.to_string_lossy()),
                            _ => None,
                        })
                        .collect();
                    assert_eq!(names.join(", "), card.voiding.names());
                }
                other => panic!("voiding is a string list, got {other:?}"),
            }
        }
        // A card with no type at all is the jar's `shape_card_def`, and an unknown type does
        // not grow a behaviour this slice does not carry.
        let mut bare = Compound::new();
        put_card(&mut bare, Card::default());
        assert_eq!(card_of(&bare).kind, CardKind::Shape);
        let mut unknown = Compound::new();
        put_card(&mut unknown, Card::default());
        unknown.insert(key!(K_TYPE), Tag::String("mover".into()));
        assert_eq!(card_of(&unknown).kind, CardKind::Shape);
        let mut broken = Compound::new();
        broken.insert(key!(K_VOIDING), Tag::String("not a list".into()));
        assert!(card_of(&broken).voiding.is_empty());
    }

    #[test]
    fn the_component_is_registered_under_the_manifest_namespace() {
        assert_eq!(CardTag.id(), "rftoolsbuilder:shape_card");
        assert_eq!(COMPONENT, "rftoolsbuilder:shape_card");
        assert_eq!(runtime::SHAPE_CARD, "rftoolsbuilder:shape_card_def");
        assert_eq!(runtime::BUILDER, "rftoolsbuilder:builder");
    }
}
