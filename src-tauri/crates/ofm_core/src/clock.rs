use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct GameClock {
    pub current_date: DateTime<Utc>,
    pub start_date: DateTime<Utc>,
}

impl GameClock {
    pub fn new(start_date: DateTime<Utc>) -> Self {
        Self {
            current_date: start_date,
            start_date,
        }
    }

    pub fn advance_days(&mut self, days: i64) {
        self.current_date += Duration::days(days);
    }

    pub fn get_date(&self) -> DateTime<Utc> {
        self.current_date
    }
}
