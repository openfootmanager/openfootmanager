import { useId } from "react";
import { useTranslation } from "react-i18next";

import type { PlayerData } from "../../store/gameStore";
import type { PlayerRole } from "../../store/types";
import { TabPanel, Tabs } from "../ui";
import { visibleWageText } from "../../lib/playerKnowledge";
import { isGoalkeeper } from "./PlayerProfile.attributes";
import { formatPlayerWage } from "./PlayerProfile.helpers";
import type { buildPlayerProfileRelationship } from "./PlayerProfile.viewModel";
import PlayerProfileAdvancedStatsCard from "./PlayerProfileAdvancedStatsCard";
import PlayerProfileCareerHistoryCard from "./PlayerProfileCareerHistoryCard";
import PlayerProfileContractCard from "./PlayerProfileContractCard";
import PlayerProfilePotentialOpinion from "./PlayerProfilePotentialOpinion";
import PlayerProfileAttributesTab from "./attributes/PlayerProfileAttributesTab";
import PlayerProfileMovementHistoryCard from "./PlayerProfileMovementHistoryCard";
import PlayerProfileOverview from "./overview/PlayerProfileOverview";
import ProfileLoadFailed from "./ProfileLoadFailed";
import type { usePlayerProfileData } from "./usePlayerProfileData";
import type { PlayerProfileActions } from "./usePlayerProfileActions";
import type { PlayerProfileKnowledge } from "./usePlayerProfileKnowledge";

/** The profile's sections, in tab order. */
export const PROFILE_TABS = ["overview", "attributes", "career", "contract"] as const;
export type ProfileTab = (typeof PROFILE_TABS)[number];

interface PlayerProfileTabsProps {
  tab: ProfileTab;
  onTabChange: (tab: ProfileTab) => void;
  player: PlayerData;
  /** The game's date, for the years left on his contract. */
  currentDate: string;
  /** The own club's player in a screen that can edit (the role select is his alone). */
  canSetTacticalRole: boolean;
  relationship: ReturnType<typeof buildPlayerProfileRelationship>;
  data: ReturnType<typeof usePlayerProfileData>;
  knowledge: PlayerProfileKnowledge;
  actions: PlayerProfileActions;
  contractRisk: { level: "critical" | "warning" | "stable"; label: string };
}

/** The four tabs of the profile and what each holds. */
export default function PlayerProfileTabs({
  tab,
  onTabChange,
  player,
  currentDate,
  canSetTacticalRole,
  relationship,
  data,
  knowledge,
  actions,
  contractRisk,
}: PlayerProfileTabsProps) {
  const { t, i18n } = useTranslation();
  const tabsId = useId();
  const tabItems = PROFILE_TABS.map((id) => ({ id, label: t(`playerProfile.tabs.${id}`) }));
  const hidden = (id: ProfileTab) => tab !== id;
  const { isManagerOwnedProfile, isFreeAgent, hasLetExpireIntent } = relationship;
  const { opinionAnswer, potentialOpinion } = knowledge;
  const opinion =
    opinionAnswer.status === "failed" ? (
      <ProfileLoadFailed t={t} onRetry={opinionAnswer.retry} />
    ) : potentialOpinion ? (
      <PlayerProfilePotentialOpinion opinion={potentialOpinion} locale={i18n.language} t={t} />
    ) : null;

  return (
    <>
      <div className="mb-4 border-b border-line">
        <Tabs
          idPrefix={tabsId}
          label={t("playerProfile.sections")}
          tabs={tabItems}
          selectedId={tab}
          onSelect={(id) => onTabChange(id as ProfileTab)}
        />
      </div>

      <TabPanel idPrefix={tabsId} tabId="overview" hidden={hidden("overview")}>
        <PlayerProfileOverview
          player={player}
          language={i18n.language}
          showsPrivateState={knowledge.showsPrivateState}
          recentMatches={data.recentMatches}
          recentMatchesStatus={data.recentMatchesStatus}
          retryRecentMatches={data.retryRecentMatches}
          seasonRatingRated={data.advancedStats.seasonRatingRated}
          tacticalRole={
            canSetTacticalRole
              ? {
                  value: actions.tacticalRole.value,
                  options: actions.tacticalRole.options,
                  onChange: (role) => void actions.tacticalRole.onChange(role as PlayerRole),
                }
              : null
          }
          contractRiskLevel={contractRisk.level}
          contractRiskLabel={contractRisk.label}
          wageText={visibleWageText(knowledge.visibleWage, formatPlayerWage, t)}
          wageRetry={knowledge.wageRetry}
          potentialOpinion={opinion}
          ratings={knowledge.ratingsAnswer}
          attributes={knowledge.readableAttributes}
          traits={knowledge.traits}
          onOpenTab={onTabChange}
        />
      </TabPanel>

      <TabPanel idPrefix={tabsId} tabId="attributes" hidden={hidden("attributes")}>
        <PlayerProfileAttributesTab
          attributes={knowledge.readableAttributes}
          isGoalkeeper={isGoalkeeper(player)}
          ratings={knowledge.ratingsAnswer}
          scoutedOn={knowledge.scoutedReport?.scouted_on ?? null}
          scoutOutOfDate={knowledge.scoutedReport?.out_of_date ?? false}
          language={i18n.language}
          potentialOpinion={opinion}
        />
      </TabPanel>

      <TabPanel
        idPrefix={tabsId}
        tabId="career"
        hidden={hidden("career")}
        className="grid grid-cols-1 gap-5"
      >
        <PlayerProfileAdvancedStatsCard summary={data.advancedStats} t={t} />

        <PlayerProfileCareerHistoryCard career={player.career} t={t} />

        <PlayerProfileMovementHistoryCard
          movementHistory={player.movement_history ?? []}
          showsWages={knowledge.visibleWage.kind === "exact"}
          t={t}
        />
      </TabPanel>

      <TabPanel
        idPrefix={tabsId}
        tabId="contract"
        hidden={hidden("contract")}
        className="flex flex-col gap-5"
      >
        <PlayerProfileContractCard
          contractEnd={player.contract_end}
          currentDate={currentDate}
          isOwnClub={isManagerOwnedProfile}
          isFreeAgent={isFreeAgent}
          hasLetExpireIntent={hasLetExpireIntent}
          actionSubmitting={actions.contract.contractActionSubmitting}
          onOpenRenewal={actions.renewal.openRenewalModal}
          onMarkLetExpire={() => void actions.contract.handleMarkLetExpire()}
          onClearLetExpire={() => void actions.contract.handleClearLetExpire()}
          onOpenTermination={() => void actions.contract.openTerminationModal()}
          onOpenFreeAgentContract={() => actions.freeAgent.openFreeAgentContract(player)}
          t={t}
        />
      </TabPanel>
    </>
  );
}
