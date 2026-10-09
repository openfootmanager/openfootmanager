import type { ReactNode } from "react";
import { ArrowRightLeft, Gavel, UserPlus } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { PlayerData, TeamData } from "../../store/gameStore";
import { getPlayerDealBlocker } from "./TransfersTab.model";
import {
  DealWorkspaceHeader,
  DealWorkspaceRoutes,
  DealWorkspaceFacts,
  type DealOption,
} from "./PlayerDealWorkspace.layout";
import PlayerOfferHistory from "./PlayerOfferHistory";
import { useDealWorkspaceFocus } from "./useDealWorkspaceFocus";

export type DealKind = "transfer" | "loan" | "contract";

interface PlayerDealWorkspaceProps {
  player: PlayerData;
  teams: TeamData[];
  myTeam: TeamData | null;
  weeklySuffix: string;
  transferWindowBlocksRegistration: boolean;
  transferWindowSummary: string;
  loanNoticeDetail: string | null;
  selectedKind: DealKind;
  /**
   * Live wage being offered in the active deal (same units as `player.wage`),
   * or null when the route has no wage offer. Shown alongside the player's
   * current wage so the two never read as one contradictory figure (#305).
   */
  offeredWage?: number | null;
  onSelectKind: (kind: DealKind) => void;
  onClose: () => void;
  renderDealPanel: (kind: DealKind) => ReactNode;
}

export default function PlayerDealWorkspace({
  player,
  teams,
  myTeam,
  weeklySuffix,
  transferWindowBlocksRegistration,
  transferWindowSummary,
  loanNoticeDetail,
  selectedKind,
  offeredWage,
  onSelectKind,
  onClose,
  renderDealPanel,
}: PlayerDealWorkspaceProps) {
  const { t } = useTranslation();
  const blocker = getPlayerDealBlocker(player, myTeam?.id ?? null);
  const unavailableReason = blocker ? t(blocker) : null;
  const dialogRef = useDealWorkspaceFocus(onClose, unavailableReason);
  const options: DealOption[] = [
    {
      kind: "transfer",
      title: t("transfers.makeBid"),
      description: t("transfers.dealTransferDescription"),
      detail: player.transfer_listed
        ? t("transfers.dealAvailableTransfer")
        : t("transfers.dealUnavailableTransfer"),
      disabledReason:
        unavailableReason ??
        (!player.transfer_listed
          ? t("transfers.dealUnavailableTransfer")
          : transferWindowBlocksRegistration
            ? transferWindowSummary
            : null),
      icon: <Gavel className="h-4 w-4" />,
    },
    {
      kind: "loan",
      title: t("transfers.makeLoanOffer"),
      description: t("transfers.dealLoanDescription"),
      detail: player.loan_listed
        ? (loanNoticeDetail ?? t("transfers.dealAvailableLoan"))
        : t("transfers.dealUnavailableLoan"),
      disabledReason:
        unavailableReason ??
        (!player.loan_listed
          ? t("transfers.dealUnavailableLoan")
          : transferWindowBlocksRegistration
            ? transferWindowSummary
            : null),
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
        unavailableReason ??
        (player.team_id === null ? null : t("transfers.dealUnavailableContract")),
      icon: <UserPlus className="h-4 w-4" />,
    },
  ];
  const selectedOption = options.find((option) => option.kind === selectedKind) ?? options[0];

  return (
    <div
      ref={dialogRef}
      role="dialog"
      aria-modal="true"
      aria-labelledby="player-deal-workspace-title"
      className="fixed inset-0 z-50 bg-gray-100 text-gray-900 dark:bg-navy-900 dark:text-gray-100"
    >
      <div className="flex h-full min-h-0 flex-col">
        <DealWorkspaceHeader
          player={player}
          teams={teams}
          transferWindowSummary={transferWindowSummary}
          onClose={onClose}
        />
        <div className="grid min-h-0 flex-1 gap-4 overflow-y-auto p-4 lg:grid-cols-[260px_minmax(0,1fr)_280px] lg:overflow-hidden">
          <DealWorkspaceRoutes
            options={options}
            selectedKind={selectedKind}
            onSelectKind={onSelectKind}
          />
          <section className="min-h-0 overflow-y-auto rounded-lg border border-gray-200 dark:border-navy-600 bg-white p-5  dark:bg-navy-800 ">
            <PlayerOfferHistory player={player} userTeamId={myTeam?.id ?? null} teams={teams} />
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
                <p
                  role="status"
                  className="mx-auto mt-2 max-w-md text-sm font-semibold text-red-600 dark:text-red-300"
                >
                  {selectedOption.disabledReason}
                </p>
              </div>
            ) : (
              renderDealPanel(selectedKind)
            )}
          </section>
          <DealWorkspaceFacts
            player={player}
            myTeam={myTeam}
            offeredWage={offeredWage}
            weeklySuffix={weeklySuffix}
          />
        </div>
      </div>
    </div>
  );
}
