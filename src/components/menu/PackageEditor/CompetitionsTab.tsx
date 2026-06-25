import { useTranslation } from "react-i18next";
import { EntityListShell, EntityRow } from "./shared";
import type { CompetitionDef } from "./types";

interface CompetitionsTabProps {
  competitions: CompetitionDef[];
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

export function CompetitionsTab({ competitions, onAdd, onEdit, onDelete, selectedIndex, onSelect }: CompetitionsTabProps) {
  const { t } = useTranslation();
  return (
    <EntityListShell
      addLabel={t("packageEditor.addCompetition")}
      onAdd={onAdd}
      emptyLabel={t("packageEditor.noCompetitions")}
      isEmpty={competitions.length === 0}
    >
      {competitions.map((comp, i) => (
        <EntityRow
          key={i}
          title={comp.name || comp.id}
          subtitle={[t(`teamSelect.kinds.${comp.type}`), t(`teamSelect.scopes.${comp.scope}`)]
            .join(" · ")}
          badge={
            <div className="w-8 h-8 rounded-lg flex-shrink-0 border border-gray-200 dark:border-navy-600 bg-gray-100 dark:bg-navy-600 flex items-center justify-center">
              <span className="text-[8px] font-heading font-bold text-gray-500 dark:text-gray-400 text-center leading-tight px-0.5">
                {comp.type.slice(0, 3).toUpperCase()}
              </span>
            </div>
          }
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("packageEditor.editCompetition")}
          deleteLabel={t("packageEditor.deleteCompetition")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
