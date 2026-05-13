import { useEffect, useMemo, useState } from "react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";

import type { GameStateData } from "../../store/gameStore";
import {
  clearOldMessages,
  deleteMessage,
  deleteMessages,
  markAllMessagesRead,
  markMessageRead,
  resolveMessageAction,
} from "../../services/inboxService";
import { resolveBackendText, resolveMessage } from "../../utils/backendI18n";
import InboxDeleteConfirmModal from "./InboxDeleteConfirmModal";
import InboxMessageDetailPane from "./InboxMessageDetailPane";
import InboxMessageListPane from "./InboxMessageListPane";
import InboxToolbar from "./InboxToolbar";
import {
  type DeleteModalState,
  getFilteredMessages,
  getNavigationTarget,
  isNavigateAction,
  type MessageSortOrder,
  sortInboxMessages,
} from "./inboxHelpers";

interface InboxTabProps {
  gameState: GameStateData;
  onGameUpdate: (g: GameStateData) => void;
  initialMessageId?: string | null;
  onNavigate?: (tab: string, context?: { messageId?: string }) => void;
}

export default function InboxTab({
  gameState,
  onGameUpdate,
  initialMessageId,
  onNavigate,
}: InboxTabProps): JSX.Element {
  const { i18n } = useTranslation();
  const messages = gameState.messages ?? [];
  const allMessages = useMemo(() => messages.map(resolveMessage), [messages]);
  const [selectedMessageId, setSelectedMessageId] = useState<string | null>(
    initialMessageId ?? null,
  );
  const [categoryFilter, setCategoryFilter] = useState<string | null>(null);
  const [sortOrder, setSortOrder] = useState<MessageSortOrder>("newest");
  const [bulkSelectionEnabled, setBulkSelectionEnabled] = useState(false);
  const [selectedMessageIds, setSelectedMessageIds] = useState<string[]>([]);
  const [deleteModalState, setDeleteModalState] =
    useState<DeleteModalState>(null);
  const [effectFeedback, setEffectFeedback] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  const categoryCounts = useMemo(() => {
    const counts = new Map<string, number>();

    for (const message of allMessages) {
      const currentCount = counts.get(message.category) ?? 0;
      counts.set(message.category, currentCount + 1);
    }

    return counts;
  }, [allMessages]);

  const categories = useMemo(
    () => Array.from(categoryCounts.keys()),
    [categoryCounts],
  );

  const filteredMessages = useMemo(
    () =>
      sortInboxMessages(
        getFilteredMessages(allMessages, categoryFilter),
        sortOrder,
      ),
    [allMessages, categoryFilter, sortOrder],
  );

  const unreadCount = useMemo(
    () => allMessages.filter((message) => !message.read).length,
    [allMessages],
  );

  const selectedMessage = useMemo(
    () =>
      allMessages.find((message) => message.id === selectedMessageId) ?? null,
    [allMessages, selectedMessageId],
  );

  useEffect(() => {
    const availableMessageIds = new Set(
      allMessages.map((message) => message.id),
    );

    setSelectedMessageIds((currentIds) =>
      currentIds.filter((messageId) => availableMessageIds.has(messageId)),
    );

    if (selectedMessageId && !availableMessageIds.has(selectedMessageId)) {
      setSelectedMessageId(null);
    }
  }, [allMessages, selectedMessageId]);

  async function handleSelectMessage(messageId: string): Promise<void> {
    setSelectedMessageId(messageId);
    const message = allMessages.find(
      (currentMessage) => currentMessage.id === messageId,
    );

    if (message && !message.read) {
      try {
        const updatedGameState = await markMessageRead(messageId);
        onGameUpdate(updatedGameState);
      } catch { }
    }
  }

  async function handleAction(
    messageId: string,
    actionId: string,
    optionId?: string,
  ): Promise<void> {
    const message = allMessages.find(
      (currentMessage) => currentMessage.id === messageId,
    );
    const action = message?.actions.find(
      (currentAction) => currentAction.id === actionId,
    );

    if (action && isNavigateAction(action.action_type)) {
      const navigationTarget = getNavigationTarget(
        action.action_type.NavigateTo.route,
      );
      onNavigate?.(navigationTarget.tab, navigationTarget.context);

      if (!navigationTarget.shouldResolveAction) {
        return;
      }
    }

    try {
      const result = await resolveMessageAction(messageId, actionId, optionId);

      onGameUpdate(result.game);

      if (result.effect) {
        const effectParams = result.effect_i18n_params
          ? Object.fromEntries(
            Object.entries(result.effect_i18n_params).map(([key, value]) => [
              key,
              String(value),
            ]),
          )
          : undefined;
        const resolvedEffect = resolveBackendText(
          result.effect_i18n_key ?? undefined,
          result.effect,
          effectParams,
        );
        setEffectFeedback(resolvedEffect);
        setTimeout(() => setEffectFeedback(null), 4000);
      }
    } catch { }
  }

  async function handleMarkAllRead(): Promise<void> {
    try {
      const updatedGameState = await markAllMessagesRead();
      onGameUpdate(updatedGameState);
    } catch { }
  }

  async function handleClearOld(): Promise<void> {
    try {
      const updatedGameState = await clearOldMessages();
      onGameUpdate(updatedGameState);
      setSelectedMessageId(null);
    } catch { }
  }

  async function handleConfirmDelete(): Promise<void> {
    if (!deleteModalState) {
      return;
    }

    setIsDeleting(true);

    try {
      let updatedGameState: GameStateData;
      let deletedMessageIds: string[];

      if (deleteModalState.mode === "single") {
        deletedMessageIds = [deleteModalState.messageId];
        updatedGameState = await deleteMessage(deleteModalState.messageId);
      } else {
        deletedMessageIds = deleteModalState.messageIds;
        updatedGameState = await deleteMessages(deleteModalState.messageIds);
        setBulkSelectionEnabled(false);
      }

      onGameUpdate(updatedGameState);
      setSelectedMessageIds((currentIds) =>
        currentIds.filter((messageId) => !deletedMessageIds.includes(messageId)),
      );

      if (selectedMessageId && deletedMessageIds.includes(selectedMessageId)) {
        setSelectedMessageId(null);
      }

      setDeleteModalState(null);
    } catch {
    } finally {
      setIsDeleting(false);
    }
  }

  function handleCloseDeleteModal(): void {
    if (isDeleting) {
      return;
    }

    setDeleteModalState(null);
  }

  function handleSortOrderChange(nextSortOrder: MessageSortOrder): void {
    setSortOrder(nextSortOrder);
  }

  function handleToggleBulkSelectionMode(): void {
    setBulkSelectionEnabled((currentValue) => {
      if (currentValue) {
        setSelectedMessageIds([]);
      }

      return !currentValue;
    });
  }

  function handleToggleMessageSelection(messageId: string): void {
    setSelectedMessageIds((currentIds) => {
      if (currentIds.includes(messageId)) {
        return currentIds.filter((currentId) => currentId !== messageId);
      }

      return [...currentIds, messageId];
    });
  }

  function handleRequestBulkDelete(): void {
    if (selectedMessageIds.length === 0) {
      return;
    }

    setDeleteModalState({
      mode: "bulk",
      messageIds: selectedMessageIds,
    });
  }

  function handleRequestSingleDelete(): void {
    if (!selectedMessage) {
      return;
    }

    setDeleteModalState({
      mode: "single",
      messageId: selectedMessage.id,
      subject: selectedMessage.subject,
    });
  }

  function handleRequestDeleteMessage(messageId: string, subject: string): void {
    setDeleteModalState({
      mode: "single",
      messageId,
      subject,
    });
  }

  function handleShowAll(): void {
    setCategoryFilter(null);
  }

  function handleShowUnread(): void {
    setCategoryFilter("__unread");
  }

  function handleToggleCategory(category: string): void {
    setCategoryFilter((currentFilter) => {
      if (currentFilter === category) {
        return null;
      }

      return category;
    });
  }

  function handleCloseSelectedMessage(): void {
    setSelectedMessageId(null);
  }

  function handleScoutPlayerClick(playerId: string): void {
    onNavigate?.("__selectPlayer", { messageId: playerId });
  }

  return (
    <div className="max-w-6xl mx-auto flex flex-col h-full">
      <InboxToolbar
        allMessagesCount={allMessages.length}
        bulkSelectionEnabled={bulkSelectionEnabled}
        categories={categories}
        categoryCounts={categoryCounts}
        categoryFilter={categoryFilter}
        selectedMessageCount={selectedMessageIds.length}
        sortOrder={sortOrder}
        unreadCount={unreadCount}
        onClearOld={() => {
          void handleClearOld();
        }}
        onDeleteSelected={handleRequestBulkDelete}
        onMarkAllRead={() => {
          void handleMarkAllRead();
        }}
        onShowAll={handleShowAll}
        onShowUnread={handleShowUnread}
        onSortOrderChange={handleSortOrderChange}
        onToggleBulkSelectionMode={handleToggleBulkSelectionMode}
        onToggleCategory={handleToggleCategory}
      />

      <div className="flex-1 flex gap-0 rounded-xl overflow-hidden border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 min-h-0">
        <InboxMessageListPane
          bulkSelectionEnabled={bulkSelectionEnabled}
          filteredMessages={filteredMessages}
          hasSelectedMessage={selectedMessage !== null}
          language={i18n.language}
          selectedMessageId={selectedMessageId}
          selectedMessageIds={selectedMessageIds}
          onRequestDeleteMessage={(message) => {
            handleRequestDeleteMessage(message.id, message.subject);
          }}
          onRequestMarkMessageRead={(messageId) => {
            void handleSelectMessage(messageId);
          }}
          onSelectMessage={(messageId) => {
            void handleSelectMessage(messageId);
          }}
          onToggleMessageSelection={handleToggleMessageSelection}
        />

        <div className="flex-1 flex flex-col min-w-0">
          <InboxMessageDetailPane
            effectFeedback={effectFeedback}
            gameState={gameState}
            language={i18n.language}
            selectedMessage={selectedMessage}
            onAction={(messageId, actionId, optionId) => {
              void handleAction(messageId, actionId, optionId);
            }}
            onCloseSelectedMessage={handleCloseSelectedMessage}
            onRequestDelete={handleRequestSingleDelete}
            onScoutPlayerClick={handleScoutPlayerClick}
          />
        </div>
      </div>

      <InboxDeleteConfirmModal
        deleteModalState={deleteModalState}
        isDeleting={isDeleting}
        onCancel={handleCloseDeleteModal}
        onConfirm={() => {
          void handleConfirmDelete();
        }}
      />
    </div>
  );
}
