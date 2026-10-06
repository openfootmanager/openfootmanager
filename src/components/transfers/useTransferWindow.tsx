import type { GameStateData } from "../../store/gameStore";
import type { ComponentProps } from "react";
import type { Badge } from "../ui";
import { useTranslation } from "react-i18next";
import { resolveSeasonContext } from "../../lib/seasonContext";
import { formatDate } from "../../lib/dateFormatting";
function parseDateOnlyMs(value: string | null | undefined): number | null {
  if (!value) {
    return null;
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return null;
  }

  return Date.UTC(parsed.getUTCFullYear(), parsed.getUTCMonth(), parsed.getUTCDate());
}
function futureClosedWindowRegistrationDate(
  currentDateValue: string,
  opensOnValue: string | null | undefined,
): string | null {
  const currentDate = parseDateOnlyMs(currentDateValue);
  const opensOn = parseDateOnlyMs(opensOnValue);

  if (currentDate === null || opensOn === null || opensOn <= currentDate) {
    return null;
  }

  return opensOnValue ?? null;
}
interface Input {
  gameState: GameStateData;
}
export function useTransferWindow({ gameState }: Input) {
  const { t, i18n } = useTranslation();
  const weeklySuffix = t("finances.perWeekSuffix");
  const userTeamId = gameState.manager.team_id;
  const seasonContext = resolveSeasonContext(gameState);
  const transferWindow = seasonContext.transfer_window;
  const closedWindowRegistrationDate =
    transferWindow.status === "Closed"
      ? futureClosedWindowRegistrationDate(gameState.clock.current_date, transferWindow.opens_on)
      : null;
  const loanRegistrationDate =
    transferWindow.status === "Closed" && closedWindowRegistrationDate
      ? closedWindowRegistrationDate
      : gameState.clock.current_date;
  const transferWindowVariant: ComponentProps<typeof Badge>["variant"] =
    transferWindow.status === "DeadlineDay"
      ? "danger"
      : transferWindow.status === "Open"
        ? "success"
        : "neutral";
  const transferWindowSummary =
    transferWindow.status === "DeadlineDay"
      ? t("season.windowClosesToday")
      : transferWindow.status === "Open" && transferWindow.days_remaining !== null
        ? t("season.windowClosesInDays", {
            count: transferWindow.days_remaining,
          })
        : transferWindow.status === "Closed" && transferWindow.days_until_opens !== null
          ? t("season.windowOpensInDays", {
              count: transferWindow.days_until_opens,
            })
          : t("season.windowClosed");
  const isTransferWindowClosed = transferWindow.status === "Closed";
  const transferWindowBlocksRegistration = isTransferWindowClosed && !closedWindowRegistrationDate;
  const transferWindowBlockingTitle = transferWindowBlocksRegistration
    ? t("season.windowClosed")
    : null;
  const transferWindowBlockingDetail =
    transferWindowBlocksRegistration && transferWindowSummary !== transferWindowBlockingTitle
      ? transferWindowSummary
      : null;
  const loanWindowNoticeTitle = isTransferWindowClosed
    ? t("transfers.loanWindowClosedNoticeTitle")
    : null;
  const loanWindowNoticeDetail =
    isTransferWindowClosed && closedWindowRegistrationDate
      ? t("transfers.loanWindowClosedNoticeDetail", {
          date: formatDate(closedWindowRegistrationDate, i18n.language),
        })
      : isTransferWindowClosed
        ? t("transfers.loanWindowClosedUnavailableDetail")
        : null;
  return {
    t,
    i18n,
    weeklySuffix,
    userTeamId,
    transferWindow,
    closedWindowRegistrationDate,
    loanRegistrationDate,
    transferWindowVariant,
    transferWindowSummary,
    isTransferWindowClosed,
    transferWindowBlocksRegistration,
    transferWindowBlockingTitle,
    transferWindowBlockingDetail,
    loanWindowNoticeTitle,
    loanWindowNoticeDetail,
  };
}
