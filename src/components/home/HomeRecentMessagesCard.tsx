import { useTranslation } from "react-i18next";

import { formatDateShort } from "../../lib/helpers";
import type { MessageData } from "../../store/gameStore";
import ContextMenu from "../ContextMenu";
import type { ContextMenuItem } from "../ContextMenu";
import { Card, CardBody, CardHeader } from "../ui";
import {
  buildOpenMessageMenuItem,
} from "../inbox/inboxContextMenuItems";

interface HomeRecentMessagesCardProps {
  messages: MessageData[];
  lang: string;
  onNavigate?: (tab: string, context?: { messageId?: string }) => void;
}

export default function HomeRecentMessagesCard({
  messages,
  lang,
  onNavigate,
}: HomeRecentMessagesCardProps) {
  const { t } = useTranslation();

  return (
    <Card>
      <CardHeader
        action={
          <button
            onClick={() => onNavigate?.("Inbox")}
            className="text-primary-500 dark:text-primary-400 text-xs font-heading font-bold uppercase tracking-wider hover:text-primary-600 dark:hover:text-primary-300 transition-colors"
          >
            {t("home.viewAll")}
          </button>
        }
      >
        {t("home.recentMessages")}
      </CardHeader>
      <CardBody className="p-0">
        <div className="divide-y divide-gray-100 dark:divide-navy-600">
          {messages.length === 0 ? (
            <p className="text-gray-500 dark:text-gray-400 p-6 text-sm">
              {t("home.noMessages")}
            </p>
          ) : (
            messages.map((message) => {
              const contextItems: ContextMenuItem[] = [
                buildOpenMessageMenuItem(t, () =>
                  onNavigate?.("Inbox", { messageId: message.id }),
                ),
                {
                  label: t("home.viewAll"),
                  onClick: () => onNavigate?.("Inbox"),
                },
              ];

              return (
                <ContextMenu items={contextItems} key={message.id}>
                  <div
                    onClick={() => onNavigate?.("Inbox", { messageId: message.id })}
                    className={`flex gap-4 px-6 py-3.5 hover:bg-gray-50 dark:hover:bg-navy-600/50 cursor-pointer transition-colors ${!message.read ? "border-l-4 border-l-primary-500" : "border-l-4 border-l-transparent"
                      }`}
                  >
                    <div
                      className={`w-9 h-9 rounded-lg flex items-center justify-center flex-shrink-0 font-heading font-bold text-sm ${message.read
                          ? "bg-gray-100 dark:bg-navy-600 text-gray-400 dark:text-gray-500"
                          : "bg-primary-500/10 dark:bg-primary-500/20 text-primary-600 dark:text-primary-400"
                        }`}
                    >
                      {message.sender.charAt(0)}
                    </div>
                    <div className="min-w-0 flex-1">
                      <h4
                        className={`font-semibold text-sm ${message.read ? "text-gray-500 dark:text-gray-400" : "text-gray-900 dark:text-gray-100"
                          }`}
                      >
                        {message.subject}
                      </h4>
                      <p
                        className={`text-xs truncate mt-0.5 ${message.read ? "text-gray-400 dark:text-gray-500" : "text-gray-600 dark:text-gray-300"
                          }`}
                      >
                        {message.body}
                      </p>
                    </div>
                    <span className="text-[10px] text-gray-400 dark:text-gray-500 flex-shrink-0 mt-1">
                      {formatDateShort(message.date, lang)}
                    </span>
                  </div>
                </ContextMenu>
              );
            })
          )}
        </div>
      </CardBody>
    </Card>
  );
}