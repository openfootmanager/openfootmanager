import type { BlockerData } from "../services/advanceTimeService";

export interface BlockerModal {
  blockers: BlockerData[];
  pendingAction?: () => void;
}
