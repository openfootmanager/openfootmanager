import { Mail, MailOpen } from "lucide-react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import { formatDateShort } from "../../lib/helpers";
import type { MessageData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import {
  buildDeleteMessageMenuItem,
  buildMarkMessageReadMenuItem,
  buildOpenMessageMenuItem,
} from "./inboxContextMenuItems";
import {
  getCategoryColor,
  getCategoryIcon,
  getListPaneClassName,
  getMessageIconClassName,
  getMessageRowClassName,
  getMessageSubjectClassName,
} from "./inboxHelpers";

interface InboxMessageListPaneProps {
  bulkSelectionEnabled: boolean;
  filteredMessages: MessageData[];
  hasSelectedMessage: boolean;
  language: string;
  selectedMessageId: string | null;
  selectedMessageIds: string[];
  onSelectMessage: (messageId: string) => void;
  onToggleMessageSelection: (messageId: string) => void;
  onRequestDeleteMessage: (message: MessageData) => void;
  onRequestMarkMessageRead: (messageId: string) => void;
}

export default function InboxMessageListPane({
  bulkSelectionEnabled,
  filteredMessages,
  hasSelectedMessage,
  language,
  selectedMessageId,
  selectedMessageIds,
  onSelectMessage,
  onToggleMessageSelection,
  onRequestDeleteMessage,
  onRequestMarkMessageRead,
}: InboxMessageListPaneProps): JSX.Element {
  const { t } = useTranslation();

  return (
    <div className={getListPaneClassName(hasSelectedMessage)}>
      <div className="bg-linear-to-r from-navy-700 to-navy-800 shrink-0 border-b border-gray-100 p-4 dark:border-navy-600">
        <h3 className="text-sm font-heading font-bold text-white flex items-center gap-2 uppercase tracking-wide">
          <Mail className="w-4 h-4 text-accent-400" />
          {t("inbox.title")}
        </h3>
        <p className="text-xs text-gray-400 mt-0.5 font-heading uppercase tracking-wider">
          {t("inbox.nMessages", { count: filteredMessages.length })}
        </p>
      </div>

      <div className="flex-1 overflow-y-auto">
        {filteredMessages.length === 0 ? (
          <div className="p-6 text-center">
            <MailOpen className="w-8 h-8 text-gray-300 dark:text-navy-600 mx-auto mb-2" />
            <p className="text-sm text-gray-400 dark:text-gray-500">
              {t("inbox.noMessages")}
            </p>
          </div>
        ) : (
          filteredMessages.map((message) => {
            const categoryIcon = getCategoryIcon(message.category);
            const categoryColor = getCategoryColor(message.category);
            const isSelected = selectedMessageId === message.id;
            const contextItems = [
              buildOpenMessageMenuItem(t, () => onSelectMessage(message.id)),
              buildMarkMessageReadMenuItem(t, message.read, () =>
                onRequestMarkMessageRead(message.id),
              ),
              buildDeleteMessageMenuItem(t, () => onRequestDeleteMessage(message)),
            ];

            return (
              <ContextMenu items={contextItems} key={message.id}>
                <div
                  onClick={() => onSelectMessage(message.id)}
                  className={getMessageRowClassName(isSelected, message.read)}
                  data-testid={`inbox-row-${message.id}`}
                >
                  {bulkSelectionEnabled ? (
                    <div
                      className="mt-1 flex shrink-0 items-center"
                      onClick={(event) => event.stopPropagation()}
                    >
                      <input
                        type="checkbox"
                        checked={selectedMessageIds.includes(message.id)}
                        onChange={() => onToggleMessageSelection(message.id)}
                        aria-label={t("inbox.selectMessageForDeletion", {
                          subject: message.subject,
                        })}
                        data-testid={`inbox-select-message-${message.id}`}
                        className="h-4 w-4 rounded border-gray-300 text-primary-500 focus:ring-primary-500/30"
                      />
                    </div>
                  ) : null}
                  <div
                    className={getMessageIconClassName(
                      categoryColor,
                      isSelected,
                      message.read,
                    )}
                  >
                    {categoryIcon}
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-1.5">
                      <h4 className={getMessageSubjectClassName(message.read)}>
                        {message.subject}
                      </h4>
                      {!message.read ? (
                        <span className="w-2 h-2 rounded-full bg-primary-500 shrink-0" />
                      ) : null}
                    </div>
                    <p className="text-xs text-gray-400 dark:text-gray-500 truncate mt-0.5">
                      {message.sender}
                    </p>
                    <p className="text-xs text-gray-400 dark:text-gray-500 mt-0.5">
                      {formatDateShort(message.date, language)}
                    </p>
                  </div>
                </div>
              </ContextMenu>
            );
          })
        )}
      </div>
    </div>
  );
}
