import {
  Briefcase,
  ClipboardList,
  Crosshair,
  DollarSign,
  Dumbbell,
  FileText,
  Info,
  Landmark,
  Newspaper,
  ScanSearch,
  Smile,
  Stethoscope,
  TableProperties,
  TrendingUp,
  Trophy,
} from "lucide-react";
import type { JSX, ReactNode } from "react";

import type { MessageAction, MessageData } from "../../store/gameStore";

export interface NavigateActionType {
  NavigateTo: { route: string };
}

export interface ChooseOptionActionType {
  ChooseOption: {
    options: { id: string; label: string; description: string }[];
  };
}

export interface NavigationTarget {
  tab: string;
  context?: { messageId?: string };
  shouldResolveAction: boolean;
}

export type MessageSortOrder = "newest" | "oldest";

export type DeleteModalState =
  | { mode: "single"; messageId: string; subject: string }
  | { mode: "bulk"; messageIds: string[] }
  | null;

export const UNREAD_FILTER = "__unread";

const FILTER_BUTTON_BASE_CLASS =
  "px-3 py-1.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all";
const FILTER_BUTTON_ACTIVE_CLASS = "bg-primary-500 text-white shadow-sm";
const FILTER_BUTTON_INACTIVE_CLASS =
  "bg-white dark:bg-navy-800 text-gray-500 dark:text-gray-400 border border-gray-200 dark:border-navy-600 hover:text-gray-700 dark:hover:text-gray-200";

const ROUTE_TAB_MAP: Record<string, string> = {
  squad: "Squad",
  tactics: "Tactics",
  training: "Training",
  schedule: "Schedule",
  finances: "Finances",
  transfers: "Transfers",
  players: "Players",
  teams: "Teams",
  tournaments: "Tournaments",
  staff: "Staff",
  inbox: "Inbox",
  manager: "Manager",
  home: "Home",
};

const PLAYER_EVENT_MESSAGE_PREFIXES = [
  "morale_talk_",
  "bench_complaint_",
  "happy_player_",
  "contract_concern_",
];

const CATEGORY_ICONS: Record<string, ReactNode> = {
  Welcome: <Trophy className="w-4 h-4" />,
  LeagueInfo: <ClipboardList className="w-4 h-4" />,
  MatchPreview: <Crosshair className="w-4 h-4" />,
  MatchResult: <TableProperties className="w-4 h-4" />,
  Transfer: <TrendingUp className="w-4 h-4" />,
  BoardDirective: <Landmark className="w-4 h-4" />,
  PlayerMorale: <Smile className="w-4 h-4" />,
  Injury: <Stethoscope className="w-4 h-4" />,
  Training: <Dumbbell className="w-4 h-4" />,
  Finance: <DollarSign className="w-4 h-4" />,
  Contract: <FileText className="w-4 h-4" />,
  ScoutReport: <ScanSearch className="w-4 h-4" />,
  Media: <Newspaper className="w-4 h-4" />,
  System: <Info className="w-4 h-4" />,
  JobOffer: <Briefcase className="w-4 h-4" />,
};

const CATEGORY_COLORS: Record<string, string> = {
  Welcome: "text-primary-500",
  LeagueInfo: "text-blue-500",
  MatchPreview: "text-accent-500",
  MatchResult: "text-accent-600",
  Transfer: "text-purple-500",
  BoardDirective: "text-red-500",
  PlayerMorale: "text-yellow-500",
  Injury: "text-red-400",
  Training: "text-green-500",
  Finance: "text-emerald-500",
  Contract: "text-indigo-500",
  ScoutReport: "text-cyan-500",
  Media: "text-orange-500",
  System: "text-gray-400",
  JobOffer: "text-blue-500",
};

export function getCategoryIcon(category: string): ReactNode {
  return CATEGORY_ICONS[category] ?? CATEGORY_ICONS.System;
}

export function getCategoryColor(category: string): string {
  return CATEGORY_COLORS[category] ?? CATEGORY_COLORS.System;
}

export function getFilterButtonClassName(
  isActive: boolean,
  extraClasses = "",
): string {
  let className = FILTER_BUTTON_BASE_CLASS;

  if (extraClasses.length > 0) {
    className = `${className} ${extraClasses}`;
  }

  if (isActive) {
    return `${className} ${FILTER_BUTTON_ACTIVE_CLASS}`;
  }

  return `${className} ${FILTER_BUTTON_INACTIVE_CLASS}`;
}

export function getFilteredMessages(
  messages: MessageData[],
  categoryFilter: string | null,
): MessageData[] {
  if (categoryFilter === UNREAD_FILTER) {
    return messages.filter((message) => !message.read);
  }

  if (categoryFilter) {
    return messages.filter((message) => message.category === categoryFilter);
  }

  return messages;
}

function getMessageDateValue(date: string): number {
  const value = Date.parse(date);

  if (Number.isNaN(value)) {
    return 0;
  }

  return value;
}

export function sortInboxMessages(
  messages: MessageData[],
  sortOrder: MessageSortOrder,
): MessageData[] {
  return [...messages].sort((leftMessage, rightMessage) => {
    const leftDateValue = getMessageDateValue(leftMessage.date);
    const rightDateValue = getMessageDateValue(rightMessage.date);

    if (sortOrder === "oldest") {
      return leftDateValue - rightDateValue;
    }

    return rightDateValue - leftDateValue;
  });
}

export function getListPaneClassName(hasSelectedMessage: boolean): string {
  const visibilityClassName = hasSelectedMessage ? "hidden md:flex" : "flex";

  return `${visibilityClassName} flex-col w-full md:w-96 md:min-w-[384px] border-r border-gray-200 dark:border-navy-600`;
}

export function getMessageRowClassName(
  isSelected: boolean,
  isRead: boolean,
): string {
  const baseClassName =
    "flex gap-3 px-4 py-3 cursor-pointer transition-colors border-b border-gray-100 dark:border-navy-600/50";

  if (isSelected) {
    return `${baseClassName} bg-primary-50 dark:bg-primary-500/10 border-l-2 border-l-primary-500`;
  }

  if (!isRead) {
    return `${baseClassName} bg-white dark:bg-navy-800 border-l-2 border-l-accent-500 hover:bg-gray-50 dark:hover:bg-navy-700/50`;
  }

  return `${baseClassName} border-l-2 border-l-transparent hover:bg-gray-50 dark:hover:bg-navy-700/30`;
}

export function getMessageIconClassName(
  categoryColor: string,
  isSelected: boolean,
  isRead: boolean,
): string {
  const baseClassName =
    "w-8 h-8 rounded-lg flex items-center justify-center shrink-0";

  if (isSelected) {
    return `${baseClassName} ${categoryColor} bg-primary-500/10`;
  }

  if (isRead) {
    return `${baseClassName} text-gray-400 bg-gray-100 dark:bg-navy-600`;
  }

  return `${baseClassName} ${categoryColor} bg-primary-500/10 dark:bg-primary-500/20`;
}

export function getMessageSubjectClassName(isRead: boolean): string {
  if (isRead) {
    return "text-sm truncate flex-1 font-medium text-gray-500 dark:text-gray-400";
  }

  return "text-sm truncate flex-1 font-bold text-gray-900 dark:text-gray-100";
}

export function getActionButtonClassName(action: MessageAction): string {
  const baseClassName =
    "px-5 py-2.5 rounded-lg text-xs font-heading font-bold uppercase tracking-wider transition-all mr-2 mb-2";

  if (action.resolved) {
    return `${baseClassName} bg-gray-100 dark:bg-navy-700 text-gray-400 dark:text-gray-500 cursor-default`;
  }

  if (
    action.action_type === "Acknowledge" ||
    action.action_type === "Dismiss"
  ) {
    return `${baseClassName} bg-gray-200 dark:bg-navy-600 text-gray-700 dark:text-gray-300 hover:bg-gray-300 dark:hover:bg-navy-500`;
  }

  return `${baseClassName} bg-primary-500 text-white hover:bg-primary-600 shadow-sm hover:shadow-md hover:shadow-primary-500/20`;
}

export function renderMessageBodyLine(line: string, index: number): JSX.Element {
  const baseClassName = "text-sm leading-relaxed mb-1";

  if (line.trim() === "") {
    return (
      <p key={index} className={`${baseClassName} h-3`}>
        {line}
      </p>
    );
  }

  if (line.startsWith("•")) {
    return (
      <p
        key={index}
        className={`${baseClassName} text-gray-700 dark:text-gray-300`}
      >
        <span className="flex items-start gap-2">
          <span className="text-primary-500 mt-0.5">•</span>
          <span>{line.replace("•", "").trim()}</span>
        </span>
      </p>
    );
  }

  return (
    <p
      key={index}
      className={`${baseClassName} text-gray-700 dark:text-gray-300`}
    >
      {line}
    </p>
  );
}

export function isNavigateAction(
  actionType: MessageAction["action_type"],
): actionType is NavigateActionType {
  return typeof actionType === "object" && "NavigateTo" in actionType;
}

export function isChooseOptionAction(
  actionType: MessageAction["action_type"],
): actionType is ChooseOptionActionType {
  return typeof actionType === "object" && "ChooseOption" in actionType;
}

export function getNavigationTarget(route: string): NavigationTarget {
  const teamMatch = route.match(/^\/team\/(.+)$/);

  if (teamMatch) {
    return {
      tab: "__selectTeam",
      context: { messageId: teamMatch[1] },
      shouldResolveAction: false,
    };
  }

  const playerMatch = route.match(/^\/player\/(.+)$/);

  if (playerMatch) {
    return {
      tab: "__selectPlayer",
      context: { messageId: playerMatch[1] },
      shouldResolveAction: false,
    };
  }

  const tabMatch = route.match(/[?&]tab=([^&]+)/i);

  if (tabMatch) {
    return {
      tab: tabMatch[1],
      shouldResolveAction: true,
    };
  }

  const simpleRoute = route.replace(/^\/+/, "").split(/[/?#]/)[0].toLowerCase();

  return {
    tab: ROUTE_TAB_MAP[simpleRoute] ?? "Home",
    shouldResolveAction: true,
  };
}

export function isPlayerEventMessage(messageId: string): boolean {
  return PLAYER_EVENT_MESSAGE_PREFIXES.some((prefix) =>
    messageId.startsWith(prefix),
  );
}
