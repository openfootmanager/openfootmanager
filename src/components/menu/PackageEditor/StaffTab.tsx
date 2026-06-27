import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { EntityListShell, EntityRow } from "./shared";
import type { StaffDef, TeamDef } from "./types";

const ROLE_COLOR: Record<string, string> = {
  AssistantManager: "bg-purple-600",
  Coach: "bg-blue-600",
  Scout: "bg-amber-600",
  Physio: "bg-green-600",
};

const ROLE_ABBR: Record<string, string> = {
  AssistantManager: "AM",
  Coach: "CO",
  Scout: "SC",
  Physio: "PH",
};

interface StaffTabProps {
  staff: StaffDef[];
  teams?: TeamDef[];
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

export function StaffTab({ staff, teams, onAdd, onEdit, onDelete, selectedIndex, onSelect }: StaffTabProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");

  const q = query.trim().toLowerCase();
  const filtered = q
    ? staff.map((s, i) => ({ s, i })).filter(({ s }) => {
        const name = `${s.firstName} ${s.lastName}`.toLowerCase();
        return (
          name.includes(q) ||
          s.id.toLowerCase().includes(q) ||
          s.role.toLowerCase().includes(q) ||
          s.nationality.toLowerCase().includes(q) ||
          s.club.toLowerCase().includes(q)
        );
      })
    : staff.map((s, i) => ({ s, i }));

  return (
    <EntityListShell
      addLabel={t("worldEditor.addStaff")}
      onAdd={onAdd}
      emptyLabel={t("worldEditor.noStaff")}
      isEmpty={staff.length === 0}
      searchSlot={
        staff.length > 0 && (
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400 dark:text-gray-500 pointer-events-none" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              aria-label={t("worldEditor.searchStaff")}
              placeholder={t("worldEditor.searchStaff")}
              className="w-full pl-8 pr-3 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
            />
          </div>
        )
      }
    >
      {filtered.map(({ s, i }) => {
        const name = `${s.firstName} ${s.lastName}`.trim() || s.id;
        const roleColor = ROLE_COLOR[s.role] ?? "bg-gray-500";
        const roleAbbr = ROLE_ABBR[s.role] ?? s.role.slice(0, 2).toUpperCase();
        const clubName = s.club ? (teams?.find((t) => t.id === s.club)?.name ?? s.club) : null;
        return (
          <EntityRow
            key={s.id}
            title={name}
            subtitle={[
              t(`worldEditor.staffRole.${s.role}`, { defaultValue: s.role }),
              clubName,
            ].filter(Boolean).join(" · ")}
            badge={
              <div className={`flex items-center justify-center w-9 h-9 rounded-full text-white text-[10px] font-bold flex-shrink-0 ${roleColor}`}>
                {roleAbbr}
              </div>
            }
            onEdit={() => onEdit(i)}
            onDelete={() => onDelete(i)}
            editLabel={t("worldEditor.editStaff")}
            deleteLabel={t("worldEditor.deleteStaff")}
            isSelected={selectedIndex === i}
            onClick={onSelect ? () => onSelect(i) : undefined}
          />
        );
      })}
    </EntityListShell>
  );
}
