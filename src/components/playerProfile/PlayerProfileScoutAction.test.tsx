import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import PlayerProfileScoutAction from "./PlayerProfileScoutAction";
import type { StaffData } from "../../store/gameStore";
import type { ScoutAvailability } from "./PlayerProfile.scouting";

vi.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string) => {
            if (key === "scouting.noScoutsHint") return "Hire a scout first";
            if (key === "scouting.scoutingInProgress") return "Scouting in progress";
            if (key === "scouting.scoutBtn") return "Scout";
            return key;
        },
    }),
}));

describe("PlayerProfileScoutAction", () => {
    const scout: StaffData = {
        id: "scout-1",
        first_name: "Sam",
        last_name: "Scout",
        date_of_birth: "1990-01-01",
        nationality: "ES",
        role: "Scout",
        attributes: {
            coaching: 0,
            judging_ability: 14,
            judging_potential: 15,
            physiotherapy: 0,
        },
        team_id: "team-1",
        specialization: null,
        wage: 1500,
        contract_end: "2027-06-30",
    };

    function buildAvailability(overrides: Partial<ScoutAvailability>): ScoutAvailability {
        return {
            scouts: [scout],
            availableScout: scout,
            alreadyScouting: false,
            allBusy: false,
            canScout: true,
            ...overrides,
        };
    }

    it("renders translated scout actions", () => {
        const onScout = vi.fn();

        render(
            <PlayerProfileScoutAction
                availability={buildAvailability({})}
                scoutStatus="idle"
                scoutError={null}
                onScout={onScout}
            />,
        );

        fireEvent.click(screen.getByRole("button", { name: /Scout/i }));

        expect(onScout).toHaveBeenCalledTimes(1);
        expect(screen.getByRole("button", { name: /Scout/i })).toBeInTheDocument();
    });

    it("renders translated hints and progress states", () => {
        const { rerender } = render(
            <PlayerProfileScoutAction
                availability={buildAvailability({
                    scouts: [],
                    availableScout: null,
                    canScout: false,
                })}
                scoutStatus="idle"
                scoutError={null}
                onScout={vi.fn()}
            />,
        );

        expect(screen.getByText("Hire a scout first")).toBeInTheDocument();

        rerender(
            <PlayerProfileScoutAction
                availability={buildAvailability({
                    alreadyScouting: true,
                    canScout: false,
                })}
                scoutStatus="sent"
                scoutError={null}
                onScout={vi.fn()}
            />,
        );

        expect(screen.getByText("Scouting in progress")).toBeInTheDocument();
    });
});