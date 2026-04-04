import { describe, expect, it } from "vitest";

import {
    getConfirmedSwapXiIds,
    getDroppedLineupXiIds,
    getEmptyTacticsLineupSelection,
    getNextTacticsLineupSelection,
} from "./TacticsTab.helpers";

describe("TacticsTab.helpers lineup selection", () => {
    it("starts a new selection from the clicked player", () => {
        expect(
            getNextTacticsLineupSelection(
                getEmptyTacticsLineupSelection(),
                "d1",
                "xi",
            ),
        ).toEqual({
            selectedPlayerId: "d1",
            selectedPlayerSection: "xi",
            comparePlayerId: null,
            comparePlayerSection: null,
        });
    });

    it("promotes the comparison player when the selected player is clicked again", () => {
        expect(
            getNextTacticsLineupSelection(
                {
                    selectedPlayerId: "d1",
                    selectedPlayerSection: "xi",
                    comparePlayerId: "m1",
                    comparePlayerSection: "xi",
                },
                "d1",
                "xi",
            ),
        ).toEqual({
            selectedPlayerId: "m1",
            selectedPlayerSection: "xi",
            comparePlayerId: null,
            comparePlayerSection: null,
        });
    });

    it("clears the comparison player when it is clicked again", () => {
        expect(
            getNextTacticsLineupSelection(
                {
                    selectedPlayerId: "d1",
                    selectedPlayerSection: "xi",
                    comparePlayerId: "m1",
                    comparePlayerSection: "bench",
                },
                "m1",
                "bench",
            ),
        ).toEqual({
            selectedPlayerId: "d1",
            selectedPlayerSection: "xi",
            comparePlayerId: null,
            comparePlayerSection: null,
        });
    });
});

describe("TacticsTab.helpers lineup mutations", () => {
    it("builds dropped xi ids for a bench player moved into a slot", () => {
        expect(
            getDroppedLineupXiIds({
                currentXiIds: ["gk1", "d1", "d2"],
                draggedPlayerId: "bench1",
                dragState: null,
                slotIndex: 1,
                xiIds: new Set(["gk1", "d1", "d2"]),
            }),
        ).toEqual(["gk1", "bench1", "d2"]);
    });

    it("builds dropped xi ids for an in-xi player reordered onto another slot", () => {
        expect(
            getDroppedLineupXiIds({
                currentXiIds: ["gk1", "d1", "d2"],
                draggedPlayerId: "d2",
                dragState: null,
                slotIndex: 1,
                xiIds: new Set(["gk1", "d1", "d2"]),
            }),
        ).toEqual(["gk1", "d2", "d1"]);
    });

    it("builds confirmed swap ids from the current selection state", () => {
        expect(
            getConfirmedSwapXiIds({
                currentXiIds: ["gk1", "d1", "d2"],
                selectionState: {
                    selectedPlayerId: "bench1",
                    selectedPlayerSection: "bench",
                    comparePlayerId: "d2",
                    comparePlayerSection: "xi",
                },
            }),
        ).toEqual(["gk1", "d1", "bench1"]);
    });
});