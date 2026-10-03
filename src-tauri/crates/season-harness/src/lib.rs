//! Plays a generated world through consecutive seasons, by the same code paths
//! the game uses, and checks what has to stay true.
//!
//! The bugs that reach players are whole-world, multi-season bugs: promotion
//! that never runs, a season that cannot be rolled over, a fixture nobody ever
//! plays. A unit test does not see them because it builds the state it then
//! checks. This crate builds a world the way a new career does, advances it a
//! day at a time through `ofm_core::turn`, rolls each finished season over with
//! `ofm_core::end_of_season::advance_to_next_season`, and asks the rules after
//! every step.
//!
//! It drives the game and adds no game logic of its own: a rule that is a game
//! rule lives in `ofm_core` and is called from here, so the harness cannot
//! disagree with the game about what the rule is.
//!
//! * [`worlds`] builds the world a run plays in.
//! * [`driver`] is the loop, and the only place that advances the game.
//! * [`fingerprint`] reduces a world to what a replay must reproduce.
//! * [`invariants`] are the rules, held to after every day and every rollover.

pub mod driver;
pub mod fingerprint;
pub mod invariants;
pub mod worlds;
