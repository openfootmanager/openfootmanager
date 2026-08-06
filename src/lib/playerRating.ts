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
