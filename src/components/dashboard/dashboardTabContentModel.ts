import type {
    GameStateData,
    PlayerSelectionOptions,
} from "../../store/gameStore";
import type { DashboardNavigateContext } from "./dashboardProfileNavigation";

export interface DashboardTabContentHandlers {
    onSelectPlayer: (id: string, options?: PlayerSelectionOptions) => void;
    onSelectTeam: (id: string) => void;
    onGameUpdate: (state: GameStateData) => void;
    onNavigate: (tab: string, context?: DashboardNavigateContext) => void;
}

export interface DashboardTabContentModel {
    activeTab: string;
    gameState: GameStateData;
    seasonComplete: boolean;
    visitedOnboardingTabs: ReadonlySet<string>;
    initialMessageId: string | null;
    preferredNation?: string | null;
    preferredCompetitionId?: string | null;
    managerId: string;
    handlers: DashboardTabContentHandlers;
}

interface CreateDashboardTabContentModelArgs {
    activeTab: string;
    gameState: GameStateData;
    seasonComplete: boolean;
    visitedOnboardingTabs: ReadonlySet<string>;
    initialMessageId: string | null;
    preferredNation?: string | null;
    preferredCompetitionId?: string | null;
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
        initialMessageId: args.initialMessageId,
        preferredNation: args.preferredNation ?? null,
        preferredCompetitionId: args.preferredCompetitionId ?? null,
        managerId: args.gameState.manager.id,
        handlers: args.handlers,
    };
}
