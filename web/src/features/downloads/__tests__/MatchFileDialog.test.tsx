import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { MatchFileDialog } from "../MatchFileDialog";

const mocks = vi.hoisted(() => ({ find: vi.fn(), match: vi.fn(), success: vi.fn(), danger: vi.fn(), candidates: [] as unknown[] }));
vi.mock("@apollo/client/react", () => ({
  useLazyQuery: () => [mocks.find, { data: { torrentFileCandidates: mocks.candidates }, loading: false, error: undefined }],
  useMutation: () => [mocks.match],
}));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({
  useCompleteConnection: () => ({ rows: [{ id: "tv", name: "TV Shows" }], loading: false }),
}));
vi.mock("glass-ux/toast", async (original) => ({ ...(await original<typeof import("glass-ux/toast")>()), toast: { success: mocks.success, danger: mocks.danger } }));

Element.prototype.getAnimations ??= () => [];
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;
const file = { infoHash: "hash", fileIndex: 3, name: "S01E03.mkv", libraryId: "tv" };

describe("matching one file of a download by hand", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.candidates = [{ targetType: "Episode", targetId: "ep-3", targetName: "Past Prologue", score: 0.8, reason: "S01E03" }];
  });

  it("asks about this file in this library, and imports the choice", async () => {
    mocks.match.mockResolvedValue({ data: { matchTorrentFile: { success: true, matchCount: 1, error: null } } });
    const onClose = vi.fn(); const onMatched = vi.fn();
    render(<MatchFileDialog file={file} onClose={onClose} onMatched={onMatched} />, { wrapper });
    await waitFor(() => expect(mocks.find).toHaveBeenCalledWith({ variables: { infoHash: "hash", fileIndex: 3, libraryId: "tv" } }));
    await userEvent.click(await screen.findByText("Past Prologue"));
    await waitFor(() => expect(mocks.match).toHaveBeenCalledWith({ variables: { input: { infoHash: "hash", fileIndex: 3, libraryId: "tv", targetType: "Episode", targetId: "ep-3" } } }));
    await waitFor(() => expect(onMatched).toHaveBeenCalledOnce());
    expect(onClose).toHaveBeenCalledOnce();
    expect(mocks.success).toHaveBeenCalledWith("Imported as Past Prologue");
  });

  it("stays open and says why when the import is refused", async () => {
    mocks.match.mockResolvedValue({ data: { matchTorrentFile: { success: false, matchCount: 0, error: "That title is not in the chosen library" } } });
    const onClose = vi.fn();
    render(<MatchFileDialog file={file} onClose={onClose} />, { wrapper });
    await userEvent.click(await screen.findByText("Past Prologue"));
    await waitFor(() => expect(mocks.danger).toHaveBeenCalledWith("That title is not in the chosen library"));
    expect(onClose).not.toHaveBeenCalled();
  });

  it("points at adding the title when there is nothing to match", async () => {
    mocks.candidates = [];
    render(<MatchFileDialog file={file} onClose={vi.fn()} />, { wrapper });
    expect(await screen.findByText("Nothing to match it to")).toBeInTheDocument();
  });
});
