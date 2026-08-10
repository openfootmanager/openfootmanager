import { useId, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Globe, PencilLine } from "lucide-react";
import { LabeledInput } from "./primitives";
import { EntityFormShell } from "./shared";
import { Select } from "../../ui/Select";
import { CountryCombobox } from "../../ui/CountryCombobox";
import { useNations } from "../../../hooks/useNations";
import { buildRegionLabel } from "../../../lib/teamRegions";
import type { ConfederationDef, CountryDef } from "./types";

interface CountryFormProps {
  editing: CountryDef;
  editingIndex: number | null;
  confederations: ConfederationDef[];
  isBusy: boolean;
  onBack: () => void;
  onSave: () => void;
  updateField: <K extends keyof CountryDef>(key: K, value: CountryDef[K]) => void;
}

/**
 * How the country's identity is being authored.
 *
 * `builtin` picks from the game's nation catalog, which fixes the id to a code
 * the rest of the game already understands — flags, translated names, and
 * nationality matching all key off it. `custom` is the escape hatch for a
 * nation the catalog does not carry (a historical state, an invented one), and
 * is the only way to author a free-text id.
 */
type IdentityMode = "builtin" | "custom";

const labelClass =
  "text-[10px] font-heading font-bold uppercase tracking-[0.18em] text-gray-500 dark:text-gray-400";

export function CountryForm({
  editing,
  editingIndex,
  confederations,
  isBusy,
  onBack,
  onSave,
  updateField,
}: CountryFormProps) {
  const { t } = useTranslation();
  const nations = useNations();
  // Set only when the author picks a mode. Until then the mode is inferred from
  // the record, so opening an existing country never depends on state that
  // could have been left behind by the previously edited one. The form is
  // remounted on every buffer swap (keyed on `revision` in WorldEditorFormPanel),
  // so this starts empty for each record.
  const [chosenMode, setChosenMode] = useState<IdentityMode | null>(null);

  // The regions the catalog itself uses, which are exactly the ids
  // `is_builtin_region` accepts. Derived from the nation catalog rather than
  // listed here, so the editor cannot come to disagree with the backend about
  // which confederations exist.
  const builtInRegions = useMemo(() => {
    const seen = new Set((nations ?? []).map((nation) => nation.region));
    return [...seen].sort();
  }, [nations]);

  // `null` until the catalog resolves, for a new country as much as an existing
  // one. `CountryCombobox` loads its own copy of the name data, so offering the
  // picker early lets a nation be adopted before there is a catalog to read its
  // canonical name out of — and `selectNation` would then store the code as the
  // name, which is the one thing the canonical-name rule exists to prevent.
  //
  // A catalog that resolves empty (it could not be read) means nothing can be
  // called built-in, and the picker would fall back to offering every ISO
  // country — a wider list than this format accepts. Author by hand instead.
  //
  // Guessing in either direction is what would rewrite authored data: show a
  // custom country in the picker and it reads as unset.
  const inferredMode: IdentityMode | null =
    nations === null
      ? null
      : nations.length === 0
        ? "custom"
        : !editing.id || nations.some((nation) => nation.code === editing.id)
          ? "builtin"
          : "custom";
  const mode = chosenMode ?? inferredMode;

  /**
   * Adopt a catalog nation: its code becomes the id, its canonical name the
   * name.
   *
   * The name deliberately comes from the catalog rather than from the label the
   * author just clicked. That label is translated into the editor's language,
   * so filling from it would write "Brasil" into a package authored in pt-BR
   * and "Brazil" into the same package authored in English — two packages that
   * no longer agree, and a name that is wrong in the other nine locales. The
   * stored name is a stable fallback; what a player reads is translated from
   * the id at render time.
   */
  function selectNation(code: string) {
    const nation = nations?.find((entry) => entry.code === code);
    // A code the catalog does not carry has no canonical name, and falling back
    // to the code would store "BR" as the country's *name* — the one outcome
    // this function exists to prevent. The picker is built from the same
    // catalog, so this should not arise; refusing keeps that true here instead
    // of trusting that it was.
    if (!nation) {
      return;
    }
    updateField("id", nation.code);
    updateField("name", nation.name);
  }

  const switchClass =
    "inline-flex items-center gap-1.5 self-start rounded-lg px-2 py-1 text-[11px] font-medium text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-500/10 focus:outline-none focus:ring-2 focus:ring-primary-400 focus:ring-offset-2 dark:focus:ring-offset-navy-800 transition";

  return (
    <EntityFormShell
      title={editingIndex === null ? t("worldEditor.addCountry") : t("worldEditor.editCountry")}
      onBack={onBack}
      onSave={onSave}
      isBusy={isBusy}
      saveDisabled={!editing.id || !editing.name}
      saveLabel={t("worldEditor.saveCountry")}
    >
      {mode === null ? (
        <div className="flex min-h-[62px] items-center justify-center" aria-live="polite">
          <span className="sr-only">{t("worldEditor.countryLoadingNations")}</span>
          <div className="h-5 w-5 motion-safe:animate-spin rounded-full border-2 border-primary-500 border-t-transparent" />
        </div>
      ) : mode === "builtin" ? (
        <>
          <CountryCombobox
            label={t("worldEditor.countryNation")}
            value={editing.id}
            onChange={selectNation}
            placeholder={t("worldEditor.countryNationPlaceholder")}
          />
          <button type="button" className={switchClass} onClick={() => setChosenMode("custom")}>
            <PencilLine className="h-3.5 w-3.5" aria-hidden="true" />
            {t("worldEditor.countryCustom")}
          </button>
        </>
      ) : (
        <>
          <LabeledInput
            label={t("worldEditor.countryId")}
            value={editing.id}
            onChange={(v) => updateField("id", v)}
            placeholder="YUG"
            help={t("worldEditor.countryIdHelp")}
          />
          <LabeledInput
            label={t("worldEditor.countryName")}
            value={editing.name}
            onChange={(v) => updateField("name", v)}
            placeholder="Yugoslavia"
          />
          <button type="button" className={switchClass} onClick={() => setChosenMode("builtin")}>
            <Globe className="h-3.5 w-3.5" aria-hidden="true" />
            {t("worldEditor.countryFromCatalog")}
          </button>
        </>
      )}
      {/*
        Withheld until the catalog resolves, for the same reason the identity
        field is: the built-in list is empty while it loads, so a perfectly good
        `europe` would be filed under "not recognised" for as long as the
        backend takes to answer, then jump to the built-in group.
      */}
      {mode !== null && (
        <ConfederationField
          value={editing.confederation}
          builtInRegions={builtInRegions}
          confederations={confederations}
          onChange={(id) => updateField("confederation", id)}
        />
      )}
    </EntityFormShell>
  );
}

interface ConfederationFieldProps {
  value: string;
  /** Region ids the game already knows, in the order they should be offered. */
  builtInRegions: string[];
  /** Confederations this package defines itself. */
  confederations: ConfederationDef[];
  onChange: (id: string) => void;
}

/**
 * Picks the confederation a country belongs to, grouped by who owns the value.
 *
 * Which of these the author controls is the distinction the field exists to
 * make: a built-in region resolves without the package defining anything, and
 * an author who cannot see that has no way to know whether they need to define
 * a confederation at all — the question that sent one package to `uefa`, an id
 * nothing resolves.
 */
function ConfederationField({
  value,
  builtInRegions,
  confederations,
  onChange,
}: ConfederationFieldProps) {
  const { t } = useTranslation();
  const labelId = useId();

  // A value that is neither built in nor defined by this package. It still has
  // to be selectable, or opening the country would show an empty field and
  // saving would write that emptiness back over what the author wrote.
  const unrecognised =
    value && !builtInRegions.includes(value) && !confederations.some((c) => c.id === value)
      ? value
      : null;

  return (
    <div className="flex flex-col gap-1">
      <label className={labelClass} id={labelId}>
        {t("worldEditor.countryConfederation")}
      </label>
      <Select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        aria-labelledby={labelId}
        fullWidth
      >
        <option value="">—</option>
        {/* Built-ins first, because most countries want one. */}
        {builtInRegions.length > 0 && (
          <optgroup label={t("worldEditor.confederationBuiltIn")}>
            {builtInRegions.map((region) => (
              <option key={region} value={region}>
                {buildRegionLabel(t, region)}
              </option>
            ))}
          </optgroup>
        )}
        {confederations.length > 0 && (
          <optgroup label={t("worldEditor.confederationInPackage")}>
            {confederations.map((c) => (
              <option key={c.id} value={c.id}>
                {c.name || c.id}
              </option>
            ))}
          </optgroup>
        )}
        {unrecognised && (
          <optgroup label={t("worldEditor.confederationUnrecognised")}>
            <option value={unrecognised}>{unrecognised}</option>
          </optgroup>
        )}
      </Select>
    </div>
  );
}
