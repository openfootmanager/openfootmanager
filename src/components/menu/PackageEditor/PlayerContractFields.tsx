import { useId } from "react";
import { useTranslation } from "react-i18next";
import { DatePicker } from "../../ui/DatePicker";
import { WIRE_MAX, parseOptionalWhole } from "./helpers";
import { LabeledInput, labelClass } from "./primitives";
import type { PlayerDef } from "./types";

interface PlayerContractFieldsProps {
  editing: PlayerDef;
  updateField: <K extends keyof PlayerDef>(key: K, value: PlayerDef[K]) => void;
}

/**
 * A player's contract, wage and market value.
 *
 * Every field writes exactly what it is given and works nothing out. What a length
 * resolves to, which of an end date and a length wins, and the limits on each are
 * the backend's rules; the validator reports a breach in words, and showing an
 * end date here worked out from a start and a length would be a second, wrong,
 * copy of it (it depends on the year a career opens in, which the editor does not
 * know). The number fields are held only to what they can store.
 */
export function PlayerContractFields({ editing, updateField }: PlayerContractFieldsProps) {
  const { t } = useTranslation();
  const startLabelId = useId();
  const endLabelId = useId();

  return (
    <div className="flex flex-col gap-3">
      <p className={labelClass}>{t("worldEditor.playerContractSection")}</p>
      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-1">
          <label id={startLabelId} className={labelClass}>
            {t("worldEditor.playerContractStart")}
          </label>
          <DatePicker
            labelledBy={startLabelId}
            value={editing.contractStart ?? ""}
            onChange={(v) => updateField("contractStart", v || null)}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label id={endLabelId} className={labelClass}>
            {t("worldEditor.playerContractEnd")}
          </label>
          <DatePicker
            labelledBy={endLabelId}
            value={editing.contractEnd ?? ""}
            onChange={(v) => updateField("contractEnd", v || null)}
          />
        </div>
      </div>
      <LabeledInput
        label={t("worldEditor.playerContractLength")}
        help={t("worldEditor.playerContractLengthHelp")}
        value={editing.contractLength?.toString() ?? ""}
        type="number"
        onChange={(v) => updateField("contractLength", parseOptionalWhole(v, WIRE_MAX.u32))}
      />
      <div className="grid grid-cols-2 gap-3">
        <LabeledInput
          label={t("common.wage")}
          help={t("worldEditor.playerWageHelp")}
          value={editing.wage?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("wage", parseOptionalWhole(v, WIRE_MAX.u32))}
        />
        <LabeledInput
          label={t("common.value")}
          value={editing.value?.toString() ?? ""}
          type="number"
          onChange={(v) => updateField("value", parseOptionalWhole(v, WIRE_MAX.safe))}
        />
      </div>
    </div>
  );
}
