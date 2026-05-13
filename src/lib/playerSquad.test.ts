import { describe, expect, it } from "vitest";

import {
    canDelegateToYouthAcademy,
    getPlayerSquadRole,
    isSeniorSquadPlayer,
    isYouthAcademyPlayer,
} from "./playerSquad";

describe("playerSquad", () => {
    it("defaults missing squad role to senior", () => {
        expect(getPlayerSquadRole({ squad_role: undefined })).toBe("Senior");
        expect(isSeniorSquadPlayer({ squad_role: undefined })).toBe(true);
    });

    it("identifies youth academy players explicitly", () => {
        expect(isYouthAcademyPlayer({ squad_role: "Youth" })).toBe(true);
        expect(isSeniorSquadPlayer({ squad_role: "Youth" })).toBe(false);
    });

    it("only allows under-21 senior players to be delegated", () => {
        expect(
            canDelegateToYouthAcademy({
                date_of_birth: "2008-01-01",
                squad_role: "Senior",
            }),
        ).toBe(true);
        expect(
            canDelegateToYouthAcademy({
                date_of_birth: "1998-01-01",
                squad_role: "Senior",
            }),
        ).toBe(false);
        expect(
            canDelegateToYouthAcademy({
                date_of_birth: "2008-01-01",
                squad_role: "Youth",
            }),
        ).toBe(false);
    });
});