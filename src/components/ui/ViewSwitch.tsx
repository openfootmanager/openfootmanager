import { LayoutGrid, Rows3 } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";

import type { ListView } from "../../store/settingsStore";
import { SegmentedControl } from "./SegmentedControl";

interface ViewSwitchProps {
  value: ListView;
  onChange: (view: ListView) => void;
}

/**
 * The Table/Cards choice at the end of a list's toolbar. The page binds it to the one global
 * setting, `list_view`, so whatever the player picks on one list is how every list opens.
 */
export function ViewSwitch({ value, onChange }: ViewSwitchProps) {
  const { t } = useTranslation();
  const labelId = useId();
  return (
    <div className="flex items-center gap-2">
      <span id={labelId} className="sr-only">
        {t("common.listViewLabel")}
      </span>
      <SegmentedControl
        labelledBy={labelId}
        value={value}
        onChange={onChange}
        options={[
          {
            value: "table",
            label: t("common.listViewTable"),
            icon: <Rows3 aria-hidden="true" className="h-4 w-4" />,
          },
          {
            value: "cards",
            label: t("common.listViewCards"),
            icon: <LayoutGrid aria-hidden="true" className="h-4 w-4" />,
          },
        ]}
      />
    </div>
  );
}
