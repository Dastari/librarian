import { render, screen, within } from "@testing-library/react";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { ScanIssueDetail } from "../ScanIssueDetail";
import { useScanIssueActions } from "../useScanIssueActions";
import type { IssueRow } from "../useScans";

vi.mock("@apollo/client/react", () => ({ useMutation: () => [vi.fn()] }));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: { mediaFiles: { edges: [] } }, loading: false, refetch: vi.fn() }) }));

const issue = (overrides: Partial<IssueRow>): IssueRow => ({
  id: "issue-1", libraryId: "lib-1", scanRunId: null, severity: "WARNING", stage: "LINKING", issueCode: "BYTE_IDENTICAL_DUPLICATE",
  message: "Two identical files", remediation: null, occurrenceCount: 2, resolvedAt: null, resolution: null,
  createdAt: "2026-11-01T10:00:00Z", updatedAt: "2026-11-02T10:00:00Z",
  detailsJson: JSON.stringify({ keeperPath: "/music/A/01.flac", duplicatePath: "/music/B/01.flac", size: 1000 }),
  mediaFile: { id: "mf-1", libraryId: "lib-1", path: "/music/B/01.flac", size: 1000, track: null, chapter: null, movie: null, episode: null },
  ...overrides,
}) as unknown as IssueRow;

function Harness({ row }: { row: IssueRow }) {
  const actions = useScanIssueActions(vi.fn(), true);
  return <ScanIssueDetail row={row} actions={actions} onChanged={vi.fn()} />;
}
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;

describe("scan issue dialog", () => {
  it("asks a duplicate as one decision: the two files side by side, keep either or both", () => {
    render(<Harness row={issue({})} />, { wrapper });
    expect(screen.getAllByText("01.flac")).toHaveLength(2);
    expect(screen.getAllByRole("button", { name: "Keep this one" })).toHaveLength(2);
    expect(screen.getByRole("button", { name: "Keep both" })).toBeInTheDocument();
    // The header offers no second, competing way to settle it (no trash button there).
    const toolbar = screen.queryByRole("toolbar");
    expect(toolbar ? within(toolbar).queryByRole("button", { name: /trash|better copy|resolved/i }) : null).toBeNull();
  });

  it("keeps resolve and retry for other issues, with no duplicate choices", () => {
    render(<Harness row={issue({ issueCode: "ANALYSIS_FAILED", stage: "ANALYSIS", detailsJson: null })} />, { wrapper });
    expect(screen.queryByRole("button", { name: "Keep this one" })).toBeNull();
    expect(screen.getAllByRole("button", { name: /Retry analysis|Mark resolved/ }).length).toBeGreaterThan(0);
  });
});
