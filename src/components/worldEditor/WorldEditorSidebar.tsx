import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AlertCircle, Flag, Globe2, Info, Shield, Trophy, Type, Users, UserCog, GraduationCap } from "lucide-react";
import type { EditTab } from "../menu/PackageEditor/types";

interface SidebarItem {
  key: EditTab;
  label: string;
  count?: number;
  icon: LucideIcon;
}

interface SidebarGroup {
  groupKey: string;
  label: string;
  items: SidebarItem[];
}

interface WorldEditorSidebarProps {
  selectedSection: EditTab;
  onSelectSection: (section: EditTab) => void;
  confederationCount: number;
  countryCount: number;
  teamCount: number;
  playerCount: number;
  youthCount: number;
  staffCount: number;
  namePoolCount: number;
  competitionCount: number;
  issueCount: number;
  onShowIssues: () => void;
  showingIssues: boolean;
}

export function WorldEditorSidebar({
  selectedSection,
  onSelectSection,
  confederationCount,
  countryCount,
  teamCount,
  playerCount,
  youthCount,
  staffCount,
  namePoolCount,
  competitionCount,
  issueCount,
  onShowIssues,
  showingIssues,
}: WorldEditorSidebarProps) {
  const { t } = useTranslation();

  const groups: SidebarGroup[] = [
    {
      groupKey: "package",
      label: t("worldEditor.sectionPackage"),
      items: [{ key: "metadata", label: t("worldEditor.metadata"), icon: Info }],
    },
    {
      groupKey: "world",
      label: t("worldEditor.sectionWorld"),
      items: [
        { key: "confederations", label: t("worldEditor.sectionConfederations"), count: confederationCount, icon: Globe2 },
        { key: "countries", label: t("worldEditor.sectionCountries"), count: countryCount, icon: Flag },
      ],
    },
    {
      groupKey: "clubs",
      label: t("worldEditor.sectionClubs"),
      items: [
        { key: "teams", label: t("worldEditor.sectionTeams"), count: teamCount, icon: Shield },
        { key: "players", label: t("worldEditor.sectionPlayers"), count: playerCount, icon: Users },
        { key: "youth", label: t("worldEditor.sectionYouth"), count: youthCount, icon: GraduationCap },
        { key: "staff", label: t("worldEditor.sectionStaff"), count: staffCount, icon: UserCog },
        { key: "names", label: t("worldEditor.sectionNames"), count: namePoolCount, icon: Type },
      ],
    },
    {
      groupKey: "competitions",
      label: t("worldEditor.sectionCompetitions"),
      items: [
        { key: "competitions", label: t("worldEditor.sectionCompetitionsList"), count: competitionCount, icon: Trophy },
      ],
    },
  ];

  const itemClass = (active: boolean) =>
    `flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm transition-colors text-left ${
      active
        ? "bg-primary-100 dark:bg-primary-500/15 text-primary-700 dark:text-primary-300 font-semibold"
        : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-navy-700 hover:text-gray-900 dark:hover:text-white"
    }`;

  return (
    <div className="flex flex-col h-full py-3">
      <div className="flex-1 overflow-y-auto scrollbar-thin px-2">
        {groups.map((group) => (
          <div key={group.groupKey} className="mb-4">
            <p className="px-3 mb-1 text-[10px] font-heading font-bold uppercase tracking-[0.15em] text-gray-400 dark:text-gray-500">
              {group.label}
            </p>
            {group.items.map((item) => (
              <button
                key={item.key}
                onClick={() => onSelectSection(item.key)}
                className={itemClass(selectedSection === item.key && !showingIssues)}
              >
                <item.icon className="w-4 h-4 flex-shrink-0 opacity-70" />
                <span className="flex-1">{item.label}</span>
                {item.count !== undefined && (
                  <span
                    className={`text-xs rounded-full px-1.5 py-0.5 min-w-[20px] text-center leading-none ${
                      selectedSection === item.key && !showingIssues
                        ? "bg-primary-200 dark:bg-primary-500/30 text-primary-700 dark:text-primary-300"
                        : "bg-gray-200 dark:bg-navy-600 text-gray-500 dark:text-gray-400"
                    }`}
                  >
                    {item.count}
                  </span>
                )}
              </button>
            ))}
          </div>
        ))}
      </div>

      {/* Issues badge at bottom */}
      <div className="flex-shrink-0 px-2 pt-2 border-t border-gray-200 dark:border-navy-700">
        <button
          onClick={onShowIssues}
          className={`flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm transition-colors ${
            showingIssues
              ? "bg-red-50 dark:bg-red-500/10 text-red-600 dark:text-red-400"
              : issueCount > 0
              ? "text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-500/10"
              : "text-gray-400 dark:text-gray-500 hover:bg-gray-100 dark:hover:bg-navy-700"
          }`}
        >
          <AlertCircle className="w-4 h-4 flex-shrink-0" />
          {issueCount > 0
            ? t("worldEditor.issuesBadge", { count: issueCount })
            : t("worldEditor.noIssues")}
        </button>
      </div>
    </div>
  );
}
