import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { coerceInputValue, isInputType } from "graphql";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  EntityAlbumListDocument, EntityAudiobookListDocument, EntityLibraryListDocument, EntityMovieListDocument, EntityShowListDocument, EntityTrackListDocument,
  type LibraryFieldsFragment, type TrackFieldsFragment,
} from "@/graphql/generated/graphql";

import { SmartResults, smartRule } from "../SmartResults";
import { connection, expectValidOperation, installTableGeometry, observeTableEnd, operationName, OWNER, personalHarness, personalSchema, respond, savedView, STAMP } from "./personalHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-self", role: "admin" }, status: "authenticated" }) }));

const cases = [
  { kind: "MOVIE", document: EntityMovieListDocument, field: "movies", text: "title", availability: "hasFile", where: "MovieWhereInput" },
  { kind: "SHOW", document: EntityShowListDocument, field: "shows", text: "name", availability: null, where: "ShowWhereInput" },
  { kind: "ALBUM", document: EntityAlbumListDocument, field: "albums", text: "name", availability: "hasFiles", where: "AlbumWhereInput" },
  { kind: "AUDIOBOOK", document: EntityAudiobookListDocument, field: "audiobooks", text: "title", availability: "hasFiles", where: "AudiobookWhereInput" },
  { kind: "TRACK", document: EntityTrackListDocument, field: "tracks", text: "title", availability: "mediaFileId", where: "TrackWhereInput" },
] as const;

function expectValidWhere(name: string, value: unknown) {
  const type = personalSchema().getType(name);
  if (!isInputType(type)) throw new Error(`${name} is not a server input type`);
  const errors: string[] = [];
  coerceInputValue(value, type, (_path, _invalid, error) => errors.push(error.message));
  expect(errors).toEqual([]);
}
const library = (id: string): LibraryFieldsFragment => ({
  id, userId: OWNER, name: id, path: `/media/${id}`, libraryType: "movies", icon: null, color: null,
  autoScan: false, autoOrganize: false, namingPattern: "", scanIntervalMinutes: 60,
  watchForChanges: false, scanning: false, lastScannedAt: null, qualityProfileId: null, createdAt: STAMP, updatedAt: STAMP,
});

describe("smartRule server query contract", () => {
  it.each(cases)("maps $kind title, owner and availability to real server fields", ({ kind, text, availability, where }) => {
    const variants = kind === "SHOW" ? ["ANY"] as const : ["ANY", "AVAILABLE", "MISSING"] as const;
    for (const state of variants) {
      const rule = smartRule(savedView({ contentType: kind, availability: state }), ["library-a"])!;
      const wire = JSON.parse(JSON.stringify(rule.where));
      expectValidWhere(where, wire);
      expect(wire.libraryId).toEqual({ eq: "library-a" });
      expect(wire[text]).toEqual({ contains: "Night" });
      if (kind === "TRACK") expect(wire).not.toHaveProperty("userId");
      else expect(wire.userId).toEqual({ eq: OWNER });
      if (availability && state !== "ANY") {
        expect(wire[availability]).toEqual(availability === "mediaFileId" ? { isNull: state === "MISSING" } : { eq: state === "AVAILABLE" });
      } else if (availability) expect(wire).not.toHaveProperty(availability);
      expect(wire).not.toHaveProperty("wanted");
      expect(rule.orderBy).toEqual([{ [text]: "ASC" }]);
    }
  });

  it.each(cases)("maps $kind date sorting to the generated order fields", ({ kind }) => {
    expect(smartRule(savedView({ contentType: kind, sort: "CREATED_AT", descending: true }), ["library-a"])?.orderBy).toEqual([{ createdAt: "DESC" }]);
    expect(smartRule(savedView({ contentType: kind, sort: "UPDATED_AT", descending: false }), ["library-a"])?.orderBy).toEqual([{ updatedAt: "ASC" }]);
  });

  it("keeps an administrator's all-library rule restricted to that administrator's resolved libraries", () => {
    const owned = ["library-a", "library-b"];
    const movie = smartRule(savedView({ libraryId: null, titleContains: null }), owned)!;
    expect(movie.where).toMatchObject({ userId: { eq: OWNER }, libraryId: { inList: owned } });
    expect(JSON.parse(JSON.stringify(movie.where))).not.toHaveProperty("title");
    const track = smartRule(savedView({ contentType: "TRACK", libraryId: null }), owned)!;
    expect(track.where.libraryId).toEqual({ inList: owned });
    expect(track.where).not.toHaveProperty("userId");
    expectValidWhere("TrackWhereInput", track.where);
  });

  it("refuses empty, foreign-library and unsupported-version scopes instead of generating an empty IN filter", () => {
    expect(() => smartRule(savedView(), [])).toThrow();
    expect(() => smartRule(savedView({ libraryId: null }), [])).toThrow();
    expect(() => smartRule(savedView({ libraryId: "another-owner-library" }), ["library-a"])).toThrow();
    expect(() => smartRule(savedView({ version: 2 }), ["library-a"])).toThrow();
  });
});

describe("SmartResults owner-scoped loading", () => {
  it.each(cases)("queries generated $kind results only after resolving personal libraries", async ({ kind, document, field }) => {
    const view = personalHarness();
    await view.mount(<SmartResults view={savedView({ contentType: kind, availability: "ANY" })} preview />);
    const scope = await view.next(EntityLibraryListDocument);
    expect(scope.operation.variables.where).toEqual({ userId: { eq: OWNER } });
    expect(view.requests).toHaveLength(1);
    expectValidOperation(scope.operation);
    await respond(scope, connection("libraries", [library("library-a")]));
    const results = await view.next(document);
    expectValidOperation(results.operation);
    expect(results.operation.variables.where.libraryId).toEqual({ eq: "library-a" });
    if (kind === "TRACK") expect(results.operation.variables.where).not.toHaveProperty("userId");
    else expect(results.operation.variables.where.userId).toEqual({ eq: OWNER });
    await respond(results, connection(field, []));
    expect(view.mutations()).toEqual([]);
  });

  it("waits beyond 100 owned libraries before issuing the all-library track query", async () => {
    const view = personalHarness();
    await view.mount(<SmartResults view={savedView({ contentType: "TRACK", libraryId: null })} preview />);
    const rows = Array.from({ length: 125 }, (_, index) => library(`library-${index}`));
    const first = await view.next(EntityLibraryListDocument);
    await respond(first, connection("libraries", rows.slice(0, 60), true, rows.length));
    const second = await view.next(EntityLibraryListDocument, 1);
    expect(view.requests.every(({ operation }) => operation.operationName === operationName(EntityLibraryListDocument))).toBe(true);
    await respond(second, connection("libraries", rows.slice(60, 120), true, rows.length));
    const third = await view.next(EntityLibraryListDocument, 2);
    expect(screen.getByRole("status")).toHaveTextContent("Loading collection");
    expect(view.requests.every(({ operation }) => operation.operationName === operationName(EntityLibraryListDocument))).toBe(true);
    await respond(third, connection("libraries", rows.slice(120), false, rows.length));
    const tracks = await view.next(EntityTrackListDocument);
    expect(tracks.operation.variables.where.libraryId).toEqual({ inList: rows.map((row) => row.id) });
    expect(tracks.operation.variables.where).not.toHaveProperty("userId");
    expectValidOperation(tracks.operation);
    await respond(tracks, connection("tracks", []));
  });

  it("issues no media query when the saved library is outside the resolved owner scope", async () => {
    const view = personalHarness();
    await view.mount(<SmartResults view={savedView({ libraryId: "foreign-library" })} preview />);
    await respond(await view.next(EntityLibraryListDocument), connection("libraries", [library("library-a")]));
    expect(await screen.findByText("No libraries available")).toBeInTheDocument();
    expect(view.requests).toHaveLength(1);
    expect(view.mutations()).toEqual([]);
  });
});


const track = (id: string, title: string): TrackFieldsFragment => ({
  id, title, albumId: "album-a", libraryId: "library-a", trackNumber: 1, discNumber: 1,
  musicbrainzId: null, isrc: null, durationSecs: 180, explicit: false, artistName: "Artist", artistId: null,
  wanted: false, ignored: false, mediaFileId: `file-${id}`, createdAt: STAMP, updatedAt: STAMP,
});

describe("SmartResults preview error recovery", () => {
  it("shows an initial result failure with retry and retains cards beside a later refresh error", async () => {
    const view = personalHarness({ live: true });
    await view.mount(<SmartResults view={savedView({ contentType: "TRACK", availability: "ANY" })} preview />);
    await respond(await view.next(EntityLibraryListDocument), connection("libraries", [library("library-a")]));
    const first = await view.next(EntityTrackListDocument);
    await act(async () => { first.reject(new Error("Preview results unavailable")); });
    expect(await screen.findByRole("alert")).toHaveTextContent("Preview results unavailable");
    expect(screen.queryByRole("button", { name: "Preview track" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    const retry = await view.next(EntityTrackListDocument, 1);
    expect(retry.operation.variables).toEqual(first.operation.variables);
    await respond(retry, connection("tracks", [track("preview-track", "Preview track")]));
    const card = await screen.findByRole("button", { name: "Preview track" });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();

    act(() => view.registry.invalidate({ topic: "Track" }));
    const refresh = await view.next(EntityTrackListDocument, 2);
    expect(screen.getByRole("button", { name: "Preview track" })).toBe(card);
    await act(async () => { refresh.reject(new Error("Preview refresh failed")); });
    expect(await screen.findByRole("alert")).toHaveTextContent("Preview refresh failed");
    expect(screen.getByRole("button", { name: "Preview track" })).toBe(card);
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    const refreshRetry = await view.next(EntityTrackListDocument, 3);
    expect(screen.getByRole("button", { name: "Preview track" })).toBe(card);
    await respond(refreshRetry, connection("tracks", [track("preview-track", "Preview track")]));
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Preview track" })).toBe(card);
    expect(view.mutations()).toEqual([]);
    view.registry.reset(false);
  });
});

describe("SmartResults retains its loaded window on library refresh", () => {
  beforeEach(() => installTableGeometry());
  afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

  it.each(["success", "failure"] as const)("retains continued rows during a library refresh and its %s outcome", async (outcome) => {
    const end = observeTableEnd();
    const view = personalHarness({ live: true });
    await view.mount(<SmartResults view={savedView({ contentType: "TRACK", availability: "ANY" })} />);
    await respond(await view.next(EntityLibraryListDocument), connection("libraries", [library("library-a")]));
    const first = await view.next(EntityTrackListDocument);
    const head = [track("track-1", "First track"), track("track-2", "Second track")];
    const tail = [track("track-3", "Third track"), track("track-4", "Track beyond first batch")];
    await respond(first, connection("tracks", head, true, 4));
    await screen.findByText("First track");
    await waitFor(() => expect(end.count).toBeGreaterThan(0));
    end.intersect();
    const continuation = await view.next(EntityTrackListDocument, 1);
    expect(continuation.operation.variables.page.offset).toBe(2);
    await respond(continuation, connection("tracks", tail, false, 4));
    const loaded = await screen.findByText("Track beyond first batch");
    const mediaRequests = () => view.requests.filter(({ operation }) => operation.operationName === operationName(EntityTrackListDocument));
    expect(mediaRequests()).toHaveLength(2);

    act(() => view.registry.invalidate({ topic: "Library" }));
    const refresh = await view.next(EntityLibraryListDocument, 1);
    // The old bug unmounted Results as soon as the library snapshot became fetching,
    // discarding the continuation window even though its authorized scope had not changed.
    expect(screen.getByText("Track beyond first batch")).toBe(loaded);
    expect(screen.queryByText("Loading collection…")).not.toBeInTheDocument();
    expect(mediaRequests()).toHaveLength(2);
    if (outcome === "success") {
      await respond(refresh, connection("libraries", [{ ...library("library-a"), name: "Renamed library" }]));
    } else {
      await act(async () => { refresh.reject(new Error("Library refresh failed")); });
      expect(await screen.findByRole("alert")).toHaveTextContent("Library refresh failed");
      expect(screen.getByText("Track beyond first batch")).toBe(loaded);
      expect(mediaRequests()).toHaveLength(2);
      await userEvent.click(screen.getByRole("button", { name: "Try again" }));
      const retry = await view.next(EntityLibraryListDocument, 2);
      await respond(retry, connection("libraries", [library("library-a")]));
      await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    }
    expect(screen.getByText("Track beyond first batch")).toBe(loaded);
    expect(screen.getByText("First track")).toBeInTheDocument();
    expect(mediaRequests()).toHaveLength(2);
    expect(view.mutations()).toEqual([]);
    view.registry.reset(false);
  });
});
