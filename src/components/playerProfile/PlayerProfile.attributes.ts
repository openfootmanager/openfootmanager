import type { PlayerData } from "../../store/gameStore";

type TranslateFn = (key: string) => string;

export interface PlayerAttributeEntry {
    name: string;
    value: number;
}

export interface PlayerAttributeGroup {
    label: string;
    attrs: PlayerAttributeEntry[];
    average: number;
}

export type PlayerAttributeKey = keyof PlayerData["attributes"];
type AttributeGroupKey = "physical" | "technical" | "mental" | "goalkeeper";

// Single source of truth for the profile display. Every attribute in
// PlayerData["attributes"] must appear here — `satisfies Record<...>` makes
// adding an attribute to the domain a compile-time error until it's grouped.
// Order within each group follows insertion order below (ES2015+ guarantees
// `Object.entries` returns keys in insertion order for string keys).
const ATTRIBUTE_META = {
    // Physical
    pace: "physical",
    stamina: "physical",
    strength: "physical",
    agility: "physical",
    // Aerial belongs with Physical — engine uses it for header duels for every
    // player, not just GK claims (see engine/resolution.rs & zone_resolution.rs).
    aerial: "physical",

    // Technical
    passing: "technical",
    shooting: "technical",
    tackling: "technical",
    dribbling: "technical",
    defending: "technical",

    // Mental
    positioning: "mental",
    vision: "mental",
    decisions: "mental",
    composure: "mental",
    aggression: "mental",
    teamwork: "mental",
    leadership: "mental",

    // Goalkeeper — only used in save/penalty/zone resolution for GKs. Hidden
    // for outfielders to keep the profile focused on stats that apply.
    handling: "goalkeeper",
    reflexes: "goalkeeper",
} as const satisfies Record<PlayerAttributeKey, AttributeGroupKey>;

const GROUP_LABEL_KEY: Record<AttributeGroupKey, string> = {
    physical: "common.attrGroups.physical",
    technical: "common.attrGroups.technical",
    mental: "common.attrGroups.mental",
    goalkeeper: "common.attrGroups.goalkeeper",
};

// Fixed display order for groups. Groups not applicable to the current player
// (e.g. `goalkeeper` for outfielders) are filtered downstream.
const GROUP_ORDER: readonly AttributeGroupKey[] = [
    "physical",
    "technical",
    "mental",
    "goalkeeper",
];

function createAttributeGroup(
    label: string,
    attrs: PlayerAttributeEntry[],
): PlayerAttributeGroup {
    return {
        label,
        attrs,
        average: Math.round(
            attrs.reduce((sum, attribute) => sum + attribute.value, 0) / attrs.length,
        ),
    };
}

function isGroupApplicable(
    group: AttributeGroupKey,
    player: PlayerData,
): boolean {
    if (group === "goalkeeper") {
        return player.position === "Goalkeeper";
    }
    return true;
}

export function buildPlayerAttributeGroups(
    player: PlayerData,
    translate: TranslateFn,
): PlayerAttributeGroup[] {
    const buckets = new Map<AttributeGroupKey, PlayerAttributeEntry[]>();
    for (const [attributeKey, groupKey] of Object.entries(ATTRIBUTE_META) as [
        PlayerAttributeKey,
        AttributeGroupKey,
    ][]) {
        const bucket = buckets.get(groupKey) ?? [];
        bucket.push({
            name: translate(`common.attributes.${attributeKey}`),
            value: player.attributes[attributeKey],
        });
        buckets.set(groupKey, bucket);
    }

    return GROUP_ORDER
        .filter((group) => isGroupApplicable(group, player))
        .map((groupKey) => {
            const attrs = buckets.get(groupKey) ?? [];
            return createAttributeGroup(translate(GROUP_LABEL_KEY[groupKey]), attrs);
        })
        .filter((group) => group.attrs.length > 0);
}

/**
 * Look up a single attribute entry (translated name + value) by its key,
 * without relying on positional indexing into `PlayerAttributeGroup.attrs`.
 * Callers that need a specific attribute (e.g. the radar chart picking a
 * subset for visualisation) should use this instead of `group.attrs[i]` so
 * reordering `ATTRIBUTE_META` above doesn't silently swap what they render.
 */
export function getPlayerAttributeEntry(
    player: PlayerData,
    key: PlayerAttributeKey,
    translate: TranslateFn,
): PlayerAttributeEntry {
    return {
        name: translate(`common.attributes.${key}`),
        value: player.attributes[key],
    };
}
