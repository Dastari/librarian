import { render, screen } from "@testing-library/react";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { GuidePoster, scheduledShowActions } from "../GuidePoster";

vi.mock("@tanstack/react-router", async (original) => ({
  ...(await original<typeof import("@tanstack/react-router")>()),
  Link: ({ children, "aria-label": label }: { children: ReactNode; "aria-label"?: string }) => <a href="/show" aria-label={label}>{children}</a>,
  useNavigate: () => vi.fn(),
}));
vi.mock("@apollo/client/react", () => ({ useMutation: () => [vi.fn()] }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true }));

const listing = { tvmazeShowId: 1, name: "Silo", network: "Apple TV" };
const wrap = (children: ReactNode) => render(<TooltipProvider><ActionProvider navigate={vi.fn()}>{children}</ActionProvider></TooltipProvider>);
const labels = (set: ReturnType<typeof scheduledShowActions>) => set.actions.map((action) => action.label);

describe("a show's portrait in the guide", () => {
  it("draws the same thing at every size, with its badge", () => {
    const actions = scheduledShowActions("Apple TV", vi.fn());
    wrap(<>
      <GuidePoster size="tile" name="Silo" image={null} badge="New" actions={actions} target={listing} onPress={vi.fn()} />
      <GuidePoster size="panel" name="Severance" image={null} to="/shows/$showId" actions={actions} target={listing} />
    </>);
    expect(screen.getByRole("button", { name: "Silo" }).className).toContain("guide-poster");
    expect(screen.getByRole("link", { name: "Severance" })).toBeTruthy();
    expect(screen.getByText("New").className).toContain("guide-poster-badge");
  });

  it("offers a listing's details, and its service's filters only where they apply", () => {
    const service = { only: vi.fn(), hide: vi.fn() };
    expect(labels(scheduledShowActions("Apple TV", vi.fn(), service))).toEqual(["Show details", "Only show Apple TV", "Hide Apple TV"]);
    expect(labels(scheduledShowActions("Apple TV", vi.fn()))).toEqual(["Show details"]);
    expect(labels(scheduledShowActions(null, vi.fn(), service))).toEqual(["Show details"]);
  });
});
