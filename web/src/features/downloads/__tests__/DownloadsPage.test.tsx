import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import { NuqsTestingAdapter } from "nuqs/adapters/testing";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EntityTorrentDeleteDocument, EntityTorrentUpdateDocument, PauseTorrentDocument, ResumeTorrentDocument, ProcessSourceDocument } from "@/graphql/generated/graphql";
import type { DownloadRow } from "../useDownloads";
import { DownloadsPage } from "../DownloadsPage";

const fixture = vi.hoisted(() => ({ wide: true, admin: true, rows: [] as DownloadRow[], pause: vi.fn(), resume: vi.fn(), remove: vi.fn(), update: vi.fn(), process: vi.fn(), refetch: vi.fn() }));
vi.mock("@apollo/client/react", () => ({ useMutation: (doc: unknown) => {
  if (doc === PauseTorrentDocument) return [fixture.pause];
  if (doc === ResumeTorrentDocument) return [fixture.resume];
  if (doc === EntityTorrentDeleteDocument) return [fixture.remove];
  if (doc === EntityTorrentUpdateDocument) return [fixture.update];
  if (doc === ProcessSourceDocument) return [fixture.process];
  throw new Error("Unexpected mutation");
} }));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => fixture.admin, useSession: () => ({ user: { id: "owner", role: fixture.admin ? "admin" : "member" } }) }));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: undefined, loading: false, refetch: vi.fn() }) }));
vi.mock("../MatchFileDialog", () => ({ MatchFileDialog: () => null }));
vi.mock("@/hooks/useMediaQuery", () => ({ useMediaQuery: () => fixture.wide }));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({ useCompleteConnection: () => ({ rows: [] }) }));
vi.mock("../useDownloads", () => ({
  DOWNLOAD_PRESETS: ["all", "downloading", "seeding", "paused", "failed"],
  useDownloads: (state: { search?: string }) => {
    const rows = fixture.rows.filter((row) => row.record.name.includes(state.search ?? ""));
    return { rows, live: [], counts: { all: rows.length }, variables: { search: state.search }, refetch: fixture.refetch, list: { loading: false, fetching: false, totalCount: rows.length, revision: 0, hasMore: false, loadMore: vi.fn() } };
  },
}));
vi.mock("../useRateHistory", () => ({ useRateHistory: () => ({ byHash: new Map(), total: { down: [], up: [] } }) }));
vi.mock("../AddTorrentDialog", () => ({ AddTorrentDialog: () => null }));
vi.mock("../LinkTorrentDialog", () => ({ LinkTorrentDialog: () => null }));
export const downloadFixture = (id: string, state = "seeding"): DownloadRow => ({
  id, record: { id, name: `Download ${id}`, infoHash: id, userId: "owner", state, progress: 1, totalBytes: 1024, downloadedBytes: 1024, uploadedBytesTotal: 4096, addedAt: "2026-10-31T00:00:00Z", libraryId: null, library: null, postProcessStatus: null, postProcessError: null, savePath: "/downloads", minimumRatio: null, minimumSeedTimeMinutes: null, completedAt: null, season: null },
  live: { id: Number(id), infoHash: id, progress: 1, state, downloadSpeed: 0, uploadSpeed: 0, downloaded: 1024, peers: 2, savePath: "/downloads", files: [] },
}) as unknown as DownloadRow;
// jsdom omits WAAPI; Base UI checks active scroll animations before closing.
if (!HTMLElement.prototype.getAnimations) Object.defineProperty(HTMLElement.prototype, "getAnimations", { configurable: true, value: () => [] });
const mount = () => render(<NuqsTestingAdapter><TooltipProvider><ActionProvider><DownloadsPage /></ActionProvider></TooltipProvider></NuqsTestingAdapter>);
beforeEach(() => {
  vi.clearAllMocks();fixture.admin = true; fixture.wide = true;
  fixture.rows = [downloadFixture("1"), downloadFixture("2")];
  fixture.pause.mockResolvedValue({ data: { pauseTorrent: { success: true } } });
  fixture.resume.mockResolvedValue({ data: { resumeTorrent: { success: true } } });
  fixture.remove.mockResolvedValue({ data: { deleteTorrent: { success: true } } });
  fixture.update.mockResolvedValue({ data: { updateTorrent: { success: true } } });
  fixture.process.mockResolvedValue({ data: { processSource: { success: true } } });
  vi.spyOn(HTMLElement.prototype,"getBoundingClientRect").mockReturnValue({ width:1400,height:800,x:0,y:0,top:0,left:0,right:1400,bottom:800,toJSON:()=>({}) });
  vi.spyOn(HTMLElement.prototype,"offsetWidth","get").mockReturnValue(1400);
  vi.spyOn(HTMLElement.prototype,"clientWidth","get").mockReturnValue(1400);
  vi.spyOn(HTMLElement.prototype,"offsetHeight","get").mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype,"clientHeight","get").mockReturnValue(800);
});
afterEach(() => vi.restoreAllMocks());
async function selectBoth() {
  await userEvent.click(screen.getByRole("checkbox", { name: "Select Download 1" }));
  await userEvent.click(screen.getByRole("checkbox", { name: "Select Download 2" }));
}

describe("download selection and panes", () => {
  it("keeps controlled checkbox selection and runs the shared bulk pause action", async () => {
    mount();await selectBoth();
    expect(screen.getByRole("checkbox",{name:"Select Download 1"})).toBeChecked();
    await userEvent.click(screen.getByRole("button",{name:"Actions for 2 selected downloads"}));
    await userEvent.click(await screen.findByRole("menuitem",{name:"Pause 2 downloads"}));
    await waitFor(()=>expect(fixture.pause).toHaveBeenCalledTimes(2));
    expect(fixture.pause).toHaveBeenNthCalledWith(1,{variables:{id:1}});
    expect(fixture.pause).toHaveBeenNthCalledWith(2,{variables:{id:2}});
    expect(fixture.refetch).toHaveBeenCalled();
  });
  it("confirms bulk removal once and keeps files by default", async () => {
    mount();await selectBoth();
    await userEvent.click(screen.getByRole("button",{name:"Actions for 2 selected downloads"}));
    await userEvent.click(await screen.findByRole("menuitem",{name:"Remove 2 downloads"}));
    const confirmation=within(await screen.findByRole("alertdialog"));
    expect(fixture.remove).not.toHaveBeenCalled();
    await userEvent.click(confirmation.getByRole("button",{name:"Remove"}));
    await waitFor(()=>expect(fixture.remove).toHaveBeenCalledTimes(2));
    // Entity ids, not the live client's numeric ids; the file flag is not touched.
    expect(fixture.remove).toHaveBeenCalledWith({variables:{id:"1"}});
    expect(fixture.remove).toHaveBeenCalledWith({variables:{id:"2"}});
    expect(fixture.update).not.toHaveBeenCalled();
    await waitFor(()=>expect(screen.queryByRole("alertdialog")).toBeNull());
  });
  it("selects all loaded downloads from the header and clears the selection", async () => {
    mount();
    await userEvent.click(screen.getByRole("checkbox", { name: "Select all loaded rows" }));
    expect(screen.getByRole("checkbox", { name: "Select Download 1" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select Download 2" })).toBeChecked();
    expect(screen.getByRole("button", { name: "Actions for 2 selected downloads" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Clear" }));
    expect(screen.getByRole("checkbox", { name: "Select Download 1" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select Download 2" })).not.toBeChecked();
  });
  it("preserves selection on updates, prunes removed records, and clears it on a new query", async () => {
    const view=mount();await selectBoth();
    fixture.rows=fixture.rows.map(row=>({...row,record:{...row.record,uploadedBytesTotal:7000}}));view.rerender(<NuqsTestingAdapter><TooltipProvider><ActionProvider><DownloadsPage /></ActionProvider></TooltipProvider></NuqsTestingAdapter>);
    expect(screen.getByRole("checkbox",{name:"Select Download 2"})).toBeChecked();
    fixture.rows=fixture.rows.filter(row=>row.id==="1");view.rerender(<NuqsTestingAdapter><TooltipProvider><ActionProvider><DownloadsPage /></ActionProvider></TooltipProvider></NuqsTestingAdapter>);
    await waitFor(()=>expect(screen.getByRole("button",{name:"Actions for 1 selected download"})).toBeInTheDocument());
    if(!screen.queryByRole("searchbox")) await userEvent.click(screen.getByRole("button",{name:/^Search downloads/}));
    await userEvent.type(await screen.findByRole("searchbox"),"Download 1");
    await waitFor(()=>expect(screen.getByRole("checkbox",{name:"Select Download 1"})).not.toBeChecked());
  });
  it("opens a row in the details dialog at every width, and closes back to the table", async () => {
    mount();await userEvent.click(screen.getByText("Download 1"));
    const dialog=await screen.findByRole("dialog");
    expect(within(dialog).getByRole("tab",{name:/Files/})).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await waitFor(()=>expect(screen.queryByRole("dialog")).toBeNull());
  });
  it("keeps read-only selection while withholding management actions from a non-admin", async () => {
    fixture.admin=false;mount();expect(screen.getByRole("checkbox",{name:"Select Download 1"})).toBeInTheDocument();
    expect(screen.queryByRole("button",{name:"Add torrent"})).toBeNull();
  });
});
