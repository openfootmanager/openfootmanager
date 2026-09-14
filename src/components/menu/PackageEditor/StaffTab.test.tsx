import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StaffTab } from "./StaffTab";
import { emptyStaff, emptyTeam } from "./helpers";
import type { StaffDef, TeamDef } from "./types";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: string | Record<string, string | number>) => {
      if (opts && typeof opts === "object") {
        const parts = Object.entries(opts)
          .filter(([name]) => name !== "defaultValue")
          .map(([name, value]) => `${name}=${value}`);
        return parts.length > 0 ? `${key} ${parts.join(" ")}` : key;
      }
      return typeof opts === "string" ? opts : key;
    },
    i18n: { language: "en" },
  }),
}));

function staffList(count: number, overrides: (i: number) => Partial<StaffDef> = () => ({})): StaffDef[] {
  return Array.from({ length: count }, (_, i) => ({
    ...emptyStaff(),
    id: `s${i}`,
    firstName: "Staff",
    lastName: `${i}`,
    ...overrides(i),
  }));
}

function renderTab(props: Partial<React.ComponentProps<typeof StaffTab>> = {}) {
  const onEdit = vi.fn();
  render(
    <StaffTab
      staff={staffList(500)}
      onAdd={() => {}}
      onEdit={onEdit}
      onDelete={() => {}}
      {...props}
    />,
  );
  return { onEdit };
}

function editButtons() {
  return screen.queryAllByRole("button", { name: "worldEditor.editStaff" });
}

describe("StaffTab", () => {
  it("renders one page of rows rather than every staff member", () => {
    renderTab();

    expect(editButtons()).toHaveLength(50);
    expect(screen.getByText("worldEditor.showingEntries shown=50 total=500")).toBeInTheDocument();
  });

  it("reveals another page on request", () => {
    renderTab();

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));

    expect(editButtons()).toHaveLength(100);
  });

  it("still addresses a revealed row by its index in the whole list", () => {
    const { onEdit } = renderTab();

    fireEvent.click(screen.getByRole("button", { name: "common.loadMore" }));
    fireEvent.click(editButtons()[50]);

    expect(onEdit).toHaveBeenCalledWith(50);
  });

  it("says so when a search matches nothing, instead of going blank", () => {
    renderTab();

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchStaff" }), {
      target: { value: "nobody" },
    });

    expect(editButtons()).toHaveLength(0);
    expect(screen.getByText("common.noResults")).toBeInTheDocument();
  });

  it("names a staff member's club rather than printing its id", () => {
    const teams: TeamDef[] = [{ ...emptyTeam(), id: "nsfc", name: "Northshire FC" }];
    renderTab({ staff: staffList(1, () => ({ club: "nsfc" })), teams });

    expect(screen.getByText(/Northshire FC/)).toBeInTheDocument();
  });

  it("finds a staff member by the club name shown in the row", () => {
    const teams: TeamDef[] = [{ ...emptyTeam(), id: "nsfc", name: "Northshire FC" }];
    const { onEdit } = renderTab({
      staff: staffList(3, (i) => (i === 2 ? { club: "nsfc" } : {})),
      teams,
    });

    fireEvent.change(screen.getByRole("textbox", { name: "worldEditor.searchStaff" }), {
      target: { value: "northshire" },
    });
    fireEvent.click(editButtons()[0]);

    expect(onEdit).toHaveBeenCalledWith(2);
  });
});
