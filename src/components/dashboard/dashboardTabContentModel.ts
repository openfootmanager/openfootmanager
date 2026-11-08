import type { GameStateData, PlayerSelectionOptions } from "../../store/gameStore";
import type { SquadListSortState } from "../squad/SquadRosterView.state";
import type { DashboardNavigateContext } from "./dashboardProfileNavigation";

export interface DashboardTabContentHandlers {
  onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
  onSelectTeam: (id: string) => void;
  onGameUpdate: (state: GameStateData) => void;
  onNavigate: (tab: string, context?: DashboardNavigateContext) => void;
  onSquadListSortChange?: (sortState: SquadListSortState) => void;
}

/**
 * The tabs that fill the workspace rather than scroll in it: the page never scrolls for them, and
 * their own regions do (layout rule 1's exception; the maintainer's ruling of 4 Nov 2026 for
 * Tactics, whose table, instructions and pitch each scroll inside themselves).
 */
const TABS_THAT_FILL_THE_WORKSPACE: ReadonlySet<string> = new Set(["Tactics"]);

/** Whether `tab` fills the workspace instead of scrolling in it. */
export function tabFillsWorkspace(tab: string): boolean {
  return TABS_THAT_FILL_THE_WORKSPACE.has(tab);
}

export interface DashboardTabContentModel {
  activeTab: string;
  gameState: GameStateData;
  seasonComplete: boolean;
  visitedOnboardingTabs: ReadonlySet<string>;
  squadListSortState: SquadListSortState;
  initialMessageId: string | null;
  managerId: string;
  handlers: DashboardTabContentHandlers;
}

interface CreateDashboardTabContentModelArgs {
  activeTab: string;
  gameState: GameStateData;
  seasonComplete: boolean;
  visitedOnboardingTabs: ReadonlySet<string>;
  squadListSortState?: SquadListSortState;
  initialMessageId: string | null;
  handlers: DashboardTabContentHandlers;
}

export function createDashboardTabContentModel(
  args: CreateDashboardTabContentModelArgs,
): DashboardTabContentModel {
  return {
    activeTab: args.activeTab,
    gameState: args.gameState,
    seasonComplete: args.seasonComplete,
    visitedOnboardingTabs: args.visitedOnboardingTabs,
    squadListSortState: args.squadListSortState ?? {
      sortKey: "pos",
      sortDir: "asc",
    },
    initialMessageId: args.initialMessageId,
    managerId: args.gameState.manager.id,
    handlers: args.handlers,
  };
}
