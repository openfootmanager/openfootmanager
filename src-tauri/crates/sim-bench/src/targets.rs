//! Real-football calibration bands, in one table, each with the source it came from.
//!
//! The bands used to be typed out twice, once in the terminal report and once in the HTML
//! report, and the two had drifted: each showed a metric the other did not. No band recorded a
//! source, and the five that could be checked against real results turned out wrong, most
//! visibly home and away clean sheets sharing one band when real football splits them.
//!
//! Five scoreline bands are now measured. `scripts/calibrate-scorelines.mjs` derives them from
//! openfootball's results at a pinned commit and writes `data/big5-scorelines.json`; this module
//! reads that file, so the numbers exist in one place. Every other band is `Unsourced`: shown for
//! orientation and never a calibration target, because tuning the engine to satisfy a number
//! nobody can point at turns a guess into engine behaviour.

use serde::{Deserialize, Serialize};

use crate::stats::BenchStats;

const SCORELINE_ARTIFACT: &str = include_str!("../data/big5-scorelines.json");

/// The scoreline bands describe a spread across real league-seasons; the bench replays one
/// fixture between two teams built once, so its average is a different quantity.
const ONE_FIXTURE_IS_NOT_A_LEAGUE: &str = "measured across real league-seasons; the bench \
     replays one fixture, so a value outside the band does not mean the engine is off";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    PerGame,
    Percent,
}

impl Unit {
    pub fn value(self, value: f64) -> String {
        match self {
            Unit::PerGame => format!("{value:.2}"),
            Unit::Percent => format!("{value:.2}%"),
        }
    }

    pub fn band(self, low: f64, high: f64) -> String {
        match self {
            Unit::PerGame => format!("{low:.2}–{high:.2}"),
            Unit::Percent => format!("{low:.2}–{high:.2}%"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Provenance {
    Measured {
        source: String,
        competitions: String,
        seasons: String,
        /// The pinned commit's date, not the day the script ran, so the artifact regenerates
        /// byte for byte.
        retrieved: String,
    },
    /// A published league-wide figure, cited rather than derived from committed data. Shown for
    /// orientation, like an unsourced band: it describes league seasons, not the bench's fixture.
    Cited {
        source: String,
    },
    Unsourced,
}

pub struct Target {
    pub label: &'static str,
    pub unit: Unit,
    pub low: f64,
    pub high: f64,
    pub read: fn(&BenchStats) -> f64,
    pub provenance: Provenance,
    pub not_comparable: Option<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Verdict {
    pub label: &'static str,
    pub unit: Unit,
    pub value: f64,
    pub low: f64,
    pub high: f64,
    pub inside: bool,
    pub provenance: Provenance,
    pub not_comparable: Option<&'static str>,
}

impl Verdict {
    /// Whether the band can say the engine is right or wrong here: measured, and measuring what
    /// the bench plays. Anything else is shown for orientation only.
    pub fn is_judged(&self) -> bool {
        matches!(self.provenance, Provenance::Measured { .. }) && self.not_comparable.is_none()
    }
}

#[derive(Deserialize)]
struct Artifact {
    source: ArtifactSource,
    population: ArtifactPopulation,
    metrics: Vec<ArtifactBand>,
}

#[derive(Deserialize)]
struct ArtifactSource {
    name: String,
    licence: String,
    commit: String,
    commit_date: String,
}

#[derive(Deserialize)]
struct ArtifactPopulation {
    competitions: Vec<String>,
    seasons: Vec<String>,
}

#[derive(Deserialize)]
struct ArtifactBand {
    key: String,
    low: f64,
    high: f64,
}

struct MeasuredBands {
    provenance: Provenance,
    bands: Vec<ArtifactBand>,
}

impl MeasuredBands {
    fn load() -> Result<Self, String> {
        let artifact: Artifact = serde_json::from_str(SCORELINE_ARTIFACT)
            .map_err(|error| format!("data/big5-scorelines.json is unreadable: {error}"))?;
        let commit = artifact
            .source
            .commit
            .get(..8)
            .unwrap_or(&artifact.source.commit);
        let seasons = match (
            artifact.population.seasons.first(),
            artifact.population.seasons.last(),
        ) {
            (Some(first), Some(last)) => format!(
                "{first} to {last} ({} seasons)",
                artifact.population.seasons.len()
            ),
            _ => String::new(),
        };
        let provenance = Provenance::Measured {
            source: format!(
                "{} @ {commit} ({})",
                artifact.source.name, artifact.source.licence
            ),
            competitions: artifact.population.competitions.join(", "),
            seasons,
            retrieved: artifact
                .source
                .commit_date
                .get(..10)
                .unwrap_or(&artifact.source.commit_date)
                .to_string(),
        };
        Ok(Self {
            provenance,
            bands: artifact.metrics,
        })
    }

    fn target(
        &self,
        key: &str,
        label: &'static str,
        unit: Unit,
        read: fn(&BenchStats) -> f64,
    ) -> Result<Target, String> {
        let band = self
            .bands
            .iter()
            .find(|band| band.key == key)
            .ok_or_else(|| format!("data/big5-scorelines.json has no {key} band"))?;
        Ok(Target {
            label,
            unit,
            low: band.low,
            high: band.high,
            read,
            provenance: self.provenance.clone(),
            not_comparable: Some(ONE_FIXTURE_IS_NOT_A_LEAGUE),
        })
    }
}

/// The Big Five's range of cards a match: England, Germany, France, Italy and Spain.
const CIES_CARDS: &str = "CIES Football Observatory Monthly Report n°57 (Poli, Ravenel & Besson, Sep 2020): Big Five, 2015/16-2019/20";

fn cited(
    label: &'static str,
    unit: Unit,
    low: f64,
    high: f64,
    read: fn(&BenchStats) -> f64,
    source: &str,
) -> Target {
    Target {
        label,
        unit,
        low,
        high,
        read,
        provenance: Provenance::Cited {
            source: source.to_string(),
        },
        not_comparable: None,
    }
}

fn unsourced(
    label: &'static str,
    unit: Unit,
    low: f64,
    high: f64,
    read: fn(&BenchStats) -> f64,
) -> Target {
    Target {
        label,
        unit,
        low,
        high,
        read,
        provenance: Provenance::Unsourced,
        not_comparable: None,
    }
}

/// The calibration table, in the order the reports print it.
pub fn all() -> Result<Vec<Target>, String> {
    let measured = MeasuredBands::load()?;
    Ok(vec![
        measured.target(
            "goals_per_game",
            "Goals/game",
            Unit::PerGame,
            BenchStats::gpg,
        )?,
        measured.target(
            "clean_sheets_home_pct",
            "Clean sheets (home)",
            Unit::Percent,
            BenchStats::clean_sheet_home_pct,
        )?,
        measured.target(
            "clean_sheets_away_pct",
            "Clean sheets (away)",
            Unit::Percent,
            BenchStats::clean_sheet_away_pct,
        )?,
        measured.target(
            "both_teams_scored_pct",
            "Both teams scored",
            Unit::Percent,
            BenchStats::btts_pct,
        )?,
        measured.target(
            "home_win_pct",
            "Home win %",
            Unit::Percent,
            BenchStats::home_win_pct,
        )?,
        unsourced(
            "Shots/game",
            Unit::PerGame,
            18.0,
            32.0,
            BenchStats::shots_pg,
        ),
        unsourced(
            "Shots on target %",
            Unit::Percent,
            32.0,
            45.0,
            BenchStats::shot_accuracy_pct,
        ),
        unsourced(
            "Goal conversion %",
            Unit::Percent,
            20.0,
            40.0,
            BenchStats::goal_conversion_pct,
        ),
        cited(
            "Yellow cards/game",
            Unit::PerGame,
            3.39,
            5.27,
            BenchStats::yellows_pg,
            CIES_CARDS,
        ),
        cited(
            "Red cards/game",
            Unit::PerGame,
            0.12,
            0.27,
            BenchStats::reds_pg,
            CIES_CARDS,
        ),
        unsourced(
            "Fouls/game",
            Unit::PerGame,
            18.0,
            28.0,
            BenchStats::fouls_pg,
        ),
        unsourced(
            "Penalties/game",
            Unit::PerGame,
            0.20,
            0.50,
            BenchStats::penalties_pg,
        ),
        unsourced(
            "Penalty conversion %",
            Unit::Percent,
            65.0,
            85.0,
            BenchStats::penalty_conversion_pct,
        ),
        unsourced(
            "Corners/game",
            Unit::PerGame,
            8.0,
            14.0,
            BenchStats::corners_pg,
        ),
        unsourced(
            "Goal kicks/game",
            Unit::PerGame,
            8.0,
            14.0,
            BenchStats::goal_kicks_pg,
        ),
        unsourced(
            "Crosses/game",
            Unit::PerGame,
            15.0,
            30.0,
            BenchStats::crosses_pg,
        ),
        unsourced(
            "Open play goals %",
            Unit::Percent,
            60.0,
            75.0,
            BenchStats::open_play_goal_pct,
        ),
        unsourced(
            "Corner goals %",
            Unit::Percent,
            10.0,
            20.0,
            BenchStats::corner_goal_pct,
        ),
        unsourced(
            "Free kick goals %",
            Unit::Percent,
            5.0,
            15.0,
            BenchStats::free_kick_goal_pct,
        ),
        unsourced(
            "Penalty goals %",
            Unit::Percent,
            5.0,
            15.0,
            BenchStats::penalty_goal_pct,
        ),
    ])
}

/// Both edges of a band count as inside it.
pub fn verdict(target: &Target, stats: &BenchStats) -> Verdict {
    let value = (target.read)(stats);
    Verdict {
        label: target.label,
        unit: target.unit,
        value,
        low: target.low,
        high: target.high,
        inside: (target.low..=target.high).contains(&value),
        provenance: target.provenance.clone(),
        not_comparable: target.not_comparable,
    }
}

/// Fails only if the compiled-in scoreline artifact cannot be read or lacks a band.
pub fn evaluate(stats: &BenchStats) -> Result<Vec<Verdict>, String> {
    Ok(all()?.iter().map(|target| verdict(target, stats)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Vec<Target> {
        all().expect("the compiled-in scoreline artifact loads")
    }

    /// Given the artifact the derivation script writes,
    /// When the table is built,
    /// Then its five scoreline bands are Measured and carry the artifact's numbers.
    #[test]
    fn the_scoreline_bands_come_from_the_artifact() {
        let artifact: serde_json::Value = serde_json::from_str(SCORELINE_ARTIFACT).unwrap();
        let goals = artifact["metrics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|metric| metric["key"] == "goals_per_game")
            .unwrap();
        let table = table();
        let measured: Vec<&Target> = table
            .iter()
            .filter(|target| matches!(target.provenance, Provenance::Measured { .. }))
            .collect();

        assert_eq!(measured.len(), 5);
        let goals_target = table.iter().find(|t| t.label == "Goals/game").unwrap();
        assert_eq!(goals_target.low, goals["low"].as_f64().unwrap());
        assert_eq!(goals_target.high, goals["high"].as_f64().unwrap());
    }

    /// Given every band in the table,
    /// When their edges are compared,
    /// Then each is a real interval and no label appears twice.
    #[test]
    fn every_band_is_an_interval_and_every_label_is_unique() {
        let table = table();
        for target in &table {
            assert!(
                target.low < target.high,
                "{} has an empty band",
                target.label
            );
        }
        let mut labels: Vec<&str> = table.iter().map(|target| target.label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), table.len());
    }

    /// Given a Measured band,
    /// When its provenance is read,
    /// Then every field can be followed back to a source; an empty one would make an unchecked
    /// number look checked.
    #[test]
    fn a_measured_band_can_be_followed_back_to_its_source() {
        for target in table() {
            if let Provenance::Measured {
                source,
                competitions,
                seasons,
                retrieved,
            } = &target.provenance
            {
                for field in [source, competitions, seasons, retrieved] {
                    assert!(
                        !field.is_empty(),
                        "{} has an empty provenance field",
                        target.label
                    );
                }
                assert!(
                    target.not_comparable.is_some(),
                    "{} needs its caveat",
                    target.label
                );
            }
        }
    }

    /// Given a rate and a percentage,
    /// When they and their bands are printed,
    /// Then both read to two decimals, so a value just past a measured edge never prints equal
    /// to it.
    #[test]
    fn a_rate_and_a_percentage_read_to_two_decimals() {
        assert_eq!(Unit::PerGame.value(2.982), "2.98");
        assert_eq!(Unit::PerGame.band(2.5547, 3.1232), "2.55–3.12");
        assert_eq!(Unit::Percent.value(59.1968), "59.20%");
        assert_eq!(Unit::Percent.band(48.3158, 59.183), "48.32–59.18%");
    }

    /// Given a finished run,
    /// When it is evaluated,
    /// Then every band in the table gets a verdict, in the table's order.
    #[test]
    fn every_band_in_the_table_gets_a_verdict() {
        let stats = BenchStats {
            games: 100,
            ..BenchStats::default()
        };
        let labels: Vec<&str> = evaluate(&stats)
            .unwrap()
            .iter()
            .map(|verdict| verdict.label)
            .collect();
        let expected: Vec<&str> = table().iter().map(|target| target.label).collect();
        assert_eq!(labels, expected);
    }

    fn verdict_with(provenance: Provenance, not_comparable: Option<&'static str>) -> Verdict {
        Verdict {
            label: "Goals/game",
            unit: Unit::PerGame,
            value: 2.8,
            low: 2.5,
            high: 3.1,
            inside: true,
            provenance,
            not_comparable,
        }
    }

    /// Given a measured band the bench plays, a measured band it does not, a cited one and an
    /// unsourced one,
    /// When each is asked whether it is judged,
    /// Then only the first is.
    #[test]
    fn only_a_measured_band_the_bench_plays_is_judged() {
        let measured = Provenance::Measured {
            source: "s".into(),
            competitions: "c".into(),
            seasons: "s".into(),
            retrieved: "r".into(),
        };
        assert!(verdict_with(measured.clone(), None).is_judged());
        assert!(!verdict_with(measured, Some(ONE_FIXTURE_IS_NOT_A_LEAGUE)).is_judged());
        let cited = Provenance::Cited { source: "s".into() };
        assert!(!verdict_with(cited, None).is_judged());
        assert!(!verdict_with(Provenance::Unsourced, None).is_judged());
    }

    /// Given the calibration table,
    /// When its card bands are read,
    /// Then they are the Big Five's range from CIES n°57 (3.39 to 5.27 yellows, 0.12 to 0.27
    /// reds a match), cited rather than unsourced, and shown for orientation, not judged.
    #[test]
    fn the_card_bands_are_the_big_fives_cited_range() {
        let table = all().expect("the calibration table loads");
        let band = |label: &str| {
            let target = table
                .iter()
                .find(|target| target.label == label)
                .expect("the band exists");
            let cited = matches!(target.provenance, Provenance::Cited { .. });
            (target.low, target.high, cited)
        };
        assert_eq!(band("Yellow cards/game"), (3.39, 5.27, true));
        assert_eq!(band("Red cards/game"), (0.12, 0.27, true));
    }

    /// Given the calibration table,
    /// When its cited card bands are read,
    /// Then they carry the one-fixture caveat, as the measured bands do: CIES's figures are
    /// league-season averages, and the bench replays one fixture.
    #[test]
    fn a_cited_band_carries_the_one_fixture_caveat() {
        let table = all().expect("the calibration table loads");
        for label in ["Yellow cards/game", "Red cards/game"] {
            let target = table
                .iter()
                .find(|target| target.label == label)
                .expect("the band exists");
            assert_eq!(
                target.not_comparable,
                Some(ONE_FIXTURE_IS_NOT_A_LEAGUE),
                "{label}"
            );
        }
    }

    /// Given a value exactly on a band's edge, and one just outside it,
    /// When each is judged,
    /// Then the edge counts as inside and the other does not.
    #[test]
    fn a_value_on_the_edge_of_a_band_is_inside_it() {
        let target = unsourced(
            "Shots/game",
            Unit::PerGame,
            18.0,
            32.0,
            BenchStats::shots_pg,
        );
        let on_the_edge = BenchStats {
            games: 10,
            total_shots: 320,
            ..BenchStats::default()
        };
        let just_outside = BenchStats {
            games: 10,
            total_shots: 321,
            ..BenchStats::default()
        };
        assert!(verdict(&target, &on_the_edge).inside);
        assert!(!verdict(&target, &just_outside).inside);
    }

    /// Given the measured goals band, whose lower edge is the real 10th percentile, 2.55473…,
    /// When a run averages 2.5547 goals a game, just below that edge,
    /// Then it is outside the band: the band keeps the percentile's full precision, so rounding
    /// it for storage cannot move a value from outside to inside.
    #[test]
    fn a_value_just_below_a_measured_edge_is_outside_it() {
        let table = table();
        let goals = table
            .iter()
            .find(|target| target.label == "Goals/game")
            .unwrap();
        let just_below = BenchStats {
            games: 10_000,
            total_goals: 25_547,
            ..BenchStats::default()
        };

        assert!(!verdict(goals, &just_below).inside);
    }
}
