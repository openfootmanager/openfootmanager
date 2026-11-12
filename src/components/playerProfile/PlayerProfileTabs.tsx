import { useId } from "react";
import { useTranslation } from "react-i18next";

import type { PlayerData, TeamData } from "../../store/gameStore";
import type { PlayerRole } from "../../store/types";
import { usePlayerCareer } from "../../hooks/usePlayerCareer";
import { TabPanel, Tabs } from "../ui";
import { visibleWageText } from "../../lib/playerKnowledge";
import { isGoalkeeper } from "./PlayerProfile.attributes";
import { buildContractHistory } from "./PlayerProfile.contractHistory";
import { withClubNames } from "./PlayerProfile.movements";
import { formatPlayerWage } from "./PlayerProfile.helpers";
import type { buildPlayerProfileRelationship } from "./PlayerProfile.viewModel";
import PlayerProfileAdvancedStatsCard from "./PlayerProfileAdvancedStatsCard";
import PlayerProfileCareerTable from "./PlayerProfileCareerTable";
import PlayerProfileContractHistoryCard from "./PlayerProfileContractHistoryCard";
import PlayerProfileContractSheet from "./PlayerProfileContractSheet";
import PlayerProfilePotentialOpinion from "./PlayerProfilePotentialOpinion";
import PlayerProfileAttributesTab from "./attributes/PlayerProfileAttributesTab";
import PlayerProfileOverview from "./overview/PlayerProfileOverview";
import ProfileLoadFailed from "./ProfileLoadFailed";
import type { usePlayerProfileData } from "./usePlayerProfileData";
import type { PlayerProfileActions } from "./usePlayerProfileActions";
import type { PlayerProfileKnowledge } from "./usePlayerProfileKnowledge";
import { useContractSheetData } from "./useContractSheetData";

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
  /** Opens a club (the contracts he has had link to theirs). */
  onSelectTeam?: (id: string) => void;
  /** The game's clubs: a ledger entry that names its club by id alone is named from them. */
  teams: Pick<TeamData, "id" | "name">[];
  /** Changes when anything the club-wide previews read does (`clubStateKey`): they ask again. */
  clubKey: string;
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
  onSelectTeam,
  teams,
  clubKey,
}: PlayerProfileTabsProps) {
  const { t, i18n } = useTranslation();
  const tabsId = useId();
  const tabItems = PROFILE_TABS.map((id) => ({ id, label: t(`playerProfile.tabs.${id}`) }));
  const hidden = (id: ProfileTab) => tab !== id;
  const { isManagerOwnedProfile, isFreeAgent, hasLetExpireIntent } = relationship;
  const { opinionAnswer, potentialOpinion } = knowledge;
  const movements = withClubNames(player.movement_history ?? [], teams);
  const contractHistory = buildContractHistory(movements, currentDate.slice(0, 10));
  // His career is the backend's projection; it changes with a new day (the season in progress),
  // a move and a rollover, so those make the key.
  const career = usePlayerCareer(
    player.id,
    [currentDate, player.team_id ?? "", player.career.length, movements.length].join("|"),
  );
  const exactWeeklyWage =
    knowledge.visibleWage.kind === "exact" ? knowledge.visibleWage.weekly : null;
  // Asked for only while this tab is the open one: hidden tabs stay mounted, and the previews are
  // the backend's work. They are asked again when the contract, the wage, the day or the club's squad
  // changes under the sheet.
  const contractSheet = useContractSheetData({
    player,
    active: tab === "contract",
    ownsContract: isManagerOwnedProfile,
    refreshKey: [currentDate, player.contract_end, player.wage, hasLetExpireIntent, clubKey].join(
      "|",
    ),
  });
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

      <TabPanel idPrefix={tabsId} tabId="career" hidden={hidden("career")}>
        <div className="@container">
          <div className="grid grid-cols-1 items-start gap-5 @[56rem]:grid-cols-[minmax(0,1fr)_22rem]">
            <PlayerProfileCareerTable
              career={career}
              language={i18n.language}
              onOpenTeam={onSelectTeam}
              t={t}
            />
            <PlayerProfileAdvancedStatsCard summary={data.advancedStats} t={t} />
          </div>
        </div>
      </TabPanel>

      <TabPanel
        idPrefix={tabsId}
        tabId="contract"
        hidden={hidden("contract")}
        className="flex flex-col gap-5"
      >
        <PlayerProfileContractSheet
          player={player}
          currentDate={currentDate}
          isOwnContract={isManagerOwnedProfile}
          isFreeAgent={isFreeAgent}
          hasLetExpireIntent={hasLetExpireIntent}
          exactWeeklyWage={exactWeeklyWage}
          language={i18n.language}
          contractRisk={contractRisk}
          lastSigned={contractHistory.find((row) => row.inForce) ?? null}
          below={
            <PlayerProfileContractHistoryCard
              rows={contractHistory}
              showsWages={exactWeeklyWage !== null}
              language={i18n.language}
              onOpenTeam={onSelectTeam}
              t={t}
            />
          }
          data={contractSheet}
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
