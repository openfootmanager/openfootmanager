import type { TFunction } from "i18next";
import type { JSX } from "react";

import { calcAge, formatDateFull, formatVal, formatWeeklyWage } from "../../lib/helpers";
import { countryName } from "../../lib/countries";
import { visibleTraits } from "../../lib/playerKnowledge";
import { naturalPositionOf } from "../../lib/positions";
import type { MessageData, PlayerData } from "../../store/gameStore";
import type { PotentialOpinion } from "../../store/generated/PotentialOpinion";
import PlayerPositionBadge from "../PlayerPositionBadge";
import PlayerProfilePotentialOpinion from "../playerProfile/PlayerProfilePotentialOpinion";
import { TraitList } from "../TraitBadge";
import { Badge, Button, Card, CardBody, CountryFlag } from "../ui";
import { isChooseOptionAction } from "./inboxHelpers";

interface InboxYouthProspectCardProps {
  prospect: PlayerData;
  /**
   * The scout's opinion of his potential (`get_youth_prospect_opinions`); null when the report
   * gave none. His rating and true potential are never shown: the club has only seen a report.
   */
  opinion: PotentialOpinion | null;
  /** The report's choice about this prospect (sign, shortlist, discard), if it is still open. */
  action: MessageData["actions"][number] | undefined;
  messageId: string;
  currentTeamId: string | null;
  language: string;
  t: TFunction;
  onAction: (messageId: string, actionId: string, optionId?: string) => void;
  onScoutPlayerClick: (playerId: string) => void;
}

/** One prospect in a youth scouting report: who he is, the scout's opinion of him, and the choice. */
export default function InboxYouthProspectCard({
  prospect,
  opinion,
  action,
  messageId,
  currentTeamId,
  language,
  t,
  onAction,
  onScoutPlayerClick,
}: InboxYouthProspectCardProps): JSX.Element {
  const chooseOptionActionType =
    action && isChooseOptionAction(action.action_type) ? action.action_type : null;
  const chooseOptionActionId = action?.id ?? null;
  const options = chooseOptionActionType ? chooseOptionActionType.ChooseOption.options : [];
  const prospectPosition = naturalPositionOf(prospect);
  const signedToAcademy = prospect.team_id === currentTeamId;
  const traits = visibleTraits(prospect.traits ?? [], opinion);

  return (
    <Card>
      <CardBody className="space-y-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <p className="font-heading font-bold text-base text-gray-900 dark:text-gray-100">
                {prospect.full_name}
              </p>
              <PlayerPositionBadge position={prospectPosition} />
              {signedToAcademy ? (
                <Badge variant="success" size="sm">
                  {t("inbox.youthProspectSigned")}
                </Badge>
              ) : null}
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-sm text-gray-500 dark:text-gray-400">
              <span>{t(`common.positions.${prospectPosition}`)}</span>
              <span>
                {t("common.age")} {calcAge(prospect.date_of_birth)}
              </span>
              <span className="inline-flex items-center gap-1">
                <CountryFlag
                  code={prospect.nationality}
                  locale={language}
                  className="text-xs leading-none"
                />
                <span>{countryName(prospect.nationality, language)}</span>
              </span>
            </div>
          </div>

          {traits.length > 0 ? <TraitList traits={traits} size="sm" /> : null}
        </div>

        <div className="grid gap-3 sm:grid-cols-2">
          {opinion ? (
            <PlayerProfilePotentialOpinion opinion={opinion} locale={language} t={t} />
          ) : null}

          <div className="rounded-lg bg-sunken px-3 py-2">
            <p className="font-heading text-caption font-bold uppercase tracking-label text-fg-muted">
              {t("playerProfile.contractInfo")}
            </p>
            <div className="mt-2 flex flex-wrap gap-2 text-caption text-fg">
              <Badge variant="neutral" size="sm">
                {t("finances.wagePerWeek")}: {formatWeeklyWage(prospect.wage ?? 0, "compact")}
              </Badge>
              {prospect.contract_end ? (
                <Badge variant="neutral" size="sm">
                  {formatDateFull(prospect.contract_end, language)}
                </Badge>
              ) : null}
              <Badge variant="neutral" size="sm">
                {t("finances.marketValue")}: {formatVal(prospect.market_value ?? 0)}
              </Badge>
            </div>
          </div>
        </div>

        {signedToAcademy ? (
          <div className="flex flex-wrap items-center gap-2">
            <div className="text-xs font-heading font-bold uppercase tracking-wider text-primary-500">
              {t("inbox.youthProspectSigned")}
            </div>
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() => onScoutPlayerClick(prospect.id)}
            >
              {t("squad.viewProfile")}
            </Button>
          </div>
        ) : action?.resolved ? (
          <div className="mt-3 text-xs font-heading font-bold uppercase tracking-wider text-primary-500">
            {t("inbox.responded")}
          </div>
        ) : chooseOptionActionId && options.length > 0 ? (
          <div className="mt-3 flex flex-wrap gap-2">
            {options.map((option: (typeof options)[number]) => (
              <Button
                key={option.id}
                type="button"
                size="sm"
                variant={option.id === "discard" ? "outline" : "primary"}
                onClick={() => onAction(messageId, chooseOptionActionId, option.id)}
              >
                {option.label}
              </Button>
            ))}
          </div>
        ) : null}
      </CardBody>
    </Card>
  );
}
