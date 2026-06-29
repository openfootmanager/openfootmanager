import { useState, useRef } from "react";
import { useTranslation } from "react-i18next";
import { X, Plus } from "lucide-react";
import { EntityFormShell } from "./shared";
import { CountryCombobox } from "../../ui/CountryCombobox";
import type { NamePool } from "./types";

interface NamesPoolFormProps {
  poolKey: string;
  pool: NamePool;
  isNew: boolean;
  isBusy: boolean;
  /// Keys of all existing pools, used to block renaming onto another pool.
  takenKeys: string[];
  onBack: () => void;
  onSave: (key: string, pool: NamePool) => void;
}

interface NameChipListProps {
  label: string;
  names: string[];
  addPlaceholder: string;
  addLabel: string;
  onChange: (names: string[]) => void;
}

function NameChipList({ label, names, addPlaceholder, addLabel, onChange }: NameChipListProps) {
  const [input, setInput] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const labelClass =
    "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";

  function add() {
    const trimmed = input.trim();
    if (!trimmed || names.includes(trimmed)) return;
    onChange([...names, trimmed]);
    setInput("");
    inputRef.current?.focus();
  }

  function remove(name: string) {
    onChange(names.filter((n) => n !== name));
  }

  return (
    <div className="flex flex-col gap-1.5">
      <label className={labelClass}>{label}</label>
      {names.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {names.map((name) => (
            <span
              key={name}
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-primary-100 dark:bg-primary-500/20 text-primary-700 dark:text-primary-300 text-xs font-medium"
            >
              {name}
              <button
                type="button"
                onClick={() => remove(name)}
                className="text-primary-400 hover:text-primary-700 dark:hover:text-primary-200 transition-colors"
                aria-label={`Remove ${name}`}
              >
                <X className="w-3 h-3" />
              </button>
            </span>
          ))}
        </div>
      )}
      <div className="flex gap-2">
        <input
          ref={inputRef}
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") { e.preventDefault(); add(); }
          }}
          placeholder={addPlaceholder}
          className="flex-1 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 px-3 py-2 text-sm text-gray-900 dark:text-white focus:outline-none focus:ring-2 focus:ring-primary-400 transition"
        />
        <button
          type="button"
          onClick={add}
          disabled={!input.trim()}
          className="flex items-center gap-1 px-3 py-2 rounded-lg border border-gray-200 dark:border-navy-600 bg-white dark:bg-navy-700 text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-navy-600 transition disabled:opacity-40"
        >
          <Plus className="w-4 h-4" />
          {addLabel}
        </button>
      </div>
    </div>
  );
}

export function NamesPoolForm({ poolKey, pool, isNew, isBusy, takenKeys, onBack, onSave }: NamesPoolFormProps) {
  const { t } = useTranslation();
  const [key, setKey] = useState(poolKey);
  const [firstNames, setFirstNames] = useState<string[]>(pool.first_names);
  const [lastNames, setLastNames] = useState<string[]>(pool.last_names);

  const trimmedKey = key.trim();
  // Renaming onto (or adding) a key that another pool already uses would drop
  // that pool when the map is rebuilt; block the save and explain why.
  const keyCollision = trimmedKey !== poolKey && takenKeys.includes(trimmedKey);

  function handleSave() {
    if (!trimmedKey || keyCollision) return;
    onSave(trimmedKey, { first_names: firstNames, last_names: lastNames });
  }

  return (
    <EntityFormShell
      title={isNew ? t("worldEditor.addPool") : t("worldEditor.editPool")}
      onBack={onBack}
      onSave={handleSave}
      isBusy={isBusy}
      saveDisabled={!trimmedKey || keyCollision}
      saveLabel={t("worldEditor.savePool")}
    >
      <div className="flex flex-col gap-1">
        <CountryCombobox
          label={t("worldEditor.poolKey")}
          value={key}
          onChange={setKey}
          placeholder="ENG"
        />
        {keyCollision && (
          <p className="text-xs text-red-500">{t("worldEditor.poolKeyTaken")}</p>
        )}
      </div>
      <NameChipList
        label={t("worldEditor.poolFirstNames")}
        names={firstNames}
        addPlaceholder={t("worldEditor.poolNamePlaceholder")}
        addLabel={t("worldEditor.poolAddName")}
        onChange={setFirstNames}
      />
      <NameChipList
        label={t("worldEditor.poolLastNames")}
        names={lastNames}
        addPlaceholder={t("worldEditor.poolNamePlaceholder")}
        addLabel={t("worldEditor.poolAddName")}
        onChange={setLastNames}
      />
    </EntityFormShell>
  );
}
