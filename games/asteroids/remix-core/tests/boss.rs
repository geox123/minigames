//! The boss through the seam. Reaching one by honest play means clearing a whole
//! system of waves first, which a scripted test cannot practically stage, so the fight
//! itself — ignition, the armoured hull, the phases, felling and the system advance —
//! is driven directly in the crate's white-box tests. Here we only pin the facts the
//! opening of a run exposes.

mod common;

use common::game;

#[test]
fn a_run_opens_in_the_first_system_with_no_boss() {
    let game = game(1);
    assert_eq!(game.stage(), 1, "the run opens on the first system");
    assert!(game.boss().is_none(), "with no boss yet on the field");
}
