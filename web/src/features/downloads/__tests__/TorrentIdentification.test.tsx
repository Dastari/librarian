import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";

import { canChooseAnother, IdentificationActivity, IdentifiedResults, ItemAction, ItemFindings, parseFindings, ResultsContext, StatusCell, TitleCell, type ResultsActions, type TorrentIdentificationState } from "../TorrentIdentification";

vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true }));
vi.mock("@tanstack/react-router", () => ({ Link: ({ children }: { children: ReactNode }) => <a href="/x">{children}</a> }));
vi.mock("@apollo/client/react", () => ({
  useMutation: () => [vi.fn(), { loading: false }],
  useLazyQuery: () => [vi.fn(), { loading: false, data: undefined }],
}));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: undefined, loading: false, refetch: vi.fn() }) }));

// jsdom has no Web Animations; the feed's scroll area waits on them.
Element.prototype.getAnimations ??= () => [];

type Item = TorrentIdentificationState["items"][number];
type Event = TorrentIdentificationState["events"][number];

function item(overrides: Partial<Item>): Item {
  return {
    __typename: "IdentifiedItem", id: "item-1", itemKey: "key", kind: "audiobook", status: "identified", label: "The Last Colony (Unabridged)",
    title: "The Last Colony: Old Man's War Book 3", creator: "John Scalzi", year: 2025, coverUrl: null, fileCount: 52, finishedFiles: 52,
    seconds: 35596, confidence: 1, summary: "17 chapters from the listing, placed by time.", evidence: JSON.stringify(["Author matches", "Narrator matches"]),
    providerId: "B0DSQ3SWTY", libraryItemType: null, libraryItemId: null, libraryId: "library-1", season: null, canAccept: false, addsCount: null,
    ...overrides,
  } as Item;
}

function state(items: Item[], events: Event[] = []): TorrentIdentificationState {
  return { items, events, loading: false, error: undefined, refetch: vi.fn() } as unknown as TorrentIdentificationState;
}

const wrap = (children: ReactNode) => render(<TooltipProvider>{children}</TooltipProvider>);

const finding = (source: string, text: string, extra: object = {}) => ({ source, text, warning: false, ...extra });
const actions = (overrides: Partial<ResultsActions> = {}): ResultsActions => ({ canImport: true, busy: null, importIt: vi.fn(), choose: vi.fn(), reviewUpgrade: vi.fn(), ...overrides });
const cells = (children: ReactNode, value = actions()) => wrap(<ResultsContext.Provider value={value}>{children}</ResultsContext.Provider>);

// jsdom has no layout, so the table's rows are checked through the cells it renders.
describe("identified results", () => {
  it("says what was found and who reads it", () => {
    cells(<TitleCell item={item({ identity: JSON.stringify({ narrators: ["William Dufris"] }) })} />);
    expect(screen.getByText("The Last Colony: Old Man's War Book 3")).toBeTruthy();
    expect(screen.getByText(/John Scalzi · read by William Dufris/)).toBeTruthy();
  });

  it("keeps a warning beside the title, with the row closed", () => {
    cells(<StatusCell item={item({ evidence: JSON.stringify([finding("files", "2 files are named as damaged or wrong", { warning: true, short: "2 files damaged" })]) })} />);
    expect(screen.getByText("Ready to add")).toBeTruthy();
    expect(screen.getByText("2 files damaged")).toBeTruthy();
  });

  it("offers the one next thing to do", async () => {
    const value = actions();
    const ready = item({});
    cells(<ItemAction item={ready} />, value);
    await userEvent.click(screen.getByRole("button", { name: "Add to library" }));
    expect(value.importIt).toHaveBeenCalledWith(ready);
    expect(canChooseAnother(ready)).toBe(true);
  });

  it("asks for a choice, and lets a person accept what the rules doubted", async () => {
    const value = actions();
    const doubted = item({ status: "needs_choice", canAccept: true, summary: "Its tags say something else." });
    cells(<><ItemAction item={doubted} /><ItemFindings item={doubted} /></>, value);
    await userEvent.click(screen.getByRole("button", { name: "Choose" }));
    expect(value.choose).toHaveBeenCalledWith(doubted);
    await userEvent.click(screen.getByRole("button", { name: "Import anyway" }));
    expect(value.importIt).toHaveBeenCalledWith(doubted, { acceptReview: true });
    expect(screen.getByText(/Its tags say something else/)).toBeTruthy();
    expect(canChooseAnother(doubted)).toBe(false);
  });

  it("waits until files have finished before anything can be imported", () => {
    cells(<ItemAction item={item({ finishedFiles: 10 })} />);
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("only offers to open what's already imported", () => {
    const imported = item({ status: "imported", libraryItemType: "Audiobook", libraryItemId: "book-1" });
    cells(<ItemAction item={imported} />);
    expect(screen.getByRole("link", { name: "Open" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Add to library" })).toBeNull();
    expect(canChooseAnother(imported)).toBe(false);
  });

  it("says what a result adds to an album the library has, and offers to add just that", async () => {
    const value = actions();
    const adds = item({ kind: "music", status: "adds", addsCount: 2, libraryItemType: "Album", libraryItemId: "album-1" });
    cells(<><StatusCell item={adds} /><ItemAction item={adds} /></>, value);
    expect(screen.getByText("Adds 2 tracks")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Add missing tracks" }));
    expect(value.importIt).toHaveBeenCalledWith(adds);
  });

  it("opens what the library already holds in full, with nothing to import", () => {
    const held = item({ kind: "music", status: "in_library", libraryItemType: "Album", libraryItemId: "album-1" });
    cells(<><StatusCell item={held} /><ItemAction item={held} /></>);
    expect(screen.getByText("In library")).toBeTruthy();
    expect(screen.getByRole("link", { name: "Open" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Import" })).toBeNull();
  });

  it("asks before putting better copies in place of what the library has", async () => {
    const value = actions();
    const better = item({ kind: "music", status: "upgrade", addsCount: 3, libraryItemType: "Album", libraryItemId: "album-1" });
    cells(<><StatusCell item={better} /><ItemAction item={better} /></>, value);
    expect(screen.getByText("Upgrade available")).toBeTruthy();
    await userEvent.click(screen.getByRole("button", { name: "Review upgrade" }));
    expect(value.reviewUpgrade).toHaveBeenCalledWith(better);
    expect(value.importIt).not.toHaveBeenCalled();
  });

  it("offers nothing to someone who can't import", () => {
    cells(<ItemAction item={item({})} />, actions({ canImport: false }));
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("groups the evidence by where it came from, and leaves out sources with nothing to say", () => {
    cells(<ItemFindings item={item({ evidence: JSON.stringify([
      finding("files", "Files grouped by their album tags"),
      finding("listing", "Author matches"),
      finding("heard", "\u201cRead for you by Tavia Gilbert\u201d"),
    ]) })} />);
    expect(screen.getByText("In the files")).toBeTruthy();
    expect(screen.getByText("Online listing")).toBeTruthy();
    expect(screen.getByText("Heard in the opening")).toBeTruthy();
    expect(screen.getByText("\u201cRead for you by Tavia Gilbert\u201d")).toBeTruthy();
    expect(screen.queryByText("Jev")).toBeNull();
    expect(screen.getByText(/Files say: The Last Colony \(Unabridged\)/)).toBeTruthy();
  });

  it("still shows results from before evidence recorded its source", () => {
    expect(parseFindings(JSON.stringify(["Author matches"]))).toEqual([{ source: "other", text: "Author matches", warning: false }]);
    expect(parseFindings("not json")).toEqual([]);
    cells(<ItemFindings item={item({ evidence: JSON.stringify(["Author matches"]) })} />);
    expect(screen.getByText("What was found")).toBeTruthy();
    expect(screen.getByText("Author matches")).toBeTruthy();
  });

  it("says when nothing has been identified yet", () => {
    wrap(<IdentifiedResults infoHash="hash" state={state([])} />);
    expect(screen.getByText("Nothing identified yet")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Run again/ })).toBeTruthy();
  });
});

describe("IdentificationActivity", () => {
  const step = (sequence: number, overrides: Partial<Event>): Event => ({
    __typename: "IdentificationEvent", id: `step-${sequence}`, runId: "run-1", trigger: "Download finished", sequence,
    status: "done", message: `Step ${sequence}`, detail: null, durationMs: null, createdAt: "2026-11-06T09:00:00Z", ...overrides,
  } as Event);

  it("shows the newest run's steps and timings, and what a step found when it's chosen", async () => {
    wrap(<IdentificationActivity state={state([], [
      step(0, { message: "190 audio files, 1 text file", durationMs: 3200, detail: "190 of 191 files finished" }),
      step(1, { status: "found", message: "This looks like an audiobook" }),
    ])} />);
    expect(screen.getByText("Download finished")).toBeTruthy();
    expect(screen.getByText(/190 audio files/)).toBeTruthy();
    expect(screen.getAllByText(/3\.2 s/).length).toBeGreaterThan(0);
    expect(screen.queryByText("190 of 191 files finished")).toBeNull();
    await userEvent.click(screen.getByText(/190 audio files/));
    expect(screen.getByText("190 of 191 files finished")).toBeTruthy();
  });

  it("keeps older runs to a line until one is chosen", async () => {
    wrap(<IdentificationActivity state={state([], [
      step(0, { id: "new-0", runId: "run-2", trigger: "Run by hand", message: "Newest step" }),
      step(0, { id: "old-0", runId: "run-1", trigger: "Files found", message: "Oldest step" }),
    ])} />);
    expect(screen.getByText("Newest step")).toBeTruthy();
    expect(screen.queryByText("Oldest step")).toBeNull();
    await userEvent.click(screen.getByText("Files found"));
    expect(screen.getByText("Oldest step")).toBeTruthy();
    expect(screen.queryByText("Newest step")).toBeNull();
  });

  it("says when nothing has been looked at", () => {
    wrap(<IdentificationActivity state={state([])} />);
    expect(screen.getByText("Nothing has been looked at yet.")).toBeTruthy();
  });
});
