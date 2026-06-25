import { useState } from "react";
import { useTranslation } from "react-i18next";
import { LabeledInput } from "./primitives";
import { EntityFormShell } from "./shared";
import { poolToText, parsePoolText } from "./helpers";
import type { NamePool } from "./types";

interface NamesPoolFormProps {
  poolKey: string;
  pool: NamePool;
  isNew: boolean;
  isBusy: boolean;
  onBack: () => void;
  onSave: (key: string, pool: NamePool) => void;
}

export function NamesPoolForm({ poolKey, pool, isNew, isBusy, onBack, onSave }: NamesPoolFormProps) {
  const { t } = useTranslation();
  const [key, setKey] = useState(poolKey);
  const [firstText, setFirstText] = useState(poolToText(pool.first_names));
  const [lastText, setLastText] = useState(poolToText(pool.last_names));

  const labelClass =
    "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";
  const textareaClass =
    "w-full rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm font-mono text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition resize-none";

  function handleSave() {
    onSave(key.trim(), {
      first_names: parsePoolText(firstText),
      last_names: parsePoolText(lastText),
    });
  }

  return (
    <EntityFormShell
      title={isNew ? t("packageEditor.addPool") : t("packageEditor.editPool")}
      onBack={onBack}
      onSave={handleSave}
      isBusy={isBusy}
      saveDisabled={!key.trim()}
      saveLabel={t("packageEditor.savePool")}
    >
      <LabeledInput
        label={t("packageEditor.poolKey")}
        value={key}
        onChange={setKey}
        placeholder="ENG"
      />
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("packageEditor.poolFirstNames")}</label>
        <textarea
          rows={6}
          value={firstText}
          onChange={(e) => setFirstText(e.target.value)}
          className={textareaClass}
          placeholder={"James\nOliver\nHarry"}
        />
      </div>
      <div className="flex flex-col gap-1">
        <label className={labelClass}>{t("packageEditor.poolLastNames")}</label>
        <textarea
          rows={6}
          value={lastText}
          onChange={(e) => setLastText(e.target.value)}
          className={textareaClass}
          placeholder={"Smith\nJones\nWilliams"}
        />
      </div>
    </EntityFormShell>
  );
}
