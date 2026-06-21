use crate::league::Fixture;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NationalTeam {
    pub id: String,
    pub name: String,
    pub football_nation: String,
    pub region_id: Option<String>,
    pub squad_player_ids: Vec<String>,
    pub manager_name: Option<String>,
    pub reputation: u32,
    pub fixtures: Vec<Fixture>,
    /// i18n key for the nation name (e.g. `"nations.fr"`). When set, the
    /// frontend assembles the display name via the `nations.nationalTeamTemplate`
    /// translation key rather than rendering `name` raw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_key: Option<String>,
}

impl NationalTeam {
    pub fn new(
        id: String,
        name: String,
        football_nation: String,
        region_id: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            football_nation,
            region_id,
            squad_player_ids: Vec::new(),
            manager_name: None,
            reputation: 500,
            fixtures: Vec::new(),
            name_key: None,
        }
    }
}
