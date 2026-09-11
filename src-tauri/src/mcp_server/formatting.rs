//! Formatting utilities for converting game data into agent-readable text.
//!
//! Phase 2 starts with minimal structured text; Phase 5 upgrades to rich markdown.

/// Translate a backend error key (e.g. "be.error.noActiveGameSession") into
/// a human-readable message that helps agents self-correct.
pub fn translate_error(key: &str) -> String {
    match key {
        "be.error.noActiveGameSession" => {
            "No active game session. Start or load a game first.".to_string()
        }
        "be.error.noTeamAssigned" => "No team assigned. Select a team first.".to_string(),
        "be.error.teamNotFound" => "Team not found. Check the team ID and try again.".to_string(),
        "be.error.playerNotFound" => {
            "Player not found. Check the player ID and try again.".to_string()
        }
        "be.error.playerNotInSquad" => "Player is not in your squad.".to_string(),
        "be.error.saveManagerUnavailable" => {
            "Save manager is unavailable. This is an internal error.".to_string()
        }
        "be.error.noActiveSaveSession" => {
            "No active save session. Load or create a save first.".to_string()
        }
        "be.error.noActiveStatsSession" => "No active stats session.".to_string(),
        "be.error.createManager.nameRequired" => {
            "Manager first and last name are required.".to_string()
        }
        "be.error.createManager.nationalityRequired" => {
            "Manager nationality is required.".to_string()
        }
        "be.error.createManager.invalidDobFormat" => {
            "Date of birth must be in YYYY-MM-DD format.".to_string()
        }
        "be.error.createManager.minAge" => "Manager must be at least 30 years old.".to_string(),
        "be.error.createManager.invalidDob" => "Invalid date of birth.".to_string(),
        "be.error.createManager.nameMaxLength" => {
            "Manager name must be 30 characters or less.".to_string()
        }
        "be.error.createManager.startYearMin" => "Start year must be 1900 or later.".to_string(),
        "be.error.createManager.invalidStartPhase" => {
            "Invalid start phase. Use 'seasonStart' or 'midSeason'.".to_string()
        }
        "be.error.createManager.historyDepthMax" => "History depth exceeds maximum.".to_string(),
        "be.error.invalidSquadRole" => "Invalid squad role. Use 'Senior' or 'Youth'.".to_string(),
        "be.error.youthAcademyOverage" => {
            "Player is too old for youth academy (must be 21 or under).".to_string()
        }
        "be.error.invalidDateOfBirth" => "Invalid date of birth format.".to_string(),
        "be.error.worldReadFileFailed" => {
            "Failed to read world file. Check the file path.".to_string()
        }
        "be.error.staffMemberNotFound" => "Staff member not found. Check the staff ID.".to_string(),
        "be.error.staffMemberAlreadyEmployed" => "Staff member is already employed.".to_string(),
        "be.error.staffMemberNotInTeam" => "Staff member is not in your team.".to_string(),
        "be.error.seasonNotComplete" => {
            "Season is not yet complete. Finish all fixtures first.".to_string()
        }
        "be.error.managedTeamNotFound" => {
            "Your managed team could not be found in the game state.".to_string()
        }
        "be.error.playerNotInClub" => "Player is not in your club.".to_string(),
        "be.error.unknownFacilityType" => {
            "Unknown facility type. Use 'training', 'medical', or 'scouting'.".to_string()
        }
        "be.error.saveNotFound" => "Save not found. Check the save ID.".to_string(),
        "be.error.contracts.boardWagePolicy" => {
            "Board wage policy blocks this renewal: wage would exceed budget.".to_string()
        }
        "be.error.contracts.noAssistantManagerAssigned" => {
            "No assistant manager assigned. Hire one first.".to_string()
        }
        "be.error.contracts.playerHasNoActiveContract" => {
            "Player has no active contract.".to_string()
        }
        "be.error.contracts.playerNotFreeAgent" => "Player is not a free agent.".to_string(),
        "be.error.contracts.terminationWouldLeaveMatchdaySquadShort" => {
            "Cannot terminate: would leave matchday squad too short.".to_string()
        }
        "be.error.finance.facilityUpgradeInsufficientFunds" => {
            "Insufficient funds for facility upgrade.".to_string()
        }
        "be.error.finance.amountOverflow" => {
            "This cash movement is too large to record.".to_string()
        }
        "be.error.finance.boardSupportAlreadyUsed" => {
            "Board support already used this season.".to_string()
        }
        "be.error.finance.sponsorPitchActiveSponsor" => {
            "Already have an active sponsor.".to_string()
        }
        "be.error.scouting.scoutNotInTeam" => "Scout is not in your team.".to_string(),
        "be.error.scouting.scoutNotFound" => "Scout not found. Check the staff ID.".to_string(),
        "be.error.scouting.playerAlreadyScouted" => "Player is already being scouted.".to_string(),
        "be.error.transfers.cannotBidOnOwnPlayer" => "Cannot bid on your own player.".to_string(),
        "be.error.transfers.insufficientFunds" => "Insufficient transfer funds.".to_string(),
        "be.error.transfers.offerNotPending" => "Offer is no longer pending.".to_string(),
        "be.error.transfers.transferWindowClosed" => "Transfer window is closed.".to_string(),
        "be.error.mcp.noLeagueYet" => {
            "No league found. The season may not have started yet.".to_string()
        }
        "be.error.mcp.teamAlreadyAssigned" => {
            "You already manage a team. Use `jobs_apply` to switch.".to_string()
        }
        "be.error.mcp.cannotDeleteActiveSave" => {
            "Cannot delete the save you are playing. Use `game_exit` first.".to_string()
        }
        "be.error.liveMatch.pressConferenceAlreadyHeld" => {
            "A press conference has already been held today.".to_string()
        }
        "be.error.liveMatch.noCompletedMatch" => {
            "No completed match found for your team.".to_string()
        }
        _ => format!("Error: {}", key),
    }
}

#[cfg(test)]
mod tests {
    use super::translate_error;

    /// The wrapper renders a key exactly once.
    ///
    /// `err_result` (`tools.rs`) already calls `translate_error` on its way out, so a tool that
    /// translated its own errors handed the wrapper a finished sentence, which fell through to the
    /// `_` arm and came back wearing an "Error: " prefix. The prefix is what a reader sees when the
    /// rule is broken again.
    #[test]
    fn translating_a_key_twice_is_not_the_same_as_translating_it_once() {
        let once = translate_error("be.error.playerNotInSquad");
        assert_eq!(once, "Player is not in your squad.");
        assert_eq!(
            translate_error(&once),
            "Error: Player is not in your squad.",
            "a second pass wraps the sentence — tools must emit keys, not prose"
        );
    }

    /// Every `be.error.*` key an MCP tool emits has to be renderable, or the agent is handed the
    /// key itself. This keeps `formatting.rs` in step with the tools as they grow, rather than
    /// trusting whoever adds the next one to remember.
    #[test]
    fn every_key_the_tools_emit_has_a_mapping() {
        let mut unmapped: Vec<String> = Vec::new();

        for (path, source) in tool_sources() {
            for key in keys_in(&source) {
                if translate_error(&key) == format!("Error: {}", key) {
                    unmapped.push(format!("{}: {}", path, key));
                }
            }
        }

        assert!(
            unmapped.is_empty(),
            "these keys reach the agent unrendered — add them to `translate_error`:\n{}",
            unmapped.join("\n")
        );
    }

    /// No tool translates its own errors.
    ///
    /// This is the rule the change exists to establish, and the one the other two tests do not
    /// hold: revert every call site and they both still pass, because `translate_error` itself is
    /// unchanged. What went wrong was never the function — it was 57 call sites invoking it before
    /// the wrapper did. So the check is on the layering: `tools_impl` returns keys, and
    /// `err_result` in `tools.rs` is the only place one becomes a sentence.
    #[test]
    fn no_tool_renders_its_own_errors() {
        let offenders: Vec<String> = tool_sources()
            .into_iter()
            .filter(|(_, source)| source.contains("translate_error"))
            .map(|(path, _)| path)
            .collect();

        assert!(
            offenders.is_empty(),
            "these tools translate before returning, so `err_result` re-wraps the sentence and \
             the agent reads \"Error: \" on a key that is mapped fine:\n{}",
            offenders.join("\n")
        );
    }

    /// Every `tools_impl` source file, as (path, contents).
    fn tool_sources() -> Vec<(String, String)> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp_server/tools_impl");
        let mut sources = Vec::new();

        for entry in std::fs::read_dir(dir).expect("tools_impl is readable") {
            let path = entry.expect("readable entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).expect("readable source");
            sources.push((path.display().to_string(), source));
        }

        assert!(!sources.is_empty(), "found no tools_impl sources to scan");
        sources
    }

    /// Every `"be.error…"` string literal in a source file.
    fn keys_in(source: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut rest = source;
        while let Some(start) = rest.find("\"be.error.") {
            let after = &rest[start + 1..];
            match after.find('"') {
                Some(end) => {
                    found.push(after[..end].to_string());
                    rest = &after[end..];
                }
                None => break,
            }
        }
        found
    }
}
