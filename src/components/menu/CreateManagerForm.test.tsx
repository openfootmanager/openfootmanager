import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import CreateManagerForm from "./CreateManagerForm";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: "en" } }),
}));

describe("CreateManagerForm close control", () => {
  /**
   * Given the career wizard's manager step, whose close control is an icon alone
   * When it is looked up by what a screen reader announces
   * Then it is named (the translated "Close") and shows a focus ring.
   */
  it("names its close control and gives it a focus ring", () => {
    render(
      <CreateManagerForm
        formData={{
          firstName: "",
          lastName: "",
          dob: "",
          startYear: "2026",
          startPhase: "seasonStart",
          nationality: "",
        }}
        formErrors={{}}
        dobError={null}
        profiles={[]}
        onChange={vi.fn()}
        onClearError={vi.fn()}
        onClose={vi.fn()}
        onSelectProfile={vi.fn()}
        onDeleteProfile={vi.fn()}
        onSubmit={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "common.close" })).toHaveClass("focus-visible:ring-focus");
  });
});
