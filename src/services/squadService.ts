import type { GameStateData } from "../store/gameStore";
import { invokeCommand } from "./tauriClient";

export async function toggleTransferList(
    playerId: string,
): Promise<GameStateData> {
    return invokeCommand<GameStateData>("toggle_transfer_list", { playerId });
}

export async function toggleLoanList(playerId: string): Promise<GameStateData> {
    return invokeCommand<GameStateData>("toggle_loan_list", { playerId });
}