import type { GameStateData } from "../../store/gameStore";
import { useTranslation } from "react-i18next";
import {
  HomeBoardObjectivesSection,
  HomeOverviewSection,
  HomeSquadPulseSection,
} from "./HomeTab.sections";
import { createHomeTabViewModel } from "./HomeTab.viewModel";

interface HomeTabProps {
  gameState: GameStateData;
  onNavigate?: (tab: string, context?: { messageId?: string }) => void;
  visitedOnboardingTabs: ReadonlySet<string>;
}

export default function HomeTab({
  gameState,
  onNavigate,
  visitedOnboardingTabs,
}: HomeTabProps) {
  const { t, i18n } = useTranslation();
  const viewModel = createHomeTabViewModel({
    gameState,
    lang: i18n.language,
    t,
    visitedOnboardingTabs,
  });

  return (
    <div className="max-w-6xl mx-auto flex flex-col gap-5">
      <HomeOverviewSection
        gameState={gameState}
        onNavigate={onNavigate}
        t={t}
        viewModel={viewModel}
      />
      <HomeBoardObjectivesSection
        gameState={gameState}
        t={t}
        viewModel={viewModel}
      />
      <HomeSquadPulseSection
        gameState={gameState}
        onNavigate={onNavigate}
        viewModel={viewModel}
      />
    </div>
  );
}
