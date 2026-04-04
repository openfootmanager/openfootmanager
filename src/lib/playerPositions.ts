import type { PlayerData } from "../store/gameStore";

export const CORE_POSITIONS = [
    "Goalkeeper",
    "Defender",
    "Midfielder",
    "Forward",
] as const;

const POSITION_ALIASES: Record<string, string> = {
    gk: "Goalkeeper",
    goalkeeper: "Goalkeeper",
    defender: "Defender",
    def: "Defender",
    midfielder: "Midfielder",
    mid: "Midfielder",
    forward: "Forward",
    fwd: "Forward",
    wingback: "Defender",
    winger: "Forward",
    rb: "RightBack",
    rightback: "RightBack",
    cb: "CenterBack",
    centerback: "CenterBack",
    centreback: "CenterBack",
    lb: "LeftBack",
    leftback: "LeftBack",
    rwb: "RightWingBack",
    rightwingback: "RightWingBack",
    lwb: "LeftWingBack",
    leftwingback: "LeftWingBack",
    dm: "DefensiveMidfielder",
    defensivemidfielder: "DefensiveMidfielder",
    cm: "CentralMidfielder",
    centralmidfielder: "CentralMidfielder",
    am: "AttackingMidfielder",
    attackingmidfielder: "AttackingMidfielder",
    rm: "RightMidfielder",
    rightmidfielder: "RightMidfielder",
    lm: "LeftMidfielder",
    leftmidfielder: "LeftMidfielder",
    rw: "RightWinger",
    rightwinger: "RightWinger",
    lw: "LeftWinger",
    leftwinger: "LeftWinger",
    st: "Striker",
    striker: "Striker",
};

const POSITION_GROUPS: Record<string, string> = {
    Goalkeeper: "Goalkeeper",
    Defender: "Defender",
    Midfielder: "Midfielder",
    Forward: "Forward",
    RightBack: "Defender",
    CenterBack: "Defender",
    LeftBack: "Defender",
    RightWingBack: "Defender",
    LeftWingBack: "Defender",
    DefensiveMidfielder: "Midfielder",
    CentralMidfielder: "Midfielder",
    AttackingMidfielder: "Midfielder",
    RightMidfielder: "Midfielder",
    LeftMidfielder: "Midfielder",
    RightWinger: "Forward",
    LeftWinger: "Forward",
    Striker: "Forward",
};

const POSITION_LABELS: Record<string, string> = {
    Goalkeeper: "Goalkeeper",
    Defender: "Defender",
    Midfielder: "Midfielder",
    Forward: "Forward",
    RightBack: "Right Back",
    CenterBack: "Center Back",
    LeftBack: "Left Back",
    RightWingBack: "Right Wing-Back",
    LeftWingBack: "Left Wing-Back",
    DefensiveMidfielder: "Defensive Midfielder",
    CentralMidfielder: "Central Midfielder",
    AttackingMidfielder: "Attacking Midfielder",
    RightMidfielder: "Right Midfielder",
    LeftMidfielder: "Left Midfielder",
    RightWinger: "Right Winger",
    LeftWinger: "Left Winger",
    Striker: "Striker",
};

const POSITION_CODES: Record<string, string> = {
    Goalkeeper: "GK",
    Defender: "DEF",
    Midfielder: "MID",
    Forward: "FWD",
    RightBack: "RB",
    CenterBack: "CB",
    LeftBack: "LB",
    RightWingBack: "RWB",
    LeftWingBack: "LWB",
    DefensiveMidfielder: "DM",
    CentralMidfielder: "CM",
    AttackingMidfielder: "AM",
    RightMidfielder: "RM",
    LeftMidfielder: "LM",
    RightWinger: "RW",
    LeftWinger: "LW",
    Striker: "ST",
};

function normalisePositionKey(value: string): string {
    return value.toLowerCase().replace(/[^a-z]/g, "");
}

export function canonicalPosition(position: string): string {
    const trimmed = position.trim();
    if (!trimmed) {
        return trimmed;
    }

    return POSITION_ALIASES[normalisePositionKey(trimmed)] || trimmed;
}

export function exactPosition(position: string): string {
    switch (canonicalPosition(position)) {
        case "Defender":
            return "CenterBack";
        case "Midfielder":
            return "CentralMidfielder";
        case "Forward":
            return "Striker";
        default:
            return canonicalPosition(position);
    }
}

export function normalisePosition(position: string): string {
    const canonical = canonicalPosition(position);
    return POSITION_GROUPS[canonical] || canonical;
}

export function positionGroup(position: string): string {
    return normalisePosition(position);
}

export function positionCode(position: string): string {
    const normalized = canonicalPosition(position);
    return (
        POSITION_CODES[normalized] || normalized.substring(0, 3).toUpperCase()
    );
}

export function translatePositionLabel(
    translate: (key: string, options?: { defaultValue?: string }) => string,
    position: string,
): string {
    const canonical = canonicalPosition(position);

    return translate(`common.positions.${canonical}`, {
        defaultValue: POSITION_LABELS[canonical] || canonical,
    });
}

export function translatePositionAbbreviation(
    translate: (key: string, options?: { defaultValue?: string }) => string,
    position: string,
): string {
    const normalized = canonicalPosition(position);

    return translate(`common.posAbbr.${normalized}`, {
        defaultValue: positionCode(position),
    });
}

export function getPreferredPositions(player: PlayerData): string[] {
    return [
        ...new Set(
            [
                player.natural_position || player.position,
                ...(player.alternate_positions || []),
            ]
                .filter(Boolean)
                .map(canonicalPosition),
        ),
    ];
}