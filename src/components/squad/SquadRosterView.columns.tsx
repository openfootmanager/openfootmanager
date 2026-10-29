import type { TFunction } from "i18next";
import type { ReactNode } from "react";

import { getInjurySeverity, resolveInjuryName } from "../../lib/injury";
import { calcAge, getPlayerOvr } from "../../lib/helpers";
import {
  type ContractRiskLevel,
  contractExpiryLabel,
  getContractRiskBadgeVariant,
  getContractYearsRemaining,
} from "../../lib/contractUtils";
import type { RatingTier } from "../../services/squadService";
import type { PlayerData } from "../../store/gameStore";
import { TraitList } from "../TraitBadge";
import {
  Badge,
  ConditionMeter,
  CountryFlag,
  type DataTableColumn,
  InjuryBadge,
  PlayerAvatar,
  RatingChip,
} from "../ui";
import type { SquadListSortKey } from "./SquadRosterView.state";

interface SquadColumnContext {
  t: TFunction;
  language: string;
  clockDate: string;
  tierOf: (rating: number) => RatingTier;
  riskLevelOf: (playerId: string) => ContractRiskLevel;
  /** The natural position and alternates, with where a starter is deployed: the roster's own. */
  renderPositions: (player: PlayerData) => ReactNode;
}

/** A column the roster sorts by: its key is the sort key, so a header asks for that sort. */
type SquadColumn = DataTableColumn<PlayerData> & { key: SquadListSortKey | "traits" };

/** A long injury is a danger, a short one a warning; the injury's name is the dot's tooltip. */
function InjuryDot({ player, t }: { player: PlayerData; t: TFunction }) {
  if (!player.injury) return null;
  const severity = getInjurySeverity(player.injury.days_remaining);
  const fill =
    severity === "major" || severity === "serious" ? "bg-danger-solid" : "bg-warning-solid";
  return (
    <span
      className={`inline-block h-1.5 w-1.5 shrink-0 rounded-full ${fill}`}
      title={resolveInjuryName(player.injury.name, t)}
    />
  );
}

function ContractCell({ player, context }: { player: PlayerData; context: SquadColumnContext }) {
  const { t, language, clockDate, riskLevelOf } = context;
  const risk = riskLevelOf(player.id);
  const riskLabel =
    risk === "critical"
      ? t("finances.contractRiskCritical")
      : risk === "warning"
        ? t("finances.contractRiskWarning")
        : t("finances.contractRiskStable");
  return (
    <div className="flex flex-col gap-1 text-caption text-fg-muted">
      <div className="flex items-center gap-1.5">
        <span className="font-medium text-fg">
          {getContractYearsRemaining(player.contract_end, clockDate)}
        </span>
        <Badge variant={getContractRiskBadgeVariant(risk)} size="sm">
          {riskLabel}
        </Badge>
      </div>
      <span>
        {player.contract_end ? contractExpiryLabel(player.contract_end, language, t) : "—"}
      </span>
      {player.transfer_listed || player.loan_listed || player.injury ? (
        <div className="flex flex-wrap gap-1">
          {player.transfer_listed ? (
            <Badge variant="accent" size="sm">
              {t("transfers.transfer")}
            </Badge>
          ) : null}
          {player.loan_listed ? (
            <Badge variant="primary" size="sm">
              {t("transfers.loan")}
            </Badge>
          ) : null}
          {player.injury ? <InjuryBadge injury={player.injury} /> : null}
        </div>
      ) : null}
    </div>
  );
}

/**
 * The roster's columns, as table C draws them. Traits, age, morale and the contract fold into a
 * row's details in a narrow window; the number, name, positions, condition and rating stay.
 */
export function squadRosterColumns(context: SquadColumnContext): SquadColumn[] {
  const { t, tierOf, renderPositions } = context;
  return [
    {
      key: "jersey",
      header: t("squad.jerseyNumber"),
      headerLabel: t("squad.jerseyNumberFull"),
      numeric: true,
      sortable: true,
      cell: (player) => player.jersey_number ?? "—",
    },
    {
      key: "name",
      header: t("common.name"),
      primary: true,
      sortable: true,
      cell: (player) => (
        <span className="flex min-w-0 items-center gap-2">
          <PlayerAvatar player={player} />
          <InjuryDot player={player} t={t} />
          <span className="truncate font-semibold">{player.match_name}</span>
          <CountryFlag code={player.nationality} className="shrink-0 leading-none" />
        </span>
      ),
    },
    { key: "pos", header: t("squad.pos"), sortable: true, cell: renderPositions },
    {
      key: "traits",
      header: t("squad.traits"),
      drawer: true,
      cell: (player) => <TraitList traits={player.traits || []} size="xs" max={2} />,
    },
    {
      key: "age",
      header: t("common.age"),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (player) => calcAge(player.date_of_birth),
    },
    {
      key: "condition",
      header: t("common.condition"),
      sortable: true,
      cell: (player) => (
        <ConditionMeter value={player.condition} status={player.condition_status} />
      ),
    },
    {
      key: "morale",
      header: t("common.morale"),
      numeric: true,
      sortable: true,
      drawer: true,
      cell: (player) => player.morale,
    },
    {
      key: "ovr",
      header: t("common.ovr"),
      sortable: true,
      cell: (player) => {
        const ovr = getPlayerOvr(player);
        return <RatingChip value={ovr} tier={tierOf(ovr)} />;
      },
    },
    {
      key: "contract",
      header: t("common.contract"),
      sortable: true,
      drawer: true,
      cell: (player) => <ContractCell player={player} context={context} />,
    },
  ];
}
