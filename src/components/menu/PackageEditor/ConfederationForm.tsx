import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LabeledInput } from "./primitives";
import { EntityFormShell } from "./shared";
import { toSlug } from "./helpers";
import type { ConfederationDef } from "./types";

interface ConfederationFormProps {
  editing: ConfederationDef;
  editingIndex: number | null;
  isBusy: boolean;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof ConfederationDef>(key: K, value: ConfederationDef[K]) => void;
}

export function ConfederationForm({
  editing,
  editingIndex,
  isBusy,
  onBack,
  onSave,
  updateField,
}: ConfederationFormProps) {
  const { t } = useTranslation();
  const [idAutoMode, setIdAutoMode] = useState(editingIndex === null && !editing.id);
  return (
    <EntityFormShell
      title={editingIndex === null ? t("worldEditor.addConfederation") : t("worldEditor.editConfederation")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id || !editing.name}
      saveLabel={t("worldEditor.saveConfederation")}
    >
      <LabeledInput
        label={t("worldEditor.confederationId")}
        value={editing.id}
        onChange={(v) => { setIdAutoMode(false); updateField("id", v); }}
        placeholder="europe"
      />
      <LabeledInput
        label={t("worldEditor.confederationName")}
        value={editing.name}
        onChange={(v) => {
          updateField("name", v);
          if (idAutoMode) updateField("id", toSlug(v));
        }}
        placeholder="Europe"
      />
    </EntityFormShell>
  );
}
