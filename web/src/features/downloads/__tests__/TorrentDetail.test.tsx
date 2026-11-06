import { render, renderHook, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider, defineActions } from "glass-ux/actions";
import { describe, expect, it, vi } from "vitest";

import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";

import { TorrentDetail } from "../TorrentDetail";
import { fileOutcome, filesSummary, useTorrentFiles } from "../TorrentFiles";
import { TorrentIdentity } from "../TorrentIdentity";
import type { DownloadRow } from "../useDownloads";

const fixture = vi.hoisted(() => ({ detail: undefined as unknown }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true }));
vi.mock("../MatchFileDialog", () => ({ MatchFileDialog: () => null }));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: fixture.detail, loading: false, refetch: vi.fn() }) }));

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
      minimumRatio: 1, minimumSeedTimeMinutes: 120, season: null, excludedFiles: [], deleteFilesOnRemove: false,
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
const mount = (row = download(), onClose = vi.fn()) => render(<TorrentDetail open row={row} actions={actions} onClose={onClose} />, { wrapper });

// jsdom has no Web Animations; Base UI's dialog waits on them when it closes.
Element.prototype.getAnimations ??= () => [];

describe("download details dialog", () => {
  it("uses the same public row identity in the table", () => {
    const table = render(<TorrentIdentity row={download()} />, { wrapper });
    expect(table.container.querySelector('[data-tier="row"]')).toBeInTheDocument();
    expect(screen.getByText(name)).toBeInTheDocument();
  });

  it("names the download and shows why an import failed", async () => {
    mount();
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(screen.getAllByText(name).length).toBeGreaterThan(0);
    expect(screen.getAllByText("Import failed").length).toBeGreaterThan(0);
    expect(screen.getByText(error)).toHaveClass("break-words");
  });

  it("runs shared actions against this download", async () => {
    pause.mockReset();
    const row = download();
    mount(row);
    await screen.findByRole("dialog");
    const direct = screen.queryByRole("button", { name: "Pause" });
    if (direct) await userEvent.click(direct);
    else {
      await userEvent.click(screen.getByRole("button", { name: /More actions/ }));
      await userEvent.click(await screen.findByRole("menuitem", { name: "Pause" }));
    }
    await waitFor(() => expect(pause).toHaveBeenCalled());
    expect(pause.mock.calls[0]?.[0]).toEqual([row]);
  });

  it("shows a pack's files with media first and what became of each one", async () => {
    const dir = "/data/downloads/Pack";
    fixture.detail = { torrent: {
      files: { edges: [
        { node: { id: "nfo", fileIndex: 0, filePath: `${dir}/info.nfo`, relativePath: "info.nfo", fileSize: 10, progress: 1, isExcluded: false } },
        { node: { id: "e2", fileIndex: 2, filePath: `${dir}/S01E02.mkv`, relativePath: "S01E02.mkv", fileSize: 900, progress: 1, isExcluded: false } },
        { node: { id: "e1", fileIndex: 1, filePath: `${dir}/S01E01.mkv`, relativePath: "S01E01.mkv", fileSize: 1000, progress: 1, isExcluded: false } },
        // A stale second row for the same file, as older downloads have.
        { node: { id: "e1-stale", fileIndex: 1, filePath: `${dir}/S01E01.mkv`, relativePath: "S01E01.mkv", fileSize: 1000, progress: 0, isExcluded: false } },
      ] },
      pendingMatches: { edges: [
        { node: { id: "m1", sourcePath: `${dir}/S01E01.mkv`, sourceFileIndex: 1, unmatchedReason: null, copiedAt: "1", copyError: null, episode: { id: "ep1", showId: "s1", season: 1, episode: 1, title: null, show: { id: "s1", name: "Show" } }, movie: null, track: null, chapter: null } },
        { node: { id: "m2", sourcePath: `${dir}/S01E02.mkv`, sourceFileIndex: 2, unmatchedReason: "No safe automatic match found", copiedAt: null, copyError: null, episode: null, movie: null, track: null, chapter: null } },
      ] },
    } };
    // jsdom has no layout, so the table's rows are checked through the hook behind them.
    const { result } = renderHook(() => useTorrentFiles(download()));
    const { files } = result.current;
    expect(files.map((file) => file.name)).toEqual(["S01E01.mkv", "S01E02.mkv", "info.nfo"]);
    expect(files[0]!.progress).toBe(1);
    expect(fileOutcome(files[0]!)?.label).toBe("Imported");
    expect(fileOutcome(files[1]!)).toMatchObject({ label: "No match", detail: "No safe automatic match found" });
    expect(fileOutcome(files[2]!)).toBeNull();
    expect(filesSummary(files)).toBe("1 of 2 media files imported · 1 not matched");
    fixture.detail = undefined;
  });

  it("summarises a list of failed files in the notice, keeping the whole message under Details", async () => {
    const row = download();
    row.record = { ...row.record, postProcessStatus: "unmatched", postProcessError: "2 file(s) did not match any library item: a.mkv, b.mkv (No safe automatic match found)" };
    mount(row);
    expect(await screen.findByText("2 file(s) did not match any library item. See the list below.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("tab", { name: /Details/ }));
    expect(await screen.findByText(/a\.mkv, b\.mkv/)).toBeInTheDocument();
  });

  it("updates live values in place and closes through its own control", async () => {
    const row = download(); const onClose = vi.fn();
    const view = mount(row, onClose);
    await screen.findByRole("dialog");
    view.rerender(<TorrentDetail open row={{ ...row, live: { ...row.live!, peers: 6 } }} actions={actions} onClose={onClose} />);
    expect(screen.getByText("6")).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(onClose).toHaveBeenCalledOnce());
  });

  it("reads as unknown, not zero, when the client has not loaded the torrent", async () => {
    const row = download(); row.live = null;
    mount(row);
    await screen.findByRole("dialog");
    expect(screen.queryByText("0 B/s")).toBeNull();
    expect(screen.getAllByText("Share ratio").length).toBeGreaterThan(0);
    expect(screen.getByText(/uploaded/)).toBeInTheDocument();
  });
});
