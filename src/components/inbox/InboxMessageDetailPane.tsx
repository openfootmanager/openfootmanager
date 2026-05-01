import { ArrowLeft, CheckCircle2, MailOpen, MessageCircle, Trash2 } from "lucide-react";
import type { JSX } from "react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { formatDateFull, getTeamName } from "../../lib/helpers";
import type { GameStateData } from "../../store/gameStore";
import ScoutPlayerCard from "../ScoutPlayerCard";
import SwitchClubConfirmModal from "../SwitchClubConfirmModal";
import { Badge, Button } from "../ui";
import InboxDelegatedRenewalReport from "./InboxDelegatedRenewalReport";
import {
  getActionButtonClassName,
  getCategoryColor,
  getCategoryIcon,
  isChooseOptionAction,
  isPlayerEventMessage,
  renderMessageBodyLine,
} from "./inboxHelpers";

interface PendingSwitch {
  messageId: string;
  actionId: string;
  optionId: string;
  newClubName: string;
}

interface InboxMessageDetailPaneProps {
  effectFeedback: string | null;
  gameState: GameStateData;
  language: string;
  selectedMessage: GameStateData["messages"][number] | null;
  onAction: (messageId: string, actionId: string, optionId?: string) => void;
  onCloseSelectedMessage: () => void;
  onRequestDelete: () => void;
  onScoutPlayerClick: (playerId: string) => void;
}

export default function InboxMessageDetailPane({
  effectFeedback,
  gameState,
  language,
  selectedMessage,
  onAction,
  onCloseSelectedMessage,
  onRequestDelete,
  onScoutPlayerClick,
}: InboxMessageDetailPaneProps): JSX.Element {
  const { t } = useTranslation();
  const [pendingSwitch, setPendingSwitch] = useState<PendingSwitch | null>(
    null,
  );

  const currentClubName = getTeamName(
    gameState.teams,
    gameState.manager?.team_id ?? null,
  );

  const handleOptionClick = (
    messageId: string,
    actionId: string,
    optionId: string,
  ) => {
    const isSwitch =
      messageId.startsWith("job_offer_") &&
      optionId === "accept" &&
      !!gameState.manager?.team_id &&
      !!selectedMessage;
    if (!isSwitch) {
      onAction(messageId, actionId, optionId);
      return;
    }
    const newClubName = getTeamName(
      gameState.teams,
      selectedMessage.context?.team_id ?? null,
    );
    setPendingSwitch({ messageId, actionId, optionId, newClubName });
  };

  if (!selectedMessage) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <MailOpen className="w-12 h-12 text-gray-300 dark:text-navy-600 mx-auto mb-3" />
          <p className="text-sm text-gray-400 dark:text-gray-500 font-heading uppercase tracking-wider">
            {t("inbox.selectMessage")}
          </p>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="shrink-0 border-b border-gray-100 p-5 dark:border-navy-600">
        <button
          onClick={onCloseSelectedMessage}
          className="md:hidden flex items-center gap-1.5 text-xs text-gray-500 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 mb-3"
        >
          <ArrowLeft className="w-3.5 h-3.5" /> {t("inbox.backToInbox")}
        </button>
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-start gap-3 min-w-0 flex-1">
            <div
              className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${getCategoryColor(selectedMessage.category)} bg-primary-500/10 dark:bg-primary-500/20`}
            >
              {getCategoryIcon(selectedMessage.category)}
            </div>
            <div className="flex-1 min-w-0">
              <h3 className="font-heading font-bold text-lg text-gray-900 dark:text-gray-100">
                {selectedMessage.subject}
              </h3>
              <div className="flex items-center gap-3 mt-1">
                <span className="text-sm font-medium text-gray-600 dark:text-gray-300">
                  {selectedMessage.sender}
                  {selectedMessage.sender_role
                    ? ` — ${selectedMessage.sender_role}`
                    : ""}
                </span>
                <span className="text-xs text-gray-400 dark:text-gray-500">
                  {formatDateFull(selectedMessage.date, language)}
                </span>
              </div>
              <div className="flex items-center gap-2 mt-1.5">
                <Badge variant="neutral" size="sm">
                  {t(`inbox.categories.${selectedMessage.category}`)}
                </Badge>
                {selectedMessage.priority === "Urgent" ? (
                  <Badge variant="danger" size="sm">
                    {t("inbox.urgent")}
                  </Badge>
                ) : null}
                {selectedMessage.priority === "High" ? (
                  <Badge variant="accent" size="sm">
                    {t("inbox.important")}
                  </Badge>
                ) : null}
              </div>
            </div>
          </div>
          <Button
            type="button"
            size="sm"
            onClick={onRequestDelete}
            icon={<Trash2 className="w-4 h-4" />}
            className="bg-red-500 hover:bg-red-600 active:bg-red-700 focus:ring-red-500"
            data-testid="inbox-delete-message"
          >
            {t("inbox.deleteMessage", "Delete message")}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl">
          {selectedMessage.body
            .split("\n")
            .map((line, index) => renderMessageBodyLine(line, index))}

          <InboxDelegatedRenewalReport message={selectedMessage} />

          {selectedMessage.context?.scout_report ? (
            <ScoutPlayerCard
              report={selectedMessage.context.scout_report}
              onPlayerClick={onScoutPlayerClick}
            />
          ) : null}

          {selectedMessage.context?.match_result ? (
            <div className="mt-6 p-4 bg-gray-50 dark:bg-navy-700 rounded-xl flex items-center justify-center gap-8 border border-gray-100 dark:border-navy-600">
              <span className="font-heading font-bold text-sm text-gray-700 dark:text-gray-200">
                {getTeamName(
                  gameState.teams,
                  selectedMessage.context.match_result.home_team_id,
                )}
              </span>
              <span className="font-heading font-bold text-2xl text-gray-800 dark:text-gray-100">
                {selectedMessage.context.match_result.home_goals} -{" "}
                {selectedMessage.context.match_result.away_goals}
              </span>
              <span className="font-heading font-bold text-sm text-gray-700 dark:text-gray-200">
                {getTeamName(
                  gameState.teams,
                  selectedMessage.context.match_result.away_team_id,
                )}
              </span>
            </div>
          ) : null}

          {effectFeedback ? (
            <div className="mt-4 p-3 bg-primary-50 dark:bg-primary-500/10 border border-primary-200 dark:border-primary-500/30 rounded-xl flex items-center gap-2 animate-pulse">
              <CheckCircle2 className="w-4 h-4 text-primary-500 shrink-0" />
              <span className="text-sm font-medium text-primary-700 dark:text-primary-300">
                {t("inbox.effectOutcomeLabel", "Outcome")}: {effectFeedback}
              </span>
            </div>
          ) : null}

          {selectedMessage.actions.length > 0 ? (
            <div className="mt-6">
              {selectedMessage.actions.map((action) => {
                if (isChooseOptionAction(action.action_type)) {
                  const options = action.action_type.ChooseOption.options;

                  if (action.resolved) {
                    return (
                      <div
                        key={action.id}
                        className="flex items-center gap-2 text-sm text-gray-400 dark:text-gray-500 mt-2"
                      >
                        <CheckCircle2 className="w-4 h-4 text-primary-500" />
                        <span className="font-heading font-bold uppercase tracking-wider text-xs">
                          {t("inbox.responded", "Response sent")}
                        </span>
                      </div>
                    );
                  }

                  return (
                    <div key={action.id} className="space-y-2">
                      <p className="text-xs font-heading font-bold uppercase tracking-widest text-gray-400 dark:text-gray-500 flex items-center gap-1.5 mb-3">
                        <MessageCircle className="w-3.5 h-3.5" />
                        {isPlayerEventMessage(selectedMessage.id)
                          ? t(
                              "inbox.chooseResponseOutcomeVaries",
                              "Choose your response — outcome varies",
                            )
                          : t(
                              "inbox.chooseResponse",
                              "Choose your response",
                            )}
                      </p>
                      {options.map((option) => (
                        <button
                          key={option.id}
                          onClick={() =>
                            handleOptionClick(
                              selectedMessage.id,
                              action.id,
                              option.id,
                            )
                          }
                          className="w-full text-left p-4 rounded-xl border border-gray-200 dark:border-navy-600 hover:border-primary-400 dark:hover:border-primary-500 hover:bg-primary-50/50 dark:hover:bg-primary-500/5 transition-all group"
                        >
                          <p className="text-sm font-heading font-bold text-gray-800 dark:text-gray-200 group-hover:text-primary-600 dark:group-hover:text-primary-400 transition-colors">
                            {option.label}
                          </p>
                          <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                            {option.description}
                          </p>
                        </button>
                      ))}
                    </div>
                  );
                }

                return (
                  <button
                    key={action.id}
                    disabled={action.resolved}
                    onClick={() => onAction(selectedMessage.id, action.id)}
                    className={getActionButtonClassName(action)}
                  >
                    {action.resolved ? `✓ ${action.label}` : action.label}
                  </button>
                );
              })}
            </div>
          ) : null}
        </div>
      </div>
      <SwitchClubConfirmModal
        open={pendingSwitch !== null}
        currentClubName={currentClubName}
        newClubName={pendingSwitch?.newClubName ?? ""}
        onCancel={() => setPendingSwitch(null)}
        onConfirm={() => {
          if (!pendingSwitch) return;
          const { messageId, actionId, optionId } = pendingSwitch;
          setPendingSwitch(null);
          onAction(messageId, actionId, optionId);
        }}
      />
    </>
  );
}
