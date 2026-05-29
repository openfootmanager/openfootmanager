/// Formatting utilities for converting game data into agent-readable text.
///
/// Phase 2 starts with minimal structured text; Phase 5 upgrades to rich markdown.

/// Translate a backend error key (e.g. "be.error.noActiveGameSession") into
/// a human-readable message that helps agents self-correct.
pub fn translate_error(key: &str) -> String {
    match key {
        "be.error.noActiveGameSession" => "No active game session. Start or load a game first.".to_string(),
        "be.error.noTeamAssigned" => "No team assigned. Select a team first.".to_string(),
        "be.error.teamNotFound" => "Team not found. Check the team ID and try again.".to_string(),
        "be.error.playerNotFound" => "Player not found. Check the player ID and try again.".to_string(),
        "be.error.playerNotInSquad" => "Player is not in your squad.".to_string(),
        "be.error.saveManagerUnavailable" => "Save manager is unavailable. This is an internal error.".to_string(),
        "be.error.noActiveSaveSession" => "No active save session. Load or create a save first.".to_string(),
        "be.error.noActiveStatsSession" => "No active stats session.".to_string(),
        "be.error.createManager.nameRequired" => "Manager first and last name are required.".to_string(),
        "be.error.createManager.nationalityRequired" => "Manager nationality is required.".to_string(),
        "be.error.createManager.invalidDobFormat" => "Date of birth must be in YYYY-MM-DD format.".to_string(),
        "be.error.createManager.minAge" => "Manager must be at least 30 years old.".to_string(),
        "be.error.createManager.invalidDob" => "Invalid date of birth.".to_string(),
        "be.error.createManager.nameMaxLength" => "Manager name must be 30 characters or less.".to_string(),
        "be.error.createManager.startYearMin" => "Start year must be 2020 or later.".to_string(),
        "be.error.createManager.invalidStartPhase" => "Invalid start phase. Use 'seasonStart' or 'midSeason'.".to_string(),
        "be.error.createManager.historyDepthMax" => "History depth exceeds maximum.".to_string(),
        "be.error.invalidSquadRole" => "Invalid squad role. Use 'Senior' or 'Youth'.".to_string(),
        "be.error.youthAcademyOverage" => "Player is too old for youth academy (must be 21 or under).".to_string(),
        "be.error.invalidDateOfBirth" => "Invalid date of birth format.".to_string(),
        "be.error.worldReadFileFailed" => "Failed to read world file. Check the file path.".to_string(),
        _ => format!("Error: {}", key),
    }
}
