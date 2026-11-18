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

use horizon_core::nbt::JavaString;
use mod_api::{
    self, BlockEntityEvent, BlockEntityPart, BlockEntityRef, BlockEntityTickEvent,
    BlockEntityTicksEvent, BlockKey, BlockPart, BlockUpdateFlags, Capability, Command,
    CommandArgumentValue, Component, Compound, Flow, Hand, Host, HostBlockStates, HostSide,
    HostTransaction, ITEMS, InteractionResult, ItemPart, ItemResource, ItemStack, LevelRef,
    LevelSide, ModContainer, PlayerRef, RegisterCommandsEvent, Registrar, RegistrationContext,
    ResourceHandler, RightClickBlockEvent, RunModCommandEvent, ServerHost, SoundSource, Tag, Text,
    UseOnEvent,
};

use crate::runtime::{
    self, BLOCKS_PER_TICK, Card, LOOKUPS_PER_TICK, Progress, Refusal, RefusalReason, Scan, Shape,
    Status,
};

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
/// The container's slot count. Upstream also has a filter slot (slot 1); the
/// filter module is not carried, so the slot is not declared (notes.md).
const CARD_SLOTS: usize = 1;

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

/// The card's refusal codes in the saved compound.
const REFUSAL_NONE: i8 = 0;
/// `RefusalReason::NoMaterial`.
const REFUSAL_NO_MATERIAL: i8 = 1;
/// `RefusalReason::OutOfWorld`.
const REFUSAL_OUT_OF_WORLD: i8 = 2;

/// The block key the hooks compare the loaded block against. A `BlockKey` only
/// names itself through a host, so it is asked for once, at registration.
static BLOCK_KEY: OnceLock<BlockKey> = OnceLock::new();

fn block_key() -> BlockKey {
    *BLOCK_KEY
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
    let kind = registrar
        .block_entity_type(BE_TYPE)
        .blocks(&[BLOCK])
        .build();
    registrar
        .block_entity_part(&kind, Builder)
        .expect("one Part per block-entity type");
    // The card is both put in and taken out by hand, as upstream's card slot
    // is `in().out()`.
    let slots: Vec<usize> = (0..CARD_SLOTS).collect();
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
        }
    }

    /// Writes the data into the stack's component patch; whether the stack took
    /// it.
    fn write(&self, stack: &mut ItemStack) -> bool {
        let mut data = Compound::new();
        put_card(&mut data, self.card);
        put_selection(&mut data, &self.selection);
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
            .get(&JavaString::from(K_SOLID))
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
                .get(&JavaString::from(K_FLIP_Y))
                .and_then(|tag| match tag {
                    Tag::Byte(flag) => Some(*flag != 0),
                    _ => None,
                })
                .unwrap_or(default.modifier.flip_y),
        },
    }
}

/// Writes a card into a data compound.
fn put_card(data: &mut Compound, card: Card) {
    data.insert(
        JavaString::from(K_SHAPE),
        Tag::String(card.shape.id().into()),
    );
    data.insert(JavaString::from(K_SOLID), Tag::Byte(i8::from(card.solid)));
    put_ints(data, K_DIMENSION, card.dimension);
    put_ints(data, K_OFFSET, card.offset);
    data.insert(
        JavaString::from(K_ROTATION),
        Tag::String(card.modifier.rotation.id().into()),
    );
    data.insert(
        JavaString::from(K_FLIP_Y),
        Tag::Byte(i8::from(card.modifier.flip_y)),
    );
}

/// The selection a data compound holds.
fn selection_of(data: &Compound) -> Selection {
    let target = match (string(data, K_SELECTED_DIMENSION), ints(data, K_SELECTED)) {
        (Some(dimension), Some(pos)) => Some((dimension, pos)),
        _ => None,
    };
    Selection {
        mode: data
            .get(&JavaString::from(K_MODE))
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
        JavaString::from(K_MODE),
        Tag::Byte(i8::try_from(selection.mode).unwrap_or(0)),
    );
    match &selection.target {
        Some((dimension, pos)) => {
            data.insert(
                JavaString::from(K_SELECTED_DIMENSION),
                Tag::String(dimension.as_str().into()),
            );
            put_ints(data, K_SELECTED, *pos);
        }
        None => {
            data.remove(&JavaString::from(K_SELECTED_DIMENSION));
            data.remove(&JavaString::from(K_SELECTED));
        }
    }
    match selection.corner1 {
        Some(pos) => put_ints(data, K_CORNER1, pos),
        None => {
            data.remove(&JavaString::from(K_CORNER1));
        }
    }
}

/// A compound's string field.
fn string(data: &Compound, key: &str) -> Option<String> {
    match data.get(&JavaString::from(key)) {
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
    match data.get(&JavaString::from(key)) {
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
        JavaString::from(key),
        Tag::List(vec![
            Tag::Int(value[0]),
            Tag::Int(value[1]),
            Tag::Int(value[2]),
        ]),
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
}

impl State {
    /// The state a saved compound holds.
    fn read(nbt: &Compound) -> Self {
        let number = |key: &str| match nbt.get(&JavaString::from(key)) {
            Some(Tag::Long(value)) => u64::try_from(*value).ok(),
            Some(Tag::Int(value)) => u64::try_from(*value).ok(),
            _ => None,
        };
        let refusal = match nbt.get(&JavaString::from(K_REFUSAL)) {
            Some(Tag::Byte(code)) if *code == REFUSAL_NO_MATERIAL => {
                Some(RefusalReason::NoMaterial)
            }
            Some(Tag::Byte(code)) if *code == REFUSAL_OUT_OF_WORLD => {
                Some(RefusalReason::OutOfWorld)
            }
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
            },
            done: matches!(nbt.get(&JavaString::from(K_DONE)), Some(Tag::Byte(1))),
            refusal,
        }
    }

    /// Writes the state into a block entity's compound, dropping the keys it no
    /// longer uses (a key a `merge_data` cannot delete; this writes the whole
    /// compound the block entity keeps).
    fn write(&self, nbt: &mut Compound) {
        match &self.card {
            Some(card) => {
                nbt.insert(JavaString::from(K_CARD), Tag::String(card.as_str().into()));
            }
            None => {
                nbt.remove(&JavaString::from(K_CARD));
            }
        }
        match self.cursor {
            Some(cursor) => put_ints(nbt, K_CURSOR, cursor),
            None => {
                nbt.remove(&JavaString::from(K_CURSOR));
            }
        }
        nbt.insert(
            JavaString::from(K_PLACED),
            Tag::Long(i64::try_from(self.progress.placed).unwrap_or(i64::MAX)),
        );
        nbt.insert(
            JavaString::from(K_SKIPPED),
            Tag::Long(i64::try_from(self.progress.skipped).unwrap_or(i64::MAX)),
        );
        nbt.insert(JavaString::from(K_DONE), Tag::Byte(i8::from(self.done)));
        match self.refusal {
            Some(refusal) => {
                let code = match refusal.reason {
                    RefusalReason::NoMaterial => REFUSAL_NO_MATERIAL,
                    RefusalReason::OutOfWorld => REFUSAL_OUT_OF_WORLD,
                };
                nbt.insert(JavaString::from(K_REFUSAL), Tag::Byte(code));
                put_ints(nbt, K_REFUSAL_POS, refusal.at);
            }
            None => {
                nbt.insert(JavaString::from(K_REFUSAL), Tag::Byte(REFUSAL_NONE));
                nbt.remove(&JavaString::from(K_REFUSAL_POS));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The tick
// ---------------------------------------------------------------------------

/// One server tick of a Builder: walk the shape and place a block into each
/// cell that can take one, at the jar's [`BLOCKS_PER_TICK`].
///
/// The Builder waits at a cell it has no block for (upstream's `waitMode`, its
/// default), reports `Unchanged` when there was nothing to do, and completes
/// one pass per card (upstream restarts the pass while the machine is enabled;
/// the port does one pass and says so — notes.md).
fn tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
    let pos = event.pos;
    let card = event
        .entity
        .items
        .first()
        .and_then(Option::as_ref)
        .map(CardData::read)
        .map(|data| data.card);
    let before = State::read(&event.entity.nbt);
    let mut state = before.clone();

    let Some(card) = card else {
        // No card: no scan. `Unchanged` when there was nothing to forget.
        if before != State::default() {
            state = State::default();
            state.write(&mut event.entity.nbt);
            *event.reply = mod_api::BlockEntityTickReply::Changed;
        }
        return;
    };
    let Ok(card) = card.validate() else {
        // A card past the cell limit builds nothing (upstream refuses it in the
        // GUI); the readout names it.
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

    // One pass per card (notes.md, deviation 1): a card that is not the one
    // this pass belongs to starts a fresh pass at the top of the low corner,
    // and a finished pass stays finished until the card changes.
    let fingerprint = fingerprint(card);
    if state.card.as_deref() != Some(fingerprint.as_str()) {
        state = State {
            card: Some(fingerprint),
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

    let mut placed = 0;
    let mut steps = 0;
    while placed < BLOCKS_PER_TICK && steps < LOOKUPS_PER_TICK {
        let Some(cursor) = scan.cursor else {
            break;
        };
        steps += 1;
        if card.contains(pos, cursor) {
            if !level.is_loaded(host, cursor) {
                // Nothing can be read or written there; look again next tick.
                state.refusal = Some(Refusal {
                    at: cursor,
                    reason: RefusalReason::NoMaterial,
                });
                break;
            }
            if is_replaceable(host, level, cursor) {
                match place(host, level, pos, cursor) {
                    Placed::Yes => {
                        placed += 1;
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
    if state != before {
        state.write(&mut event.entity.nbt);
        *event.reply = mod_api::BlockEntityTickReply::Changed;
    }
}

/// What identifies the card a pass belongs to: its shape, solidity, size,
/// offset and turn, so re-writing any of them starts a new pass.
fn fingerprint(card: Card) -> String {
    let [dx, dy, dz] = card.dimension;
    let [ox, oy, oz] = card.offset;
    format!(
        "{}-{}-{dx}x{dy}x{dz}-{ox},{oy},{oz}-{}{}",
        card.shape.id(),
        card.solid,
        card.modifier.rotation.id(),
        if card.modifier.flip_y { "-flip" } else { "" }
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
}

/// Places one block into `at`, taking it from the container above the Builder
/// or, failing that, below it (upstream's `createTakeableItem` order).
fn place(host: &mut dyn ServerHost, level: LevelRef, builder: [i32; 3], at: [i32; 3]) -> Placed {
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
    let occupied = entity
        .item_slot(host, u32::try_from(SLOT_CARD).unwrap_or(0))
        .ok()
        .flatten()
        .is_some();
    if occupied {
        action_bar(
            host,
            uuid,
            "Builder: a card is already in (sneak-right-click with an empty hand to take it)",
        );
        return;
    }
    let put = host
        .transaction(|host, tx| {
            entity.set_item_slot(
                host,
                tx,
                u32::try_from(SLOT_CARD).unwrap_or(0),
                Some(stack.clone()),
            )
        })
        .is_ok();
    if put {
        take_from_hand(host, uuid, &stack, 1);
        let line = status_at(host, pos);
        action_bar(host, uuid, &line);
    }
}

/// Takes the card out of the Builder into the player's inventory.
fn take_card(host: &mut dyn ServerHost, pos: [i32; 3], uuid: u128) {
    let Some(entity) = entity_at(host, pos) else {
        return;
    };
    let slot = u32::try_from(SLOT_CARD).unwrap_or(0);
    let Some(card) = entity.item_slot(host, slot).ok().flatten() else {
        action_bar(host, uuid, "Builder: no card to take");
        return;
    };
    let emptied = host
        .transaction(|host, tx| entity.set_item_slot(host, tx, slot, None))
        .is_ok();
    if !emptied {
        return;
    }
    let given = match host.player(uuid) {
        Some(player) => player.give_stack(host, card.clone()).is_ok(),
        None => false,
    };
    if !given {
        // The card stays in the Builder rather than vanishing.
        let _ = host.transaction(|host, tx| entity.set_item_slot(host, tx, slot, Some(card)));
        action_bar(host, uuid, "Builder: the card could not be given back");
        return;
    }
    // The card is out: the pass it was driving goes with it.
    let mut data = Compound::new();
    State::default().write(&mut data);
    let _ = entity.merge_data(host, data);
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
                let block = block_at(host, pos);
                feedback(host, &format!("{block}: not a builder"));
                return Flow::Cancel;
            }
            let line = status_at(host, pos);
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
        _ => Flow::Continue,
    }
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

/// The id of the block at `pos`, for a command's answer.
fn block_at(host: &dyn Host, pos: [i32; 3]) -> String {
    host.level(host.dimension())
        .and_then(|level| level.block(host, pos))
        .map_or_else(String::new, |block| block.id(host))
}

/// Reads a Builder out as `/rftoolsbuilder status` and the action bar say it.
fn status_at(host: &dyn Host, pos: [i32; 3]) -> String {
    let Some(entity) = entity_at(host, pos) else {
        return format!(
            "Builder at {} {} {}: no block entity",
            pos[0], pos[1], pos[2]
        );
    };
    let card = entity
        .item_slot(host, u32::try_from(SLOT_CARD).unwrap_or(0))
        .ok()
        .flatten()
        .map(|stack| CardData::read(&stack).card);
    let (card, refused_card) = match card {
        Some(card) => match card.validate() {
            Ok(valid) => (Some(valid), None),
            Err(error) => (None, Some(error)),
        },
        None => (None, None),
    };
    let state = State::read(&entity.data(host).unwrap_or_default());
    runtime::status_line(&Status {
        at: pos,
        card,
        refused_card,
        progress: state.progress,
        refusal: state.refusal,
        complete: state.done,
    })
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
        broken.insert(
            JavaString::from(K_SHAPE),
            Tag::String("SHAPE_COMPOSITION".into()),
        );
        broken.insert(JavaString::from(K_DIMENSION), Tag::String("nope".into()));
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
        assert!(!data.contains_key(&JavaString::from(K_SELECTED)));
        assert!(!data.contains_key(&JavaString::from(K_CORNER1)));
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
    fn a_builders_state_round_trips_through_its_compound() {
        let state = State {
            card: Some("SHAPE_BOX-true-5x5x5-0,0,0-0".to_owned()),
            cursor: Some([4, 65, -2]),
            progress: Progress {
                placed: 12,
                skipped: 40,
            },
            done: false,
            refusal: Some(Refusal {
                at: [4, 66, -2],
                reason: RefusalReason::NoMaterial,
            }),
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
        assert!(!nbt.contains_key(&JavaString::from(K_CURSOR)));
        assert!(!nbt.contains_key(&JavaString::from(K_REFUSAL_POS)));
        assert_eq!(
            nbt.get(&JavaString::from(K_REFUSAL)),
            Some(&Tag::Byte(REFUSAL_NONE))
        );
    }

    #[test]
    fn a_pass_belongs_to_one_card() {
        let card = Card {
            shape: Shape::Sphere,
            ..Card::default()
        };
        assert_eq!(fingerprint(card), fingerprint(card));
        assert_eq!(
            fingerprint(card),
            "SHAPE_SPHERE-true-5x5x5-0,0,0-0",
            "the note the saved compound holds"
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
        ] {
            assert_ne!(fingerprint(card), fingerprint(other), "{other:?}");
        }
    }

    #[test]
    fn the_component_is_registered_under_the_manifest_namespace() {
        assert_eq!(CardTag.id(), "rftoolsbuilder:shape_card");
        assert_eq!(COMPONENT, "rftoolsbuilder:shape_card");
        assert_eq!(runtime::SHAPE_CARD, "rftoolsbuilder:shape_card_def");
        assert_eq!(runtime::BUILDER, "rftoolsbuilder:builder");
    }
}
