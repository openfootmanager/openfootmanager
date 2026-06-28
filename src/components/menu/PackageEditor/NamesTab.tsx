import { useTranslation } from "react-i18next";
import { EntityListShell, EntityRow } from "./shared";
import type { NamesDefinition } from "./types";

interface NamesTabProps {
  names: NamesDefinition;
  onAdd: () => void;
  onEdit: (key: string) => void;
  onDelete: (key: string) => void;
  selectedKey?: string | null;
  onSelect?: (key: string) => void;
}

export function NamesTab({ names, onAdd, onEdit, onDelete, selectedKey, onSelect }: NamesTabProps) {
  const { t } = useTranslation();
  const poolKeys = Object.keys(names.pools);
  return (
    <EntityListShell
      addLabel={t("worldEditor.addPool")}
      onAdd={onAdd}
      emptyLabel={t("worldEditor.noPools")}
      isEmpty={poolKeys.length === 0}
    >
      {poolKeys.map((key) => {
        const pool = names.pools[key];
        return (
          <EntityRow
            key={key}
            title={key}
            subtitle={`${pool.first_names.length} first · ${pool.last_names.length} last`}
            onEdit={() => onEdit(key)}
            onDelete={() => onDelete(key)}
            editLabel={t("worldEditor.editPool")}
            deleteLabel={t("worldEditor.deletePool")}
            isSelected={selectedKey === key}
            onClick={onSelect ? () => onSelect(key) : undefined}
          />
        );
      })}
    </EntityListShell>
  );
}
