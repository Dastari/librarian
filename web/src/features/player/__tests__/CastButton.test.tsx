import { ApolloClient, ApolloLink, InMemoryCache, type Operation } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import { getOperationAST, type DocumentNode } from "graphql";
import type { ReactElement } from "react";
import { Observable } from "rxjs";
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  CastMediaDocument, CastPauseDocument, CastPlayDocument, CastSeekDocument,
  CastSetMutedDocument, CastSetVolumeDocument, CastStopDocument,
  EntityCastDeviceListDocument, EntityCastSessionGetDocument, EntityCastSessionListDocument,
  type CastDeviceFieldsFragment, type CastSessionFieldsFragment, type LegacyCastSessionFieldsFragment,
} from "@/graphql/generated/graphql";
import { LiveContext } from "@/lib/graphql/live/hooks";
import { LiveRegistry } from "@/lib/graphql/live/registry";
import { liveTransport } from "@/lib/graphql/live/transport";

import { CastButton, type CastButtonProps } from "../CastButton";
import type { PlayItem } from "../store";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "actual-admin", role: "admin" }, status: "authenticated" }) }));
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const STAMP = "2026-10-27T12:00:00Z";
const ITEM: PlayItem = { mediaFileId: "actual-file", title: "Example film", entity: { kind: "movie", id: "movie-id" }, startPosition: 12 };
function receiver(overrides: Partial<CastDeviceFieldsFragment> = {}): CastDeviceFieldsFragment {
  return {
    id: "receiver-1", name: "Living room Google TV", address: "192.168.1.20", port: 8009,
    model: "Chromecast with Google TV", deviceType: "CHROMECAST", enabled: true,
    playbackSupported: true, isFavorite: false, isManual: false, discoveryOrigin: "mdns",
    firstSeenAt: STAMP, lastSeenAt: STAMP, lastProbeAt: STAMP, lastProbeError: null,
    createdAt: STAMP, updatedAt: STAMP, ...overrides,
  };
}
function persisted(overrides: Partial<CastSessionFieldsFragment> = {}): CastSessionFieldsFragment {
  return {
    id: "persisted-session", userId: "actual-admin", deviceId: "receiver-1", mediaFileId: "actual-file",
    episodeId: null, receiverAddress: "192.168.1.20", receiverTransportId: "receiver-transport",
    receiverSessionId: "receiver-session", mediaSessionId: 701, grantExpiresAt: STAMP,
    playerState: "PLAYING", currentPosition: 90, duration: 600, volume: 0.4, isMuted: false,
    startedAt: STAMP, endedAt: null, lastPosition: 90, lastError: null,
    playbackDecision: "DIRECT_PLAY", playbackReason: null, createdAt: STAMP, updatedAt: STAMP, ...overrides,
  };
}
function legacy(row = persisted()): LegacyCastSessionFieldsFragment {
  return {
    id: row.id, deviceId: row.deviceId, deviceName: "Living room Google TV", mediaFileId: row.mediaFileId,
    episodeId: row.episodeId, playerState: row.playerState, currentTime: row.currentPosition,
    duration: row.duration, volume: row.volume, isMuted: row.isMuted, startedAt: row.startedAt,
    lastError: row.lastError, playbackDecision: row.playbackDecision, playbackReason: row.playbackReason,
  };
}
function connection(field: string, rows: object[], offset = 0, limit = 100) {
  return { [field]: { edges: rows.slice(offset, offset + limit).map((node) => ({ node })), pageInfo: {
    hasNextPage: offset + limit < rows.length, hasPreviousPage: offset > 0, totalCount: rows.length,
  } } };
}
const name = (document: DocumentNode) => getOperationAST(document)!.name!.value;
interface Request {
  operation: Operation;
  resolve: (data: Record<string, unknown>) => void;
}

/** Only the network is simulated. These tests do not qualify real receiver playback. */
function castHarness(options: { devices?: CastDeviceFieldsFragment[]; sessions?: CastSessionFieldsFragment[] } = {}) {
  const requests: Request[] = [];
  const state = {
    devices: options.devices ?? [receiver()], sessions: options.sessions ?? [],
    errors: new Set<string>(),
  };
  const registry = new LiveRegistry(() => () => {}, { coalesceMs: 0, minGapMs: 0, concurrency: 4, maxRetries: 0 });
  registry.reset(true);
  const client = new ApolloClient({ cache: new InMemoryCache(), link: new ApolloLink((operation) => new Observable((observer) => {
    const request = { operation, resolve: (data: Record<string, unknown>) => { observer.next({ data }); observer.complete(); } };
    requests.push(request);
    if (getOperationAST(operation.query)?.operation !== "query") return;
    if (operation.operationName && state.errors.has(operation.operationName)) { observer.error(new Error(`${operation.operationName} unavailable`)); return; }
    const { offset = 0, limit = 100 } = operation.variables.page ?? {};
    if (operation.operationName === name(EntityCastDeviceListDocument)) request.resolve(connection("castDevices", state.devices, offset, limit));
    else if (operation.operationName === name(EntityCastSessionListDocument)) request.resolve(connection("castSessions", state.sessions, offset, limit));
    else if (operation.operationName === name(EntityCastSessionGetDocument)) request.resolve({ castSession: state.sessions.find((row) => row.id === operation.variables.id) ?? null });
    else observer.error(new Error(`Unexpected query ${operation.operationName}`));
  })) });
  const wrap = (ui: ReactElement) => <ApolloProvider client={client}><LiveContext.Provider value={{ registry, epoch: "actual-admin" }}><TooltipProvider><ActionProvider>{ui}</ActionProvider></TooltipProvider></LiveContext.Provider></ApolloProvider>;
  return {
    state, requests, registry,
    mount: (props: CastButtonProps = {}) => {
      const view = render(wrap(<CastButton {...props} />));
      return { ...view, rerender: (next: CastButtonProps) => view.rerender(wrap(<CastButton {...next} />)) };
    },
    reads: (document: DocumentNode) => requests.filter(({ operation }) => operation.operationName === name(document)),
    async next(document: DocumentNode, index = 0) {
      const matching = () => requests.filter(({ operation }) => operation.operationName === name(document));
      await waitFor(() => expect(matching().length).toBeGreaterThan(index));
      return matching()[index]!;
    },
    async reply(request: Request, data: Record<string, unknown>) { await act(async () => request.resolve(data)); },
    mutations: () => requests.filter(({ operation }) => getOperationAST(operation.query)?.operation === "mutation"),
  };
}
async function open(props: CastButtonProps = {}, harness = castHarness()) {
  const view = harness.mount(props);
  const trigger = screen.getByRole("button", { name: props.item ? "Cast" : "Casting" });
  await userEvent.click(trigger);
  await screen.findByRole("button", { name: "Living room Google TV" });
  return { ...harness, view, trigger };
}
async function idle() {
  await waitFor(() => expect(screen.getByRole("button", { name: /^(Refresh|Retry)$/ })).not.toHaveAttribute("aria-disabled", "true"));
}

describe("CastButton generated casting workflows", () => {
  it.each(["success", "refusal"])("rejects a delayed A start %s after switching playback to B and back to A", async (outcome) => {
    const onStarted = vi.fn();
    const h = await open({ item: ITEM, onStarted });
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    const oldStart = await h.next(CastMediaDocument);
    h.view.rerender({ item: { ...ITEM, mediaFileId: "file-b", title: "B" }, onStarted });
    h.view.rerender({ item: ITEM, onStarted });
    await h.reply(oldStart, { castMedia: outcome === "success"
      ? { success: true, error: null, session: legacy() }
      : { success: false, error: "Old receiver refusal", session: null } });
    await idle();
    expect(onStarted).not.toHaveBeenCalled();
    expect(screen.queryByText("Start casting succeeded.")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await idle();
    expect(h.mutations()).toHaveLength(1);
  });

  it("rejects a pending start and its retry when a same-file playback activation is replaced", async () => {
    const onStarted = vi.fn();
    const h = await open({ item: ITEM, activationKey: 1, onStarted });
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    const first = await h.next(CastMediaDocument);
    await h.reply(first, { castMedia: { success: false, error: "Try again", session: null } });
    expect(await screen.findByRole("alert")).toHaveTextContent("Try again");
    h.view.rerender({ item: ITEM, activationKey: 2, onStarted });
    await userEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await idle();
    expect(h.mutations()).toHaveLength(1);
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    const second = await h.next(CastMediaDocument, 1);
    h.view.rerender({ item: ITEM, activationKey: 3, onStarted });
    await h.reply(second, { castMedia: { success: true, error: null, session: legacy() } });
    await idle();
    expect(onStarted).not.toHaveBeenCalled();
  });

  it("keeps a pending start valid when only the active item's metadata changes", async () => {
    const onStarted = vi.fn();
    const h = await open({ item: ITEM, activationKey: 1, onStarted });
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    const start = await h.next(CastMediaDocument);
    h.view.rerender({ item: { ...ITEM, title: "Renamed" }, activationKey: 1, onStarted });
    await h.reply(start, { castMedia: { success: true, error: null, session: legacy() } });
    await idle();
    expect(onStarted).toHaveBeenCalledTimes(1);
  });

  it.each([
    { kind: "movie", position: 42.5, expected: { startPosition: 42.5 } },
    { kind: "episode", position: 81.25, expected: { episodeId: "entity-1", startPosition: 81.25 } },
    { kind: "track", position: Number.NaN, expected: {} },
    { kind: "chapter", position: Number.POSITIVE_INFINITY, expected: {} },
  ] as const)("sends actual file and receiver IDs for $kind with finite seconds only", async ({ kind, position, expected }) => {
    const onStarted = vi.fn();
    const h = await open({ item: { ...ITEM, entity: { kind, id: "entity-1" } }, position, onStarted });
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    const start = screen.getByRole("button", { name: "Start casting" });
    await userEvent.click(start);
    const request = await h.next(CastMediaDocument);
    expect(request.operation.variables).toEqual({ input: { mediaFileId: "actual-file", deviceId: "receiver-1", ...expected } });
    expect(start).toHaveFocus();
    expect(start).toHaveAttribute("aria-busy", "true");
    expect(onStarted).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Start casting" })).toBe(start);
    h.state.sessions = [persisted()];
    await h.reply(request, { castMedia: { success: true, error: null, session: legacy() } });
    await waitFor(() => expect(onStarted).toHaveBeenCalledTimes(1));
    await idle();
    expect(await screen.findByRole("group", { name: "Receiver playback" })).toBeInTheDocument();
    // A generated snapshot supplies position; a legacy action payload is not persisted locally.
    expect(screen.getByText("1:30 / 10:00")).toBeInTheDocument();
  });

  it("keeps local playback on refusal, shows the reason, and retries the same payload", async () => {
    const onStarted = vi.fn();
    const h = await open({ item: ITEM, onStarted });
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    const request = await h.next(CastMediaDocument);
    expect(request.operation.variables.input.startPosition).toBe(12);
    await h.reply(request, { castMedia: { success: false, error: "Receiver refused this codec", session: null } });
    expect(await screen.findByRole("alert")).toHaveTextContent("Receiver refused this codec");
    expect(onStarted).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect((await h.next(CastMediaDocument, 1)).operation.variables).toEqual(request.operation.variables);
    expect(onStarted).not.toHaveBeenCalled();
  });

  it("does not report a start when a successful payload has no session", async () => {
    const onStarted = vi.fn();
    const h = await open({ item: ITEM, onStarted });
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    await h.reply(await h.next(CastMediaDocument), { castMedia: { success: true, error: null, session: null } });
    expect(await screen.findByRole("alert")).toHaveTextContent("did not return a cast session");
    expect(onStarted).not.toHaveBeenCalled();
  });

  it("selects the newly started session while retaining older sessions in the complete snapshot", async () => {
    const h = await open({ item: ITEM }, castHarness({ sessions: [persisted({ id: "older-session", currentPosition: 10 })] }));
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    const request = await h.next(CastMediaDocument);
    h.state.sessions = [persisted({ id: "older-session", currentPosition: 10 }), persisted({ id: "new-session", currentPosition: 270 })];
    await h.reply(request, { castMedia: { success: true, error: null, session: legacy(h.state.sessions[1]) } });
    await idle();
    expect(await screen.findByText("4:30 / 10:00")).toBeInTheDocument();
    expect(h.reads(EntityCastSessionGetDocument).at(-1)?.operation.variables).toEqual({ id: "new-session" });
  });

  it("loads every device and active session page with the actual admin owner filter", async () => {
    const devices = Array.from({ length: 103 }, (_, index) => receiver({ id: `device-${index}`, name: `Receiver ${index}` }));
    devices[102] = receiver();
    const sessions = Array.from({ length: 102 }, (_, index) => persisted({ id: `persisted-${index}` }));
    sessions[101] = persisted({ id: "another-user", userId: "other-user" });
    const h = await open({}, castHarness({ devices, sessions }));
    expect(await screen.findByRole("button", { name: "Receiver 101" })).toBeInTheDocument();
    expect(h.reads(EntityCastDeviceListDocument).map(({ operation }) => operation.variables.page.offset)).toEqual([0, 100]);
    const reads = h.reads(EntityCastSessionListDocument);
    expect(reads.map(({ operation }) => operation.variables.page.offset)).toEqual([0, 100]);
    for (const { operation } of reads) expect(operation.variables.where).toEqual({ userId: { eq: "actual-admin" }, endedAt: { isNull: true } });
    expect(screen.queryByRole("button", { name: "Control another-user" })).not.toBeInTheDocument();
    expect(h.mutations()).toEqual([]);
  });

  it("keeps disabled and DLNA discovery rows visible and refuses their activation", async () => {
    const h = await open({ item: ITEM }, castHarness({ devices: [receiver(), receiver({ id: "dlna", name: "DLNA screen", deviceType: "DLNA_RENDERER", playbackSupported: true }), receiver({ id: "disabled", name: "Disabled screen", enabled: false })] }));
    expect(screen.getByText("DLNA discovery only; playback is unavailable.")).toBeInTheDocument();
    expect(screen.getByText("This receiver is disabled.")).toBeInTheDocument();
    const dlna = screen.getByRole("button", { name: "DLNA screen" });
    expect(dlna).toHaveAttribute("aria-disabled", "true");
    dlna.focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    expect(h.mutations()).toEqual([]);
  });

  it("restores persisted controls after close/reopen and a fresh component mount", async () => {
    const h = await open({}, castHarness({ sessions: [persisted()] }));
    const pause = await screen.findByRole("button", { name: "Pause" });
    expect(pause).toBeInTheDocument();
    const trigger = h.trigger;
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(trigger).toHaveFocus());
    await userEvent.click(trigger);
    expect(await screen.findByRole("button", { name: "Pause" })).toBeInTheDocument();
    h.view.unmount();
    h.mount();
    await userEvent.click(screen.getByRole("button", { name: "Casting" }));
    expect(await screen.findByRole("button", { name: "Pause" })).toBeInTheDocument();
    expect(h.mutations()).toEqual([]);
  });

  it("uses the persisted session ID and seconds/0–1 units for all remote transport actions", async () => {
    // Base UI hides inset thumbs until it can measure their position. jsdom
    // supplies no layout, so give only the real slider controls/thumbs geometry.
    const getBounds = HTMLElement.prototype.getBoundingClientRect;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const thumb = this.classList.contains("fa-slider-thumb");
      if (!thumb && !this.classList.contains("fa-slider-control")) return getBounds.call(this);
      const width = thumb ? 16 : 240;
      const height = thumb ? 16 : 24;
      return { width, height, top: 0, left: 0, bottom: height, right: width, x: 0, y: 0, toJSON: () => ({}) };
    });
    const h = await open({}, castHarness({ sessions: [persisted()] }));
    await userEvent.click(await screen.findByRole("button", { name: "Pause" }));
    const paused = await h.next(CastPauseDocument);
    expect(paused.operation.variables).toEqual({ sessionId: "persisted-session" });
    h.state.sessions = [persisted({ playerState: "PAUSED" })];
    await h.reply(paused, { castPause: { success: true, error: null, session: legacy(h.state.sessions[0]) } });
    await idle();
    await userEvent.click(await screen.findByRole("button", { name: "Play" }));
    const played = await h.next(CastPlayDocument);
    expect(played.operation.variables).toEqual({ sessionId: "persisted-session" });
    h.state.sessions = [persisted()];
    await h.reply(played, { castPlay: { success: true, error: null, session: legacy() } });
    await idle();

    screen.getByRole("slider", { name: "Receiver position" }).focus();
    await userEvent.keyboard("{ArrowRight}");
    const sought = await h.next(CastSeekDocument);
    expect(sought.operation.variables).toEqual({ sessionId: "persisted-session", position: 91 });
    h.state.sessions = [persisted({ currentPosition: 91 })];
    await h.reply(sought, { castSeek: { success: true, error: null, session: legacy(h.state.sessions[0]) } });
    await idle();

    screen.getByRole("slider", { name: "Receiver volume" }).focus();
    await userEvent.keyboard("{ArrowRight}");
    const changed = await h.next(CastSetVolumeDocument);
    expect(changed.operation.variables.sessionId).toBe("persisted-session");
    expect(changed.operation.variables.volume).toBeCloseTo(0.41);
    h.state.sessions = [persisted({ currentPosition: 91, volume: 0.41 })];
    await h.reply(changed, { castSetVolume: { success: true, error: null, session: legacy(h.state.sessions[0]) } });
    await idle();

    await userEvent.click(screen.getByRole("button", { name: "Mute" }));
    const muted = await h.next(CastSetMutedDocument);
    expect(muted.operation.variables).toEqual({ sessionId: "persisted-session", muted: true });
    h.state.sessions = [persisted({ isMuted: true })];
    await h.reply(muted, { castSetMuted: { success: true, error: null, session: legacy(h.state.sessions[0]) } });
    await idle();
    await userEvent.click(await screen.findByRole("button", { name: "Unmute" }));
    const unmuted = await h.next(CastSetMutedDocument, 1);
    expect(unmuted.operation.variables).toEqual({ sessionId: "persisted-session", muted: false });
    h.state.sessions = [persisted()];
    await h.reply(unmuted, { castSetMuted: { success: true, error: null, session: legacy() } });
    await idle();

    await userEvent.click(screen.getByRole("button", { name: "Stop casting" }));
    const stopped = await h.next(CastStopDocument);
    expect(stopped.operation.variables).toEqual({ sessionId: "persisted-session" });
    h.state.sessions = [];
    await h.reply(stopped, { castStop: { success: true, error: null } });
    await idle();
    expect(screen.getByRole("button", { name: "Refresh" })).toHaveFocus();
    expect(screen.queryByRole("group", { name: "Receiver playback" })).not.toBeInTheDocument();
    expect(h.mutations().map(({ operation }) => operation.operationName)).toEqual(["CastPause", "CastPlay", "CastSeek", "CastSetVolume", "CastSetMuted", "CastSetMuted", "CastStop"]);
  });

  it.each([EntityCastDeviceListDocument, EntityCastSessionListDocument, EntityCastSessionGetDocument])("shows query errors and retries generated reads without casting", async (document) => {
    const h = castHarness({ sessions: [persisted()] });
    h.state.errors.add(name(document));
    h.mount();
    await userEvent.click(screen.getByRole("button", { name: "Casting" }));
    expect(await screen.findByRole("alert")).toHaveTextContent(`${name(document)} unavailable`);
    h.state.errors.clear();
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    expect(h.reads(document).length).toBeGreaterThan(1);
    expect(h.mutations()).toEqual([]);
  });

  it("keeps receiver selection and focused controls through live refreshes", async () => {
    const h = await open({ item: ITEM }, castHarness({ sessions: [persisted()] }));
    await userEvent.click(screen.getByRole("button", { name: "Living room Google TV" }));
    const pause = screen.getByRole("button", { name: "Pause" });
    pause.focus();
    const before = h.reads(EntityCastDeviceListDocument).length;
    h.state.devices = [receiver({ name: "Renamed Google TV" })];
    await act(async () => {
      h.registry.invalidate({ topic: "CastDevice" });
      liveTransport.emit(false);
    });
    await waitFor(() => expect(h.reads(EntityCastDeviceListDocument).length).toBeGreaterThan(before));
    expect(pause).toHaveFocus();
    expect(screen.getByText("Live updates disconnected. Reconnecting…")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Start casting" }));
    expect((await h.next(CastMediaDocument)).operation.variables.input.deviceId).toBe("receiver-1");
  });

  it("shows receiver lastError, command refusal and retry while keeping transport controls mounted", async () => {
    const h = await open({}, castHarness({ sessions: [persisted({ lastError: "Receiver connection timed out" })] }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Receiver connection timed out");
    const pause = screen.getByRole("button", { name: "Pause" });
    await userEvent.click(pause);
    expect(pause).toHaveFocus();
    expect(screen.getByRole("button", { name: "Stop casting" })).toBeInTheDocument();
    await h.reply(await h.next(CastPauseDocument), { castPause: { success: false, error: "Receiver disconnected", session: null } });
    expect(await screen.findByText("Receiver disconnected")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Retry" }));
    expect((await h.next(CastPauseDocument, 1)).operation.variables).toEqual({ sessionId: "persisted-session" });
  });

  it("keeps global Casting reachable with useful no-media and no-receiver states", async () => {
    const h = castHarness({ devices: [] });
    h.mount();
    const trigger = screen.getByRole("button", { name: "Casting" });
    await userEvent.click(trigger);
    expect(await screen.findByText(/No receivers found/)).toBeInTheDocument();
    expect(screen.getByText(/No active casting sessions/)).toHaveTextContent("Choose media in a player");
    expect(screen.getByRole("button", { name: "Start casting" })).toHaveAttribute("aria-disabled", "true");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(trigger).toHaveFocus());
    expect(h.mutations()).toEqual([]);
  });
});
