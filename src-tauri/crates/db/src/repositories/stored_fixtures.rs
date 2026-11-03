//! Reading a stored fixture list. A list that does not parse is a failed load, never an empty list:
//! returning `[]` would let the next save overwrite a whole competition's fixtures with nothing.

use domain::league::Fixture;
use rusqlite::types::Type;

pub(super) fn parse_fixtures(column: usize, json: &str) -> rusqlite::Result<Vec<Fixture>> {
    serde_json::from_str(json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
    })
}
