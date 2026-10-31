import type { TFunction } from "i18next";
import type { ReactNode } from "react";

import { calcAge, getPlayerOvr } from "../../lib/helpers";
import {
  type ContractRiskLevel,
  contractExpiryLabel,
  getContractRiskBadgeVariant,
} from "../../lib/contractUtils";
import { formatDate } from "../../lib/dateFormatting";
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
  tierOf: (rating: number) => RatingTier;
  riskLevelOf: (playerId: string) => ContractRiskLevel;
  /** The natural position and alternates, with where a starter is deployed: the roster's own. */
  renderPositions: (player: PlayerData) => ReactNode;
}

/** A column the roster sorts by: its key is the sort key, so a header asks for that sort. */
type SquadColumn = DataTableColumn<PlayerData> & { key: SquadListSortKey | "traits" };

/**
 * What a row says beside the name, outside its open button so a screen reader reads it: the
 * nationality, an injury (named, with the days left; it never folds away, unlike the contract
 * cell), and that the player starts, which the row's accent otherwise says only in colour.
 */
export function SquadNameAside({
  player,
  starting,
  t,
}: {
  player: PlayerData;
  starting: boolean;
  t: TFunction;
}) {
  return (
    <span className="flex shrink-0 items-center gap-1.5">
      <CountryFlag code={player.nationality} className="shrink-0 leading-none" />
      {player.injury ? <InjuryBadge injury={player.injury} showName={false} /> : null}
      {starting ? <span className="sr-only">{t("squad.startingXi")}</span> : null}
    </span>
  );
}

/**
 * The contract's one chip, as table C has it: an expiring or critical contract first (it needs the
 * manager), then a transfer or loan listing, then a stable contract.
 */
function contractChip(player: PlayerData, risk: ContractRiskLevel, t: TFunction) {
  if (risk === "critical")
    return {
      variant: getContractRiskBadgeVariant(risk),
      label: t("finances.contractRiskCritical"),
    };
  if (risk === "warning")
    return { variant: getContractRiskBadgeVariant(risk), label: t("finances.contractRiskWarning") };
  if (player.transfer_listed) return { variant: "accent" as const, label: t("transfers.transfer") };
  if (player.loan_listed) return { variant: "primary" as const, label: t("transfers.loan") };
  return { variant: getContractRiskBadgeVariant(risk), label: t("finances.contractRiskStable") };
}

/**
 * Table C's contract column, on one line: the chip and the month the contract ends (the full date
 * is its tooltip). The injury is beside the name.
 */
function ContractCell({ player, context }: { player: PlayerData; context: SquadColumnContext }) {
  const { t, language, riskLevelOf } = context;
  const chip = contractChip(player, riskLevelOf(player.id), t);
  return (
    <span className="flex items-center gap-1.5 text-caption text-fg-muted">
      <Badge variant={chip.variant} size="sm">
        {chip.label}
      </Badge>
      {player.contract_end ? (
        <span title={contractExpiryLabel(player.contract_end, language, t)}>
          {formatDate(player.contract_end, language, { month: "short", year: "numeric" })}
        </span>
      ) : (
        <span>—</span>
      )}
    </span>
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
          <span className="truncate font-semibold">{player.match_name}</span>
        </span>
      ),
    },
    { key: "pos", header: t("squad.pos"), sortable: true, cell: renderPositions },
    {
      key: "traits",
      header: t("squad.traits"),
      drawer: true,
      cell: (player) => <TraitList traits={player.traits || []} size="xs" max={2} oneLine />,
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
