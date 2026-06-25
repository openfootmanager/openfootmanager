import { useTranslation } from "react-i18next";
import { EntityListShell, EntityRow } from "./shared";
import type { TeamDef } from "./types";

interface TeamsTabProps {
  teams: TeamDef[];
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

export function TeamsTab({ teams, onAdd, onEdit, onDelete, selectedIndex, onSelect }: TeamsTabProps) {
  const { t } = useTranslation();
  return (
    <EntityListShell
      addLabel={t("packageEditor.addTeam")}
      onAdd={onAdd}
      emptyLabel={t("packageEditor.noTeams")}
      isEmpty={teams.length === 0}
    >
      {teams.map((team, i) => (
        <EntityRow
          key={i}
          title={team.name}
          subtitle={[team.city, team.country].filter(Boolean).join(" · ")}
          badge={
            <div
              className="w-8 h-8 rounded-lg flex-shrink-0 border border-gray-200 dark:border-navy-600"
              style={{ background: team.colors.primary }}
            />
          }
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("packageEditor.editTeam")}
          deleteLabel={t("packageEditor.deleteTeam")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
