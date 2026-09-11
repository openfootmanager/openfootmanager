//! Cheap-clone journal types live in `domain`; this module is the ofm_core façade.

pub use domain::finance::{CashJournal, TransferReservationBook};

/// Sum of posts for one club. Overflow is a bug: `post_all` rejected it.
pub fn cash_from_journal(journal: &CashJournal, club_id: &str) -> i64 {
    journal.cash_for(club_id)
}
