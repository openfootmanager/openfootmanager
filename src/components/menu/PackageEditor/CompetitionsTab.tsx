import { useTranslation } from "react-i18next";
import { useAssetDataUrl } from "../../../hooks/useAssetDataUrl";
import { EntityListShell, EntityRow } from "./shared";
import type { CompetitionDef } from "./types";

interface CompetitionsTabProps {
  competitions: CompetitionDef[];
  projectDir?: string;
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

function CompetitionBadge({ comp, projectDir }: { comp: CompetitionDef; projectDir?: string }) {
  const logoUrl = useAssetDataUrl(comp.logo, projectDir);

  if (logoUrl) {
    return (
      <img
        src={logoUrl}
        alt=""
        className="w-8 h-8 rounded-lg object-contain border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 flex-shrink-0"
      />
    );
  }
  return (
    <div className="w-8 h-8 rounded-lg flex-shrink-0 border border-gray-200 dark:border-navy-600 bg-gray-100 dark:bg-navy-600 flex items-center justify-center">
      <span className="text-[8px] font-heading font-bold text-gray-500 dark:text-gray-400 text-center leading-tight px-0.5">
        {comp.type.slice(0, 3).toUpperCase()}
      </span>
    </div>
  );
}

export function CompetitionsTab({ competitions, projectDir, onAdd, onEdit, onDelete, selectedIndex, onSelect }: CompetitionsTabProps) {
  const { t } = useTranslation();
  return (
    <EntityListShell
      addLabel={t("worldEditor.addCompetition")}
      onAdd={onAdd}
      emptyLabel={t("worldEditor.noCompetitions")}
      isEmpty={competitions.length === 0}
    >
      {competitions.map((comp, i) => (
        <EntityRow
          key={comp.id}
          title={comp.name || comp.id}
          subtitle={[t(`teamSelect.kinds.${comp.type}`), t(`teamSelect.scopes.${comp.scope}`)]
            .join(" · ")}
          badge={<CompetitionBadge comp={comp} projectDir={projectDir} />}
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("worldEditor.editCompetition")}
          deleteLabel={t("worldEditor.deleteCompetition")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
