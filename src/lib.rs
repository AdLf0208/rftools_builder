//! RFTools Builder for Horizon — the Builder block, its shape cards, the quarry
//! card types and the pump/liquid cards.
//!
//! A **shape card** (`rftoolsbuilder:shape_card_def`) carries a card type, a shape,
//! a size, an offset and whether the shape is solid or hollow, in the
//! `rftoolsbuilder:shape_card` item component. A **Builder**
//! (`rftoolsbuilder:builder`) with a card in its card slot walks the card's box from
//! the top down at the jar's eight cells a tick: a shape card places a block into
//! every cell that can take one, taking it from a container directly above (or,
//! failing that, below); a **quarry card** breaks the block in each cell, collects
//! its drops there and leaves the Builder's replacement block behind, or air for a
//! clearing card, while the card's void list removes blocks without collecting them.
//!
//! The card is authored either in the world — sneak-right-click a Builder with
//! it, then click two corners, as upstream's `ShapeCardItem.useOn` does — or by
//! `/rftoolsbuilder shape|offset|type|void|tagmatching|replace` for what the corners
//! cannot express. `/rftoolsbuilder status` reads a Builder's card, replacement and
//! progress.
//!
//! [`runtime`] is the pure geometry and the quarry rule (the jar's ten shape
//! formulas, the solid and hollow cells, the clamps, the walk, the card type's
//! decision and the readout text) and [`support`] is the game glue. See `notes.md`
//! for the slice boundary, the deliberate deviations and the gaps.
pub mod runtime;
mod support;

use mod_api::{
    self, ComponentTypesEvent, Flow, Hooks, HostSide, RegisterCommandsEvent, Registrar,
    RegistrationContext, RightClickBlockEvent, RunModCommandEvent, ServerHost,
};

/// The mod holds no state of its own: a Builder's progress is its block
/// entity's saved compound and its card is the container's slot.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the block-entity type, its Part, the Builder's card slot and the
    /// card item's Part.
    fn register(&self, registrar: &mut dyn Registrar, cx: &RegistrationContext) {
        support::register(registrar, cx);
    }

    /// The `rftoolsbuilder:shape_card` component, registered before content (and
    /// so every card stack's codec) loads.
    fn component_types(&self, _cx: &RegistrationContext, event: &mut ComponentTypesEvent<'_>) {
        event.types.push(support::COMPONENT.to_owned());
    }

    /// A right click on a Builder with a card in hand puts the card in its slot.
    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        support::on_right_click_block(host, event)
    }

    /// `/rftoolsbuilder shape` and `/rftoolsbuilder status`.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::on_register_commands(event);
    }

    /// The bodies of `/rftoolsbuilder shape|status`.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        support::on_run_mod_command(host, event)
    }
}
