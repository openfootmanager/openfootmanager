import { useTranslation } from "react-i18next";
import { LabeledInput } from "./primitives";
import { EntityFormShell } from "./shared";
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
  return (
    <EntityFormShell
      title={editingIndex === null ? t("packageEditor.addConfederation") : t("packageEditor.editConfederation")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id || !editing.name}
      saveLabel={t("packageEditor.saveConfederation")}
    >
      <LabeledInput
        label={t("packageEditor.confederationId")}
        value={editing.id}
        onChange={(v) => updateField("id", v)}
        placeholder="europe"
      />
      <LabeledInput
        label={t("packageEditor.confederationName")}
        value={editing.name}
        onChange={(v) => updateField("name", v)}
        placeholder="Europe"
      />
    </EntityFormShell>
  );
}
