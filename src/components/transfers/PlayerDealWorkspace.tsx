import type { ReactNode } from "react";
import { ArrowLeft, ArrowRightLeft, Gavel, UserPlus } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { PlayerData, TeamData } from "../../store/gameStore";
import { countryName } from "../../lib/countries";
import {
  calcAge,
  formatAnnualAmount,
  formatVal,
  getPlayerOvr,
  getTeamName,
  positionBadgeVariant,
} from "../../lib/helpers";
import { translatePositionAbbreviation } from "../squad/SquadTab.helpers";
import { Badge, CountryFlag, PlayerAvatar } from "../ui";

export type DealKind = "transfer" | "loan" | "contract";

interface PlayerDealWorkspaceProps {
  player: PlayerData;
  teams: TeamData[];
  myTeam: TeamData | null;
  annualSuffix: string;
  transferWindowBlocksRegistration: boolean;
  transferWindowSummary: string;
  loanNoticeDetail: string | null;
  selectedKind: DealKind;
  onSelectKind: (kind: DealKind) => void;
  onClose: () => void;
  renderDealPanel: (kind: DealKind) => ReactNode;
}

interface DealOption {
  kind: DealKind;
  title: string;
  description: string;
  detail: string;
  disabledReason: string | null;
  icon: ReactNode;
}

function routeButtonClass(isSelected: boolean, isDisabled: boolean): string {
  if (isDisabled) {
    return [
      "min-h-[88px] w-full rounded-lg bg-gray-50 px-3 py-3 text-left",
      "text-gray-500 opacity-80 shadow-[0_0_0_1px_rgba(0,0,0,0.06)]",
      "transition-[box-shadow,background-color,color] duration-150",
      "dark:bg-navy-900/50 dark:text-gray-300 dark:shadow-[0_0_0_1px_rgba(255,255,255,0.08)]",
    ].join(" ");
  }

  if (isSelected) {
    return [
      "min-h-[88px] w-full rounded-lg bg-primary-50 px-3 py-3 text-left",
      "text-gray-900 shadow-[0_0_0_1px_rgba(16,185,129,0.35),0_2px_8px_rgba(16,185,129,0.12)]",
      "transition-[box-shadow,background-color,color] duration-150",
      "dark:bg-primary-900/50 dark:text-white dark:shadow-[0_0_0_1px_rgba(52,211,153,0.34)]",
    ].join(" ");
  }

  return [
    "min-h-[88px] w-full rounded-lg bg-white px-3 py-3 text-left",
    "text-gray-700 shadow-[0_0_0_1px_rgba(0,0,0,0.06)]",
    "transition-[box-shadow,background-color,color] duration-150 hover:bg-gray-50 hover:text-gray-950",
    "hover:shadow-[0_0_0_1px_rgba(0,0,0,0.1),0_2px_8px_rgba(0,0,0,0.05)]",
    "dark:bg-navy-800 dark:text-gray-300 dark:shadow-[0_0_0_1px_rgba(255,255,255,0.08)]",
    "dark:hover:bg-navy-700 dark:hover:text-white dark:hover:shadow-[0_0_0_1px_rgba(255,255,255,0.14)]",
  ].join(" ");
}

function factLabelClass(): string {
  return "text-xs font-heading font-bold uppercase tracking-wider text-gray-500 dark:text-gray-300";
}

function factValueClass(): string {
  return "mt-1 text-sm font-semibold text-gray-900 dark:text-gray-100";
}

export default function PlayerDealWorkspace({
  player,
  teams,
  myTeam,
  annualSuffix,
  transferWindowBlocksRegistration,
  transferWindowSummary,
  loanNoticeDetail,
  selectedKind,
  onSelectKind,
  onClose,
  renderDealPanel,
}: PlayerDealWorkspaceProps) {
  const { t, i18n } = useTranslation();
  const teamName = player.team_id
    ? getTeamName(teams, player.team_id)
    : t("common.freeAgent");
  const age = calcAge(player.date_of_birth);
  const ovr = getPlayerOvr(player);
  const options: DealOption[] = [
    {
      kind: "transfer",
      title: t("transfers.makeBid"),
      description: t("transfers.dealTransferDescription"),
      detail: player.transfer_listed
        ? t("transfers.dealAvailableTransfer")
        : t("transfers.dealUnavailableTransfer"),
      disabledReason: !player.transfer_listed
        ? t("transfers.dealUnavailableTransfer")
        : transferWindowBlocksRegistration
          ? transferWindowSummary
          : null,
      icon: <Gavel className="h-4 w-4" />,
    },
    {
      kind: "loan",
      title: t("transfers.makeLoanOffer"),
      description: t("transfers.dealLoanDescription"),
      detail: player.loan_listed
        ? (loanNoticeDetail ?? t("transfers.dealAvailableLoan"))
        : t("transfers.dealUnavailableLoan"),
      disabledReason: !player.loan_listed
        ? t("transfers.dealUnavailableLoan")
        : transferWindowBlocksRegistration
          ? transferWindowSummary
          : null,
      icon: <ArrowRightLeft className="h-4 w-4" />,
    },
    {
      kind: "contract",
      title: t("transfers.offerContract"),
      description: t("transfers.dealContractDescription"),
      detail:
        player.team_id === null
          ? t("transfers.dealAvailableContract")
          : t("transfers.dealUnavailableContract"),
      disabledReason:
        player.team_id === null ? null : t("transfers.dealUnavailableContract"),
      icon: <UserPlus className="h-4 w-4" />,
    },
  ];
  const selectedOption =
    options.find((option) => option.kind === selectedKind) ?? options[0];

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="player-deal-workspace-title"
      className="fixed inset-0 z-50 bg-gray-100 text-gray-900 dark:bg-navy-900 dark:text-gray-100"
    >
      <div className="flex h-full min-h-0 flex-col">
        <header className="shrink-0 border-b border-gray-200 bg-white px-4 py-3 shadow-sm dark:border-navy-600 dark:bg-navy-800">
          <div className="flex items-center gap-4">
            <button
              type="button"
              onClick={onClose}
              className="-ml-2 flex shrink-0 items-center gap-2 rounded-lg px-2 py-2 text-sm text-gray-500 transition-colors duration-150 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-navy-700 dark:hover:text-white"
              aria-label={t("common.back")}
            >
              <ArrowLeft className="h-5 w-5" />
              <span className="hidden font-heading font-bold uppercase tracking-wider sm:inline">
                {t("common.back")}
              </span>
            </button>
            <div className="flex min-w-0 flex-1 items-center gap-4">
              <PlayerAvatar
                player={player}
                className="flex h-16 w-16 shrink-0 items-center justify-center overflow-hidden rounded-lg bg-gray-100 text-sm font-heading font-bold text-gray-500 shadow-[0_0_0_1px_rgba(0,0,0,0.08)] dark:bg-navy-700 dark:text-gray-300 dark:shadow-[0_0_0_1px_rgba(255,255,255,0.1)]"
                imageClassName="h-full w-full object-cover object-top"
              />
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <h2
                    id="player-deal-workspace-title"
                    className="truncate font-heading text-2xl font-bold uppercase tracking-wide text-gray-950 dark:text-white"
                  >
                    {player.full_name}
                  </h2>
                  <Badge
                    variant={positionBadgeVariant(
                      player.natural_position || player.position,
                    )}
                    size="sm"
                  >
                    {translatePositionAbbreviation(
                      t,
                      player.natural_position || player.position,
                    )}
                  </Badge>
                </div>
                <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-gray-500 dark:text-gray-300">
                  <span>{age}</span>
                  <span className="flex items-center gap-1">
                    <CountryFlag
                      code={player.nationality}
                      locale={i18n.language}
                      className="text-sm leading-none"
                    />
                    {countryName(player.nationality, i18n.language)}
                  </span>
                  <span>{teamName}</span>
                  <span>{transferWindowSummary}</span>
                </div>
              </div>
            </div>
          </div>
        </header>

        <div className="grid min-h-0 flex-1 gap-4 overflow-y-auto p-4 lg:grid-cols-[260px_minmax(0,1fr)_280px] lg:overflow-hidden">
          <nav
            aria-label={t("transfers.dealType")}
            className="space-y-3 lg:min-h-0 lg:overflow-y-auto"
          >
            {options.map((option) => {
              const disabled = Boolean(option.disabledReason);
              const selected = option.kind === selectedKind;

              return (
                <button
                  key={option.kind}
                  type="button"
                  disabled={disabled}
                  onClick={() => onSelectKind(option.kind)}
                  className={routeButtonClass(selected, disabled)}
                  aria-pressed={selected}
                  aria-label={option.title}
                >
                  <span className="flex items-start gap-3">
                    <span
                      className={`mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md ${
                        disabled
                          ? "bg-gray-200 text-gray-500 dark:bg-navy-700 dark:text-gray-300"
                          : selected
                            ? "bg-primary-700 text-white"
                            : "bg-gray-100 text-gray-600 dark:bg-navy-700 dark:text-gray-300"
                      }`}
                    >
                      {option.icon}
                    </span>
                    <span className="min-w-0">
                      <span className="font-heading text-sm font-bold uppercase tracking-wider">
                        {option.title}
                      </span>
                      <span className="mt-1 block text-xs text-gray-500 dark:text-gray-300">
                        {option.disabledReason ?? option.detail}
                      </span>
                    </span>
                  </span>
                </button>
              );
            })}
          </nav>

          <section className="min-h-0 overflow-y-auto rounded-lg bg-white p-5 shadow-[0_0_0_1px_rgba(0,0,0,0.06),0_2px_8px_rgba(0,0,0,0.04)] dark:bg-navy-800 dark:shadow-[0_0_0_1px_rgba(255,255,255,0.08)]">
            {selectedOption.disabledReason ? (
              <div className="flex min-h-[280px] flex-col justify-center rounded-lg bg-gray-50 p-6 text-center dark:bg-navy-900/50">
                <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-lg bg-gray-200 text-gray-500 dark:bg-navy-700 dark:text-gray-300">
                  {selectedOption.icon}
                </div>
                <p className="font-heading text-lg font-bold uppercase tracking-wide text-gray-900 dark:text-white">
                  {selectedOption.title}
                </p>
                <p className="mx-auto mt-2 max-w-md text-sm text-gray-600 dark:text-gray-300">
                  {selectedOption.description}
                </p>
                <p className="mx-auto mt-2 max-w-md text-sm font-semibold text-red-600 dark:text-red-300">
                  {selectedOption.disabledReason}
                </p>
              </div>
            ) : (
              renderDealPanel(selectedKind)
            )}
          </section>

          <aside className="min-h-0 space-y-4 lg:overflow-y-auto">
            <div className="rounded-lg bg-white p-4 shadow-[0_0_0_1px_rgba(0,0,0,0.06)] dark:bg-navy-800 dark:shadow-[0_0_0_1px_rgba(255,255,255,0.08)]">
              <p className={factLabelClass()}>{t("common.ovr")}</p>
              <p className="mt-1 font-heading text-3xl font-bold tabular-nums text-primary-500">
                {ovr}
              </p>
              <div className="mt-4 grid grid-cols-2 gap-3">
                <div>
                  <p className={factLabelClass()}>{t("common.value")}</p>
                  <p className={`${factValueClass()} tabular-nums`}>
                    {formatVal(player.market_value)}
                  </p>
                </div>
                <div>
                  <p className={factLabelClass()}>{t("common.wage")}</p>
                  <p className={`${factValueClass()} tabular-nums`}>
                    {formatAnnualAmount(formatVal(player.wage), annualSuffix)}
                  </p>
                </div>
              </div>
            </div>

            {myTeam ? (
              <div className="rounded-lg bg-white p-4 shadow-[0_0_0_1px_rgba(0,0,0,0.06)] dark:bg-navy-800 dark:shadow-[0_0_0_1px_rgba(255,255,255,0.08)]">
                <p className={factLabelClass()}>
                  {t("finances.transferBudget")}
                </p>
                <p className={`${factValueClass()} tabular-nums`}>
                  {formatVal(myTeam.transfer_budget)}
                </p>
                <div className="mt-4">
                  <p className={factLabelClass()}>{t("finances.wageBudget")}</p>
                  <p className={`${factValueClass()} tabular-nums`}>
                    {formatAnnualAmount(
                      formatVal(myTeam.wage_budget),
                      annualSuffix,
                    )}
                  </p>
                </div>
              </div>
            ) : null}
          </aside>
        </div>
      </div>
    </div>
  );
}
