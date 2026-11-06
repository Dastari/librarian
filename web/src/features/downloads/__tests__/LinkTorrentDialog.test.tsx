import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

import { EntityTorrentUpdateDocument, RematchSourceDocument } from "@/graphql/generated/graphql";
import type { DownloadRow } from "../useDownloads";
import { LinkTorrentDialog } from "../LinkTorrentDialog";

const calls = vi.hoisted(() => ({ update: vi.fn(), rematch: vi.fn(), success: vi.fn(), danger: vi.fn(), warning: vi.fn() }));
vi.mock("@apollo/client/react", () => ({
  useMutation: (document: unknown) => {
    if (document === EntityTorrentUpdateDocument) return [calls.update, { loading: false }];
    if (document === RematchSourceDocument) return [calls.rematch, { loading: false }];
    throw new Error("Unexpected mutation document");
  },
}));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({
  useCompleteConnection: () => ({ rows: [{ id: "library-a", name: "First library", path: "/media/a", libraryType: "TV" }, { id: "library-b", name: "Second library", path: "/media/b", libraryType: "MOVIES" }], loading: false, error: undefined }),
}));
vi.mock("glass-ux/toast", async (original) => ({ ...(await original<typeof import("glass-ux/toast")>()), toast: { success: calls.success, danger: calls.danger, warning: calls.warning } }));
vi.mock("@/components/ui", () => ({ ErrorState: () => null, SkeletonBlock: () => null }));
// jsdom has no Web Animations; Base UI's dialog waits on them when it closes.
Element.prototype.getAnimations ??= () => [];
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;

// Only the dialog's consumed fields are relevant to this mutation-contract test.
const row = (libraryId: string | null = "library-a", progress = 1) => ({
  record: { id: "torrent-id", infoHash: "hash", name: "Download", state: "seeding", totalBytes: 1024, library: null, libraryId }, live: { id: 1, infoHash: "hash", name: "Download", state: "seeding", progress },
}) as unknown as DownloadRow;
const saved = (libraryId: string | null, id = "torrent-id") => ({ data: { updateTorrent: { success: true, error: null, torrent: { id, libraryId } } } });
const mount = (download = row()) => {
  const onClose = vi.fn(); const onLinked = vi.fn();
  const view = render(<LinkTorrentDialog row={download} onClose={onClose} onLinked={onLinked} />, { wrapper });
  return { ...view, onClose, onLinked };
};
beforeEach(() => {
  vi.clearAllMocks();
  calls.rematch.mockResolvedValue({ data: { rematchSource: { success: true, error: null, matchCount: 3 } } });
});

describe("torrent library persistence confirmation", () => {
  it("rematches and reports success only after the requested link was returned", async () => {
    calls.update.mockResolvedValue(saved("library-b"));
    const user = userEvent.setup();
    const { onClose, onLinked } = mount();
    await user.click(await screen.findByRole("radio", { name: /Second library/ }));
    await user.click(screen.getByRole("button", { name: "Link and match" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(calls.update).toHaveBeenCalledWith({ variables: { id: "torrent-id", input: { libraryId: "library-b" } } });
    expect(calls.rematch).toHaveBeenCalledWith({ variables: { sourceId: "hash", sourceType: "torrent", libraryId: "library-b" } });
    expect(calls.success).toHaveBeenCalledWith("Linked and matched 3 files");
    expect(onLinked).toHaveBeenCalledOnce();
  });

  it.each([
    ["quietly dropped field", saved("library-a")],
    ["missing returned record", { data: { updateTorrent: { success: true, error: null, torrent: null } } }],
    ["different returned record", saved("library-b", "another-torrent")],
  ])("rejects a reported success with %s", async (_label, response) => {
    calls.update.mockResolvedValue(response);
    const user = userEvent.setup();
    const { onClose, onLinked } = mount();
    await user.click(await screen.findByRole("radio", { name: /Second library/ }));
    await user.click(screen.getByRole("button", { name: "Link and match" }));
    await waitFor(() => expect(calls.danger).toHaveBeenCalledWith("The download's library link was not saved. Refresh and try again."));
    expect(calls.rematch).not.toHaveBeenCalled();
    expect(calls.success).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(onLinked).not.toHaveBeenCalled();
  });

  it("confirms explicit null unlinking and does not rematch a completed download", async () => {
    calls.update.mockResolvedValue(saved(null));
    const user = userEvent.setup();
    const { onClose, onLinked } = mount();
    await user.click(await screen.findByRole("button", { name: "Remove link" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(calls.update).toHaveBeenCalledWith({ variables: { id: "torrent-id", input: { libraryId: null } } });
    expect(calls.rematch).not.toHaveBeenCalled();
    expect(calls.success).toHaveBeenCalledWith("Link removed");
    expect(onLinked).toHaveBeenCalledOnce();
  });

  it("keeps a denied update open without triggering any successful follow-up", async () => {
    calls.update.mockRejectedValue(new Error("Forbidden"));
    const user = userEvent.setup();
    const { onClose, onLinked } = mount();
    await user.click(await screen.findByRole("radio", { name: /Second library/ }));
    await user.click(screen.getByRole("button", { name: "Link and match" }));
    await waitFor(() => expect(calls.danger).toHaveBeenCalledWith("Forbidden"));
    expect(calls.rematch).not.toHaveBeenCalled();
    expect(calls.success).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(onLinked).not.toHaveBeenCalled();
  });

  it("keeps the chosen library while live progress refreshes the same download", async () => {
    const user = userEvent.setup();
    const { rerender, onClose, onLinked } = mount(row("library-a", 0.5));
    await user.click(await screen.findByRole("radio", { name: /Second library/ }));
    rerender(<LinkTorrentDialog row={row("library-a", 0.7)} onClose={onClose} onLinked={onLinked} />);
    expect(screen.getByRole("radio", { name: /Second library/ })).toHaveAttribute("aria-checked", "true");
  });
});
