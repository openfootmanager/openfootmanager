//! Building the evidence a bug report carries, and sending it when the player agrees to.
//!
//! The bundle is built on the player's machine (slice 3 of #569). `hub` and `http` upload it to
//! OFMHub only after explicit consent (slice 4), and `history` remembers the
//! reference codes that came back.

pub mod bundle;
pub mod history;
pub mod http;
pub mod hub;
pub mod redact;
