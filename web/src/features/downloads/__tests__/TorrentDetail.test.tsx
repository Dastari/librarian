import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider, defineActions } from "glass-ux/actions";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";

import { TorrentDetail } from "../TorrentDetail";
import { TorrentIdentity } from "../TorrentIdentity";
import type { DownloadRow } from "../useDownloads";

const name = "Fallout.2024.S01E04.1080p.HEVC.x265-MeGusta";
const error = 'Invalid value for argument "input", unknown field "createdAt" of type "CreatePendingFileMatchInput"';
function download(): DownloadRow {
  return {
    id: "torrent-1",
    record: {
      id: "torrent-1", userId: "user-1", infoHash: "99cbe573f0d0bf8c219584fba6e90dff0913daae", magnetUri: null,
      name, state: "seeding", progress: 1, totalBytes: 492096000, downloadedBytes: 492096000,
      uploadedBytes: 0, uploadedBytesTotal: 123024000, savePath: `/data/downloads/${name}`, downloadPath: null,
      sourceUrl: null, sourceFeedId: null, sourceIndexerId: null, libraryId: null,
      postProcessStatus: "failed", postProcessError: error, processedAt: null,
      episodeId: null, movieId: null, trackId: null, chapterId: null, showId: null, albumId: null, audiobookId: null,
      minimumRatio: 1, minimumSeedTimeMinutes: 120, season: null, excludedFiles: [],
      addedAt: "2026-09-07T12:57:00Z", createdAt: "2026-09-07T12:57:00Z", updatedAt: "2026-09-07T13:02:00Z", completedAt: "2026-09-07T13:02:00Z",
      library: null, movie: null, show: null, episode: null, album: null, audiobook: null,
    },
    live: {
      id: 1, infoHash: "99cbe573f0d0bf8c219584fba6e90dff0913daae", name, state: "seeding", progress: 1,
      size: 492096000, downloaded: 492096000, uploaded: 123024000, downloadSpeed: 0, uploadSpeed: 1024,
      peers: 3, savePath: `/data/downloads/${name}`,
      files: Array.from({ length: 30 }, (_, index) => ({ index, path: `${name}/folder/${index ? `screen${index}.png` : `${name}.mkv`}`, size: index ? 1024 : 480000000, progress: 1 })),
    },
  };
}
const pause = vi.fn();
const actions = defineActions<DownloadRow>("download", [{ id: "pause", label: "Pause", group: "manage", run: pause }]);
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;
const mount = (row = download(), onClose = vi.fn()) => render(<TorrentDetail row={row} actions={actions} onClose={onClose} />, { wrapper });

describe("download identity and bounded inspector", () => {
  it("uses the same public row identity for the table and detail header", async () => {
    const row = download();
    const table = await render(<TorrentIdentity row={row} />, { wrapper });
    const identity = table.container.querySelector('[data-tier="row"]');
    expect(identity).toBeInTheDocument();
    expect(screen.getByText(name)).toBeInTheDocument();
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
    table.unmount();
    const detail = await mount(row);
    expect(detail.container.querySelector('header [data-tier="row"]')).toBeInTheDocument();
    expect(screen.getByRole("heading", { name, level: 2 })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close download details" })).toBeInTheDocument();
    expect(detail.container.querySelector("header [data-torrent-identity]")).toHaveClass("flex-nowrap");
  });

  it("leaves glass and outer geometry to its dialog when embedded", () => {
    const view = render(<TorrentDetail row={download()} actions={actions} onClose={vi.fn()} embedded />, { wrapper });
    const root = view.container.querySelector('section[aria-label="Download details"]')!;
    expect(root).toHaveClass("flex", "min-h-0", "overflow-hidden");
    expect(view.container.querySelector(".glass-panel")).toBeNull();
    expect(root).not.toHaveAttribute("data-scroll");
    expect(screen.getByLabelText("Download details content")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Close download details" }).closest("header")).toBeInTheDocument();
    expect(screen.getAllByRole("listitem")).toHaveLength(30);
  });

  it("keeps shared actions in the identity menu and calls them for this download", async () => {
    pause.mockReset();
    const row = download();
    await mount(row);
    await userEvent.click(screen.getByRole("button", { name: `Actions for ${name}` }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Pause" }));
    await waitFor(() => expect(pause).toHaveBeenCalled());
    expect(pause.mock.calls[0]?.[0]).toEqual([row]);
  });

  it("keeps the header outside the only scrolling content slot with all files inside", async () => {
    const row = download();
    const view = await mount(row);
    const panel = view.container.querySelector('section[aria-label="Download details"]')!;
    const body = screen.getByLabelText("Download details content");
    const close = screen.getByRole("button", { name: "Close download details" });
    expect(panel).toHaveAttribute("data-scroll");
    expect(panel).toHaveClass("h-full", "min-h-0", "overflow-hidden");
    expect(body.parentElement).toBe(panel);
    expect(body).not.toContainElement(close);
    expect(body).toContainElement(screen.getByRole("list", { name: "Torrent files" }));
    expect(screen.getAllByRole("listitem")).toHaveLength(30);
    expect(screen.getByRole("list", { name: "Torrent files" })).not.toHaveClass("overflow-y-auto");
    expect(screen.getByText(error)).toHaveClass("break-words");
  });

  it("keeps header controls connected and focused across live state updates", async () => {
    const row = download(); const onClose = vi.fn();
    const view = await mount(row, onClose);
    const close = screen.getByRole("button", { name: "Close download details" });
    close.focus();
    const next = { ...row, live: { ...row.live!, uploadSpeed: 10000, peers: 6 } };
    view.rerender(<TorrentDetail row={next} actions={actions} onClose={onClose} />);
    expect(screen.getByRole("button", { name: "Close download details" })).toBe(close);
    expect(close).toHaveFocus();
    await userEvent.click(close);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("retains stored facts when the client has not loaded the torrent", async () => {
    const row = download(); row.live = null;
    await mount(row);
    expect(screen.getAllByText("Not loaded in the client")).toHaveLength(2);
    expect(screen.getByText("Load the torrent in the client to view its files.")).toBeInTheDocument();
    expect(screen.getByText(row.record.savePath)).toBeInTheDocument();
    expect(screen.getByText("0.25")).toBeInTheDocument();
  });
});
