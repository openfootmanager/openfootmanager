import { useTranslation } from "react-i18next";
import { EntityListShell, EntityRow } from "./shared";
import type { ConfederationDef } from "./types";

interface ConfederationsTabProps {
  confederations: ConfederationDef[];
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

export function ConfederationsTab({ confederations, onAdd, onEdit, onDelete, selectedIndex, onSelect }: ConfederationsTabProps) {
  const { t } = useTranslation();
  return (
    <EntityListShell
      addLabel={t("packageEditor.addConfederation")}
      onAdd={onAdd}
      emptyLabel={t("packageEditor.noConfederations")}
      isEmpty={confederations.length === 0}
    >
      {confederations.map((conf, i) => (
        <EntityRow
          key={i}
          title={conf.name || conf.id}
          subtitle={conf.name ? conf.id : undefined}
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("packageEditor.editConfederation")}
          deleteLabel={t("packageEditor.deleteConfederation")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
