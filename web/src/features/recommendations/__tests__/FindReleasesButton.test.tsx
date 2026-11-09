import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { FindReleasesButton } from "../FindReleasesButton";

describe("FindReleasesButton", () => {
  it("carries a count badge, and says it in its name for screen readers and the remote", async () => {
    const press = vi.fn();
    await renderWithProviders(<FindReleasesButton count={3} onPress={press} />, { router: false });
    const button = screen.getByRole("button", { name: "Find releases, 3 recommended" });
    expect(screen.getByText("3")).toBeInTheDocument();
    await userEvent.click(button);
    expect(press).toHaveBeenCalledOnce();
  });

  it("has no badge when there is nothing to recommend", async () => {
    await renderWithProviders(<FindReleasesButton count={0} onPress={() => {}} />, { router: false });
    expect(screen.getByRole("button", { name: "Find releases" })).toBeInTheDocument();
    expect(screen.queryByText("0")).toBeNull();
  });
});
