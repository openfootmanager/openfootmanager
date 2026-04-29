use serde::{Deserialize, Serialize};

/// A club can be seen as the real organizational unit of a football club. Later, it will be needed to move budgets and most of gestionnary part here
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Club {
    pub id: String,
    pub name: String,
    pub country: String,
    pub city: String,
    #[serde(default)]
    pub team_ids: Vec<String>,
}