import { invoke } from "@tauri-apps/api/core";

import type { ManagerStanding } from "../store/generated/ManagerStanding";
import type { ManagersPage } from "../store/generated/ManagersPage";
import type { ManagersStatus } from "../store/generated/ManagersStatus";

/**
 * The managers directory (`ofm_core::slices::managers`): every active manager with his club,
 * reputation and record, and every club with no manager. The backend picks each reputation's band
 * and tone; the screen translates the band (`managersWorld.reputationBand.<band>`) and paints the tone.
 */
export async function getManagersPage(status: ManagersStatus): Promise<ManagersPage> {
  return invoke<ManagersPage>("get_managers_page", { status });
}

/**
 * How the board and the fans feel about the user's manager (`ofm_core::manager_standing`): the
 * board is under threat exactly where it warns, so the Manager tab shows the backend's words
 * instead of picking them from the two numbers.
 */
export async function getManagerStanding(): Promise<ManagerStanding> {
  return invoke<ManagerStanding>("get_manager_standing");
}
