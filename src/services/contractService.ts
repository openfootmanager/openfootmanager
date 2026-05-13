import { invoke } from "@tauri-apps/api/core";

import type { GameStateData } from "../store/gameStore";

export type SquadSafetyIssue =
    | "too_few_healthy_players"
    | "no_healthy_goalkeeper"
    | "incomplete_formation";

export interface SquadSafetyReportData {
    team_id: string;
    projected_roster_size: number;
    healthy_players: number;
    healthy_goalkeepers: number;
    effective_xi_size: number;
    can_field_matchday_squad: boolean;
    missing_reasons: SquadSafetyIssue[];
}

export interface ContractTerminationPreviewData {
    player_id: string;
    player_name: string;
    severance_cost: number;
    squad_safety: SquadSafetyReportData;
}

export interface ContractExitIntentResponseData {
    game: GameStateData;
}

export interface ContractTerminationPreviewResponseData {
    preview: ContractTerminationPreviewData;
}

export interface ContractTerminationResponseData {
    game: GameStateData;
    severance_cost: number;
    squad_safety: SquadSafetyReportData;
}

export async function setContractExitIntent(
    playerId: string,
    reason?: string,
): Promise<ContractExitIntentResponseData> {
    return invoke<ContractExitIntentResponseData>("set_contract_exit_intent", {
        playerId,
        reason: reason ?? null,
    });
}

export async function clearContractExitIntent(
    playerId: string,
): Promise<ContractExitIntentResponseData> {
    return invoke<ContractExitIntentResponseData>("clear_contract_exit_intent", {
        playerId,
    });
}

export async function previewContractTermination(
    playerId: string,
): Promise<ContractTerminationPreviewResponseData> {
    return invoke<ContractTerminationPreviewResponseData>(
        "preview_contract_termination",
        { playerId },
    );
}

export async function terminateContractNow(
    playerId: string,
): Promise<ContractTerminationResponseData> {
    return invoke<ContractTerminationResponseData>("terminate_contract_now", {
        playerId,
    });
}
