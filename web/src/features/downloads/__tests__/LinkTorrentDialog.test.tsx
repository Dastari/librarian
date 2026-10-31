import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
  useCompleteConnection: () => ({ rows: [{ id: "library-a", name: "First library" }, { id: "library-b", name: "Second library" }], loading: false, error: undefined }),
}));
vi.mock("glass-ux/toast", () => ({ toast: { success: calls.success, danger: calls.danger, warning: calls.warning } }));
// Keep the real controlled dialog/save behavior; presentation primitives are
// immaterial to proving that persistence precedes rematch, toast and dismissal.
vi.mock("@/components/ui", () => ({
  Button: ({ children, onPress, isDisabled }: { children: ReactNode; onPress: () => void; isDisabled?: boolean }) => <button disabled={isDisabled} onClick={onPress}>{children}</button>,
  Dialog: ({ isOpen, title, children, footer }: { isOpen: boolean; title: string; children: ReactNode; footer: ReactNode }) => isOpen ? <section role="dialog" aria-label={title}>{children}{footer}</section> : null,
  ErrorState: () => null,
}));
vi.mock("glass-ux/select", () => ({
  Select: ({ value, onValueChange, options }: { value: string | null; onValueChange: (value: string | null) => void; options: Array<{ value: string; label: string }> }) => <select aria-label="Library" value={value ?? ""} onChange={(event) => onValueChange(event.target.value || null)}>
    <option value="">Unlinked</option>
    {options.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
  </select>,
}));

// Only the dialog's consumed fields are relevant to this mutation-contract test.
const row = (libraryId: string | null = "library-a", progress = 1) => ({
  record: { id: "torrent-id", libraryId }, live: { id: 1, infoHash: "hash", name: "Download", progress },
}) as unknown as DownloadRow;
const saved = (libraryId: string | null, id = "torrent-id") => ({ data: { updateTorrent: { success: true, error: null, torrent: { id, libraryId } } } });
const mount = (download = row()) => {
  const onClose = vi.fn(); const onLinked = vi.fn();
  const view = render(<LinkTorrentDialog row={download} onClose={onClose} onLinked={onLinked} />);
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
    await user.selectOptions(screen.getByLabelText("Library"), "library-b");
    await user.click(screen.getByRole("button", { name: "Link" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(calls.update).toHaveBeenCalledWith({ variables: { id: "torrent-id", input: { libraryId: "library-b" } } });
    expect(calls.rematch).toHaveBeenCalledWith({ variables: { sourceId: "torrent-id", sourceType: "torrent", libraryId: "library-b" } });
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
    await user.selectOptions(screen.getByLabelText("Library"), "library-b");
    await user.click(screen.getByRole("button", { name: "Link" }));
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
    await user.selectOptions(screen.getByLabelText("Library"), "");
    await user.click(screen.getByRole("button", { name: "Link" }));
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
    expect(calls.update).toHaveBeenCalledWith({ variables: { id: "torrent-id", input: { libraryId: null } } });
    expect(calls.rematch).not.toHaveBeenCalled();
    expect(calls.success).toHaveBeenCalledWith("Unlinked");
    expect(onLinked).toHaveBeenCalledOnce();
  });

  it("keeps a denied update open without triggering any successful follow-up", async () => {
    calls.update.mockRejectedValue(new Error("Forbidden"));
    const user = userEvent.setup();
    const { onClose, onLinked } = mount();
    await user.selectOptions(screen.getByLabelText("Library"), "library-b");
    await user.click(screen.getByRole("button", { name: "Link" }));
    await waitFor(() => expect(calls.danger).toHaveBeenCalledWith("Forbidden"));
    expect(calls.rematch).not.toHaveBeenCalled();
    expect(calls.success).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
    expect(onLinked).not.toHaveBeenCalled();
  });

  it("keeps the chosen library while live progress refreshes the same download", async () => {
    const user = userEvent.setup();
    const { rerender, onClose, onLinked } = mount(row("library-a", 0.5));
    await user.selectOptions(screen.getByLabelText("Library"), "library-b");
    rerender(<LinkTorrentDialog row={row("library-a", 0.7)} onClose={onClose} onLinked={onLinked} />);
    expect(screen.getByLabelText("Library")).toHaveValue("library-b");
  });
});
