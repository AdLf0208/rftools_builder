//! RFTools Builder for Horizon — the Builder block and its shape cards (slice 1).
//!
//! A **shape card** (`rftoolsbuilder:shape_card_def`) carries a shape, a size,
//! an offset and whether the shape is solid or hollow, in the
//! `rftoolsbuilder:shape_card` item component. A **Builder**
//! (`rftoolsbuilder:builder`) with a card in its card slot walks that shape from
//! the top down and places a block into every cell that can take one, taking the
//! block from a container directly above it (or, failing that, below), at the
//! jar's eight cells a tick.
//!
//! The card is authored either in the world — sneak-right-click a Builder with
//! it, then click two corners, as upstream's `ShapeCardItem.useOn` does — or by
//! `/rftoolsbuilder shape` for a shape the corners cannot express.
//! `/rftoolsbuilder status` reads a Builder's shape, parameters and progress.
//!
//! [`runtime`] is the pure geometry (the jar's nine shape formulas, the solid
//! and hollow cells, the clamps and the walk) and [`support`] is the game glue.
//! See `notes.md` for the slice boundary, the deliberate deviations and the
//! gaps.
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
