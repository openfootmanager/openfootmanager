import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckCircle2, Info, XCircle, Globe, ImagePlus, X } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { LabeledInput, LabeledSelect, labelClass, inputClass } from "./primitives";
import { useAssetDataUrl, evictAssetDataUrl } from "../../../hooks/useAssetDataUrl";
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

interface LicenseDetails {
  permissions: string[];
  conditions: string[];
  limitations: string[];
}

const LICENSE_DETAILS: Record<string, LicenseDetails> = {
  "CC0-1.0": {
    permissions: ["Commercial use", "Modification", "Distribution", "Private use"],
    conditions: [],
    limitations: ["No liability", "No warranty"],
  },
  "CC-BY-4.0": {
    permissions: ["Commercial use", "Modification", "Distribution", "Private use"],
    conditions: ["Attribution"],
    limitations: ["No liability"],
  },
  "CC-BY-SA-4.0": {
    permissions: ["Commercial use", "Modification", "Distribution", "Private use"],
    conditions: ["Attribution", "Share-alike"],
    limitations: ["No liability"],
  },
  "CC-BY-NC-4.0": {
    permissions: ["Modification", "Distribution", "Private use"],
    conditions: ["Attribution", "Non-commercial only"],
    limitations: ["Commercial use", "No liability"],
  },
  "MIT": {
    permissions: ["Commercial use", "Modification", "Distribution", "Private use"],
    conditions: ["Attribution"],
    limitations: ["No liability", "No warranty"],
  },
  "Apache-2.0": {
    permissions: ["Commercial use", "Modification", "Distribution", "Private use", "Patent use"],
    conditions: ["Attribution", "State changes"],
    limitations: ["No trademark use", "No liability", "No warranty"],
  },
  "GPL-2.0-only": {
    permissions: ["Commercial use", "Modification", "Distribution", "Private use"],
    conditions: ["Disclose source", "Same license", "Attribution"],
    limitations: ["No liability", "No warranty"],
  },
};

interface EntityCounts {
  teams: number;
  players: number;
  confederations: number;
  countries: number;
  competitions: number;
  namePools: number;
}

interface MetadataFormProps {
  meta: WorldMetaDef;
  onChange: (m: WorldMetaDef) => void;
  counts?: EntityCounts;
  projectDir?: string;
}

export function MetadataForm({ meta, onChange, counts, projectDir }: MetadataFormProps) {
  const { t } = useTranslation();
  const set = (patch: Partial<WorldMetaDef>) => onChange({ ...meta, ...patch });
  const [logoRefresh, setLogoRefresh] = useState(0);
  const logoDataUrl = useAssetDataUrl(meta.logo, projectDir, logoRefresh);

  async function handlePickLogo() {
    if (!projectDir) return;
    const selected = await open({
      multiple: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    try {
      const relPath = await invoke<string>("copy_package_asset", {
        dir: projectDir,
        entityId: meta.id || "package-logo",
        srcPath: selected,
      });
      evictAssetDataUrl(projectDir, relPath);
      setLogoRefresh((k) => k + 1); // refresh even if the path is unchanged
      set({ logo: relPath });
    } catch { /* ignore */ }
  }

  const isKnownLicense = SPDX_LICENSES.some(
    (l) => l.id !== "__custom__" && l.id === meta.license,
  );
  const [useCustom, setUseCustom] = useState(!isKnownLicense && meta.license !== "");

  const packageTypeLabels: Record<string, string> = {
    database: t("worldEditor.typeDatabase"),
    patch: t("worldEditor.typePatch"),
    assets: t("worldEditor.typeAssets"),
  };

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

  const licenseDetails = LICENSE_DETAILS[meta.license];

  return (
    <div className="grid grid-cols-1 xl:grid-cols-2 gap-8">
      {/* Left: form fields */}
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
          multiline
          rows={3}
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

        {projectDir && (
          <div className="flex flex-col gap-1">
            <label className={labelClass}>{t("worldEditor.packageLogo")}</label>
            <div className="flex items-center gap-3">
              {logoDataUrl ? (
                <img src={logoDataUrl} alt="" className="w-12 h-12 rounded-lg object-contain border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 flex-shrink-0" />
              ) : (
                <div className="w-12 h-12 rounded-lg border border-dashed border-gray-300 dark:border-navy-600 bg-gray-50 dark:bg-navy-700 flex items-center justify-center flex-shrink-0">
                  <ImagePlus className="w-5 h-5 text-gray-300 dark:text-navy-500" />
                </div>
              )}
              <div className="flex gap-2">
                <button
                  type="button"
                  onClick={() => { void handlePickLogo(); }}
                  className="px-3 py-1.5 text-xs font-heading font-bold uppercase tracking-wide rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600 transition"
                >
                  {t("worldEditor.chooseLogo")}
                </button>
                {meta.logo && (
                  <button
                    type="button"
                    onClick={() => { set({ logo: null }); }}
                    className="px-2 py-1.5 text-xs rounded-lg border border-gray-200 dark:border-navy-600 text-gray-400 hover:text-red-500 dark:hover:text-red-400 transition"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                )}
              </div>
            </div>
          </div>
        )}

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
        </div>

        <LabeledSelect
          label={t("worldEditor.packageType")}
          value={meta.packageType}
          options={PACKAGE_TYPES}
          optionLabels={packageTypeLabels}
          onChange={(v) => set({ packageType: v })}
          help={t("worldEditor.help.packageType")}
        />
        <LabeledInput
          label={t("worldEditor.gameMinVersion")}
          value={meta.gameMinVersion}
          onChange={(v) => set({ gameMinVersion: v })}
          placeholder="0.3.0"
        />
      </div>

      {/* Right: preview card + license card */}
      <div className="flex flex-col gap-4">
        {/* Package preview card */}
        <div className="flex flex-col gap-1.5">
          <p className={labelClass}>{t("worldEditor.licensePreview")}</p>
          <div className="rounded-xl border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 p-4 flex items-start gap-3">
            {logoDataUrl ? (
              <img src={logoDataUrl} alt="" className="w-9 h-9 rounded-lg object-contain border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 flex-shrink-0" />
            ) : (
              <div className="w-9 h-9 rounded-lg bg-gradient-to-br from-primary-400 to-primary-600 flex items-center justify-center flex-shrink-0">
                <Globe className="w-5 h-5 text-white" />
              </div>
            )}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-1.5 flex-wrap">
                <p className="font-heading font-bold text-sm uppercase tracking-wide text-gray-800 dark:text-gray-200 truncate">
                  {meta.name || meta.id || "—"}
                </p>
                {meta.packageType && meta.packageType !== "database" && (
                  <span className="text-[9px] font-heading uppercase tracking-wider px-1.5 py-0.5 rounded bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-400 flex-shrink-0">
                    {packageTypeLabels[meta.packageType] ?? meta.packageType}
                  </span>
                )}
              </div>
              <p className="text-[10px] text-gray-400 dark:text-gray-500 mt-0.5">
                {meta.author && `${meta.author}`}
                {meta.author && meta.version && " · "}
                {meta.version && `v${meta.version}`}
                {(meta.author || meta.version) && meta.license && " · "}
                {meta.license && meta.license}
              </p>
              {meta.description && (
                <p className="text-xs text-gray-500 dark:text-gray-400 mt-1 line-clamp-2">
                  {meta.description}
                </p>
              )}
              {counts && (
                <div className="flex flex-wrap gap-1 mt-2">
                  {([
                    { n: counts.teams, key: "sectionTeams" },
                    { n: counts.players, key: "sectionPlayers" },
                    { n: counts.confederations, key: "sectionConfederations" },
                    { n: counts.countries, key: "sectionCountries" },
                    { n: counts.competitions, key: "sectionCompetitions" },
                    { n: counts.namePools, key: "sectionNames" },
                  ] as const)
                    .filter(({ n }) => n > 0)
                    .map(({ n, key }) => (
                      <span
                        key={key}
                        className="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 dark:bg-navy-700 text-gray-600 dark:text-gray-300"
                      >
                        {n} {t(`worldEditor.${key}`)}
                      </span>
                    ))}
                </div>
              )}
            </div>
          </div>
        </div>

        {/* License permissions card */}
        {licenseDetails && !useCustom && (
          <div className="flex flex-col gap-2 rounded-xl border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-800 p-4">
            <p className="text-[10px] font-heading font-bold uppercase tracking-[0.15em] text-gray-400 dark:text-gray-500">
              {SPDX_LICENSES.find((l) => l.id === meta.license)?.name}
            </p>

            {licenseDetails.permissions.length > 0 && (
              <div className="flex flex-col gap-1">
                <p className="text-[10px] font-heading font-bold uppercase tracking-[0.12em] text-green-600 dark:text-green-400">
                  {t("worldEditor.licensePermissions")}
                </p>
                {licenseDetails.permissions.map((p) => (
                  <div key={p} className="flex items-center gap-1.5">
                    <CheckCircle2 className="w-3.5 h-3.5 text-green-500 flex-shrink-0" />
                    <span className="text-xs text-gray-700 dark:text-gray-300">{p}</span>
                  </div>
                ))}
              </div>
            )}

            {licenseDetails.conditions.length > 0 && (
              <div className="flex flex-col gap-1">
                <p className="text-[10px] font-heading font-bold uppercase tracking-[0.12em] text-amber-600 dark:text-amber-400">
                  {t("worldEditor.licenseConditions")}
                </p>
                {licenseDetails.conditions.map((c) => (
                  <div key={c} className="flex items-center gap-1.5">
                    <Info className="w-3.5 h-3.5 text-amber-500 flex-shrink-0" />
                    <span className="text-xs text-gray-700 dark:text-gray-300">{c}</span>
                  </div>
                ))}
              </div>
            )}

            {licenseDetails.limitations.length > 0 && (
              <div className="flex flex-col gap-1">
                <p className="text-[10px] font-heading font-bold uppercase tracking-[0.12em] text-red-600 dark:text-red-400">
                  {t("worldEditor.licenseLimitations")}
                </p>
                {licenseDetails.limitations.map((l) => (
                  <div key={l} className="flex items-center gap-1.5">
                    <XCircle className="w-3.5 h-3.5 text-red-500 flex-shrink-0" />
                    <span className="text-xs text-gray-700 dark:text-gray-300">{l}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
