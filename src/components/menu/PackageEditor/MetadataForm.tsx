import { useTranslation } from "react-i18next";
import { LabeledInput, LabeledSelect } from "./primitives";
import { PACKAGE_TYPES } from "./helpers";
import type { WorldMetaDef } from "./types";

interface MetadataFormProps {
  meta: WorldMetaDef;
  onChange: (m: WorldMetaDef) => void;
}

export function MetadataForm({ meta, onChange }: MetadataFormProps) {
  const { t } = useTranslation();
  const set = (patch: Partial<WorldMetaDef>) => onChange({ ...meta, ...patch });

  return (
    <div className="flex flex-col gap-3">
      <LabeledInput
        label={t("packageEditor.packageId")}
        value={meta.id}
        onChange={(v) => set({ id: v })}
        placeholder="my-package"
      />
      <LabeledInput
        label={t("packageEditor.packageName")}
        value={meta.name}
        onChange={(v) => set({ name: v })}
      />
      <LabeledInput
        label={t("packageEditor.description")}
        value={meta.description}
        onChange={(v) => set({ description: v })}
      />
      <div className="grid grid-cols-2 gap-3">
        <LabeledInput
          label={t("packageEditor.version")}
          value={meta.version}
          onChange={(v) => set({ version: v })}
          placeholder="1.0.0"
        />
        <LabeledInput
          label={t("packageEditor.baseYear")}
          value={meta.baseYear?.toString() ?? ""}
          type="number"
          onChange={(v) => set({ baseYear: v === "" ? null : parseInt(v) })}
          placeholder="2026"
        />
      </div>
      <LabeledInput
        label={t("packageEditor.author")}
        value={meta.author}
        onChange={(v) => set({ author: v })}
      />
      <LabeledInput
        label={t("packageEditor.license")}
        value={meta.license}
        onChange={(v) => set({ license: v })}
        placeholder="CC-BY-4.0"
      />
      <LabeledSelect
        label={t("packageEditor.packageType")}
        value={meta.packageType}
        options={PACKAGE_TYPES}
        onChange={(v) => set({ packageType: v })}
      />
      <LabeledInput
        label={t("packageEditor.gameMinVersion")}
        value={meta.gameMinVersion}
        onChange={(v) => set({ gameMinVersion: v })}
        placeholder="0.3.0"
      />
    </div>
  );
}
