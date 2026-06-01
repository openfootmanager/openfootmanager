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

export function positionBadgeVariant(pos: string): "accent" | "primary" | "success" | "danger" {
    switch (pos) {
        case "Goalkeeper":
            return "accent";
        case "Defender":
        case "RightBack":
        case "CenterBack":
        case "LeftBack":
        case "RightWingBack":
        case "LeftWingBack":
            return "primary";
        case "Midfielder":
        case "DefensiveMidfielder":
        case "CentralMidfielder":
        case "AttackingMidfielder":
        case "RightMidfielder":
        case "LeftMidfielder":
            return "success";
        case "Forward":
        case "RightWinger":
        case "LeftWinger":
        case "Striker":
            return "danger";
        default:
            return "primary";
    }
}
