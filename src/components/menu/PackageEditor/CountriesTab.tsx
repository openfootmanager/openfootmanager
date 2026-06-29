import { useTranslation } from "react-i18next";
import { EntityListShell, EntityRow } from "./shared";
import type { CountryDef } from "./types";

interface CountriesTabProps {
  countries: CountryDef[];
  onAdd: () => void;
  onEdit: (index: number) => void;
  onDelete: (index: number) => void;
  selectedIndex?: number | null;
  onSelect?: (index: number) => void;
}

export function CountriesTab({ countries, onAdd, onEdit, onDelete, selectedIndex, onSelect }: CountriesTabProps) {
  const { t } = useTranslation();
  return (
    <EntityListShell
      addLabel={t("worldEditor.addCountry")}
      onAdd={onAdd}
      emptyLabel={t("worldEditor.noCountries")}
      isEmpty={countries.length === 0}
    >
      {countries.map((country, i) => (
        <EntityRow
          key={i}
          title={country.name || country.id}
          subtitle={[country.name ? country.id : undefined, country.confederation]
            .filter(Boolean)
            .join(" · ")}
          onEdit={() => onEdit(i)}
          onDelete={() => onDelete(i)}
          editLabel={t("worldEditor.editCountry")}
          deleteLabel={t("worldEditor.deleteCountry")}
          isSelected={selectedIndex === i}
          onClick={onSelect ? () => onSelect(i) : undefined}
        />
      ))}
    </EntityListShell>
  );
}
