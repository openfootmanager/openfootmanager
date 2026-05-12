import type {
    ScoutingAssignment,
    StaffData,
    YouthScoutingAssignment,
} from "../../store/gameStore";

export type PlayerProfileScoutStatus =
    | "idle"
    | "sending"
    | "sent"
    | "error";

interface ScoutAvailabilityArgs {
    staff: StaffData[];
    scoutingAssignments: ScoutingAssignment[];
    youthScoutingAssignments: YouthScoutingAssignment[];
    managerTeamId: string | null;
    playerId: string;
    scoutStatus: PlayerProfileScoutStatus;
}

export interface ScoutAvailability {
    scouts: StaffData[];
    availableScout: StaffData | null;
    alreadyScouting: boolean;
    allBusy: boolean;
    canScout: boolean;
}

export function getScoutAvailability({
    staff,
    scoutingAssignments,
    youthScoutingAssignments,
    managerTeamId,
    playerId,
    scoutStatus,
}: ScoutAvailabilityArgs): ScoutAvailability {
    const scouts = staff.filter(
        (member) =>
            member.role === "Scout" && member.team_id === managerTeamId,
    );
    const alreadyScouting = scoutingAssignments.some(
        (assignment) => assignment.player_id === playerId,
    );
    const allAssignments = [...scoutingAssignments, ...youthScoutingAssignments];
    const availableScout =
        scouts.find(
            (scout) =>
                !allAssignments.some(
                    (assignment) => assignment.scout_id === scout.id,
                ),
        ) ?? null;
    const allBusy = scouts.length > 0 && availableScout === null;
    const canScout =
        scouts.length > 0 &&
        !alreadyScouting &&
        !allBusy &&
        scoutStatus !== "sent" &&
        availableScout !== null;

    return {
        scouts,
        availableScout,
        alreadyScouting,
        allBusy,
        canScout,
    };
}