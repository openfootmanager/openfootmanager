import {
  ATTRIBUTE_META,
  type AttributeGroupKey,
  GROUP_LABEL_KEY,
  type PlayerAttributeKey,
} from "../PlayerProfile.attributes";
import type { PlayerData } from "../../../store/gameStore";

/**
 * How many of three dots an attribute's weight at a position earns. The game's tables weigh 4 to 28
 * percent of a position's rating on an attribute: 16 and more is three dots, 8 and more two,
 * anything else the position weighs one, and an attribute it does not weigh none. The dots only
 * draw the table's own numbers in three steps; they decide nothing.
 */
export function weightDots(weight: number | undefined): 0 | 1 | 2 | 3 {
  if (weight === undefined || weight <= 0) return 0;
  if (weight >= 16) return 3;
  if (weight >= 8) return 2;
  return 1;
}

export interface AttributeRow {
  key: PlayerAttributeKey;
  name: string;
  value: number;
}

export interface AttributeColumn {
  label: string;
  attrs: AttributeRow[];
  average: number;
}

/** The columns' order: Technical, Mental, Physical, and Goalkeeping in place of Technical for a keeper. */
const OUTFIELD_COLUMNS: readonly AttributeGroupKey[] = ["technical", "mental", "physical"];
const GOALKEEPER_COLUMNS: readonly AttributeGroupKey[] = ["goalkeeper", "mental", "physical"];

/** His attributes in the profile's columns, each attribute once, with each column's average. */
export function attributeColumns(
  attributes: PlayerData["attributes"],
  isGoalkeeper: boolean,
  translate: (key: string) => string,
): AttributeColumn[] {
  const order = isGoalkeeper ? GOALKEEPER_COLUMNS : OUTFIELD_COLUMNS;
  return order.map((group) => {
    const attrs = (Object.entries(ATTRIBUTE_META) as [PlayerAttributeKey, AttributeGroupKey][])
      .filter(([, attributeGroup]) => attributeGroup === group)
      .map(([key]) => ({
        key,
        name: translate(`common.attributes.${key}`),
        value: attributes[key],
      }));
    const mean = attrs.reduce((sum, attr) => sum + attr.value, 0) / Math.max(1, attrs.length);
    return { label: translate(GROUP_LABEL_KEY[group]), attrs, average: Math.round(mean) };
  });
}
