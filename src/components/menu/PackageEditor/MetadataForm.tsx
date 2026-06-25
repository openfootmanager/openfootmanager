import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LabeledInput, LabeledSelect, labelClass, inputClass } from "./primitives";
import { PACKAGE_TYPES } from "./helpers";
import type { WorldMetaDef } from "./types";

const SPDX_LICENSES = [
  { id: "CC0-1.0",      name: "CC0 1.0 Public Domain" },
  { id: "CC-BY-4.0",   name: "CC BY 4.0" },
  { id: "CC-BY-SA-4.0",name: "CC BY-SA 4.0" },
  { id: "CC-BY-NC-4.0",name: "CC BY-NC 4.0" },
  { id: "MIT",          name: "MIT" },
  { id: "Apache-2.0",  name: "Apache 2.0" },
  { id: "GPL-2.0-only",name: "GPL 2.0" },
  { id: "__custom__",  name: "Custom / Other" },
];

const LICENSE_DESCRIPTIONS: Record<string, string> = {
  "CC0-1.0": "No rights reserved — anyone can use, modify and distribute for any purpose with no attribution required.",
  "CC-BY-4.0": "Free to use and redistribute. Attribution to the author required.",
  "CC-BY-SA-4.0": "Free to use. Attribution required. Derivative works must use the same license.",
  "CC-BY-NC-4.0": "Free for non-commercial use with attribution. Commercial use prohibited.",
  "MIT": "Very permissive. Attribution required. Can be used commercially.",
  "Apache-2.0": "Permissive with patent protection. Attribution required.",
  "GPL-2.0-only": "Copyleft — modifications must also be open-source under GPL.",
};

interface MetadataFormProps {
  meta: WorldMetaDef;
  onChange: (m: WorldMetaDef) => void;
}

export function MetadataForm({ meta, onChange }: MetadataFormProps) {
  const { t } = useTranslation();
  const set = (patch: Partial<WorldMetaDef>) => onChange({ ...meta, ...patch });

  const isKnownLicense = SPDX_LICENSES.some(
    (l) => l.id !== "__custom__" && l.id === meta.license,
  );
  const [useCustom, setUseCustom] = useState(!isKnownLicense && meta.license !== "");

  const packageTypeLabels: Record<string, string> = {
    database: t("worldEditor.typeDatabase"),
    patch: t("worldEditor.typePatch"),
    assets: t("worldEditor.typeAssets"),
  };

  const packageTypeHelp = t("worldEditor.help.packageType");

  function handleLicenseSelect(val: string) {
    if (val === "__custom__") {
      setUseCustom(true);
    } else {
      setUseCustom(false);
      set({ license: val });
    }
  }

  const selectedLicenseKey = useCustom
    ? "__custom__"
    : (SPDX_LICENSES.find((l) => l.id === meta.license)?.id ?? "__custom__");

  const licenseDesc = LICENSE_DESCRIPTIONS[meta.license];

  return (
    <div className="flex flex-col gap-3">
      <LabeledInput
        label={t("worldEditor.packageId")}
        value={meta.id}
        onChange={(v) => set({ id: v })}
        placeholder="my-world"
        help={t("worldEditor.help.packageId")}
      />
      <LabeledInput
        label={t("worldEditor.packageName")}
        value={meta.name}
        onChange={(v) => set({ name: v })}
      />
      <LabeledInput
        label={t("worldEditor.description")}
        value={meta.description}
        onChange={(v) => set({ description: v })}
      />
      <div className="grid grid-cols-2 gap-3">
        <LabeledInput
          label={t("worldEditor.version")}
          value={meta.version}
          onChange={(v) => set({ version: v })}
          placeholder="1.0.0"
        />
        <LabeledInput
          label={t("worldEditor.baseYear")}
          value={meta.baseYear?.toString() ?? ""}
          type="number"
          onChange={(v) => set({ baseYear: v === "" ? null : parseInt(v) })}
          placeholder="2026"
        />
      </div>
      <LabeledInput
        label={t("worldEditor.author")}
        value={meta.author}
        onChange={(v) => set({ author: v })}
      />

      {/* License picker */}
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("worldEditor.license")}</label>
        <LabeledSelect
          label=""
          value={selectedLicenseKey}
          options={SPDX_LICENSES.map((l) => l.id)}
          optionLabels={Object.fromEntries(SPDX_LICENSES.map((l) => [l.id, l.name]))}
          onChange={handleLicenseSelect}
        />
        {useCustom && (
          <input
            type="text"
            value={meta.license}
            onChange={(e) => set({ license: e.target.value })}
            placeholder="e.g. Proprietary"
            className={inputClass}
          />
        )}
        {licenseDesc && !useCustom && (
          <p className="text-[11px] text-gray-400 dark:text-gray-500 leading-relaxed">
            {licenseDesc}
          </p>
        )}
      </div>

      <LabeledSelect
        label={t("worldEditor.packageType")}
        value={meta.packageType}
        options={PACKAGE_TYPES}
        optionLabels={packageTypeLabels}
        onChange={(v) => set({ packageType: v })}
        help={packageTypeHelp}
      />
      <LabeledInput
        label={t("worldEditor.gameMinVersion")}
        value={meta.gameMinVersion}
        onChange={(v) => set({ gameMinVersion: v })}
        placeholder="0.3.0"
      />
    </div>
  );
}
