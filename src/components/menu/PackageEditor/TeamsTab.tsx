import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import { GeneratedCrest } from "../../ui/GeneratedCrest";
import { useAssetDataUrl } from "../../../hooks/useAssetDataUrl";
import { EntityListShell, EntityRow } from "./shared";
import type { TeamDef } from "./types";

interface TeamsTabProps {
  teams: TeamDef[];
  projectDir?: string;
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

function TeamBadge({ team, projectDir }: { team: TeamDef; projectDir?: string }) {
  const logoUrl = useAssetDataUrl(team.logo, projectDir);

  if (logoUrl) {
    return (
      <img
        src={logoUrl}
        alt=""
        className="w-9 h-9 rounded-lg object-contain border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 flex-shrink-0"
      />
    );
  }
  return (
    <GeneratedCrest
      name={team.name || team.id}
      label={team.shortName || team.name?.slice(0, 3) || "?"}
      colors={team.colors}
      className="w-9 h-9"
    />
  );
}

export function TeamsTab({ teams, projectDir, onAdd, onEdit, onDelete, selectedIndex, onSelect }: TeamsTabProps) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");

  const q = query.trim().toLowerCase();
  const filtered = q
    ? teams.map((team, i) => ({ team, i })).filter(({ team }) =>
        team.name.toLowerCase().includes(q) ||
        team.city.toLowerCase().includes(q) ||
        team.country.toLowerCase().includes(q) ||
        team.id.toLowerCase().includes(q)
      )
    : teams.map((team, i) => ({ team, i }));

  return (
    <EntityListShell
      addLabel={t("worldEditor.addTeam")}
      onAdd={onAdd}
      emptyLabel={t("worldEditor.noTeams")}
      isEmpty={teams.length === 0}
      searchSlot={
        teams.length > 0 && (
          <div className="relative">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-gray-400 dark:text-gray-500 pointer-events-none" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              aria-label={t("worldEditor.searchTeams")}
              placeholder={t("worldEditor.searchTeams")}
              className="w-full pl-8 pr-3 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-900 dark:text-white placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
            />
          </div>
        )
      }
    >
      {filtered.map(({ team, i }) => (
        <EntityRow
          key={team.id}
          title={team.name}
          subtitle={[team.city, team.country].filter(Boolean).join(" · ")}
          badge={<TeamBadge team={team} projectDir={projectDir} />}
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("worldEditor.editTeam")}
          deleteLabel={t("worldEditor.deleteTeam")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
