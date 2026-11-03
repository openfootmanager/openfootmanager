use ofm_core::turn::{
    NotableUpset, RoundResultSummary, RoundSummary, StandingDelta, TopScorerDelta,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundSummaryDto {
    pub matchday: u32,
    pub is_complete: bool,
    pub pending_fixture_count: u32,
    pub completed_results: Vec<RoundResultSummary>,
    pub standings_delta: Vec<StandingDelta>,
    pub notable_upset: Option<NotableUpset>,
    pub top_scorer_delta: Vec<TopScorerDelta>,
}

impl From<RoundSummary> for RoundSummaryDto {
    fn from(value: RoundSummary) -> Self {
        Self {
            matchday: value.matchday,
            is_complete: value.is_complete,
            pending_fixture_count: value.pending_fixture_count,
            completed_results: value.completed_results,
            standings_delta: value.standings_delta,
            notable_upset: value.notable_upset,
            top_scorer_delta: value.top_scorer_delta,
        }
    }
}

pub fn build_round_summary_dto(
    game: &ofm_core::game::Game,
    round: &ofm_core::matchday::LeagueRoundContext,
) -> Option<RoundSummaryDto> {
    ofm_core::turn::build_round_summary(game, round).map(Into::into)
}
