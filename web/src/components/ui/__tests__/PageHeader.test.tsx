import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { PageHeader } from "../PageHeader";

describe("PageHeader route titles", () => {
  it.each(["Activity", "Downloads", "Wanted"])("keeps %s accessible without repeated visible framing", (title) => {
    render(<PageHeader title={title} titleVisibility="accessible" meta="12 loaded" actions={<button>Refresh</button>} />);
    const heading = screen.getByRole("heading", { level: 1, name: title });
    expect(heading).toHaveClass("sr-only", "route-print-title");
    expect(heading).not.toHaveAttribute("aria-hidden");
    expect(screen.getByText("12 loaded")).not.toHaveClass("sr-only", "mt-1.5");
    expect(screen.getByRole("button", { name: "Refresh" })).toBeVisible();
    expect(screen.getByRole("button").parentElement).toHaveAttribute("data-print", "hide");
  });

  it("preserves the focused action while counts update", async () => {
    const refresh = vi.fn();
    const header = (count: number) => <PageHeader title="Wanted" titleVisibility="accessible" meta={`${count} titles`} actions={<button onClick={refresh}>Refresh</button>} />;
    const { rerender } = render(header(1));
    const action = screen.getByRole("button", { name: "Refresh" });
    action.focus();
    rerender(header(12));
    expect(action).toHaveFocus();
    expect(screen.getByText("12 titles")).toBeVisible();
    await userEvent.keyboard("{Enter}");
    expect(refresh).toHaveBeenCalledOnce();
  });

  it("retains the visible contextual heading and its metadata spacing by default", () => {
    render(<PageHeader title="Family films" eyebrow="Movies" meta="80 movies" />);
    expect(screen.getByRole("heading", { name: "Family films" })).not.toHaveClass("sr-only");
    expect(screen.getByText("Movies")).toBeVisible();
    expect(screen.getByText("80 movies")).toHaveClass("mt-1.5");
  });
});
