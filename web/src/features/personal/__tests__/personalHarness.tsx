import { ApolloClient, ApolloLink, InMemoryCache, type Operation } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import { act, render, waitFor } from "@testing-library/react";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { buildSchema, getOperationAST, getVariableValues, validate, type DocumentNode } from "graphql";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactElement } from "react";
import { Observable } from "rxjs";
import { expect, vi } from "vitest";

import type { HomeShelfFieldsFragment, PersonalEntryFragment, PlaylistCoverEntryFragment, PersonalListFieldsFragment, SavedViewFieldsFragment } from "@/graphql/generated/graphql";
import { createTestRouter, TestRouterProvider } from "@/test/router";
import { LiveContext } from "@/lib/graphql/live/hooks";
import { LiveRegistry } from "@/lib/graphql/live/registry";

export const OWNER = "admin-self";
export const STAMP = "2026-10-26T00:00:00Z";

export const personalList = (overrides: Partial<PersonalListFieldsFragment> = {}): PersonalListFieldsFragment & { __typename: "PersonalList" } => ({
  __typename: "PersonalList",
  id: "list-a", userId: OWNER, name: "Evening films", kind: "MANUAL", description: null, savedViewId: null,
  createdAt: STAMP, updatedAt: STAMP, ...overrides,
});
export const movieFields = (id: string, title: string, overrides: Record<string, unknown> = {}) => ({
  __typename: "Movie" as const,
  id, libraryId: "library-a", userId: OWNER, title, sortTitle: null, originalTitle: null, year: 2026, tmdbId: null, imdbId: null,
  overview: null, tagline: null, runtime: 100, genres: [], lockedFields: null, director: null, castNames: [], productionCountries: [],
  spokenLanguages: [], tmdbRating: null, tmdbVoteCount: null, collectionId: null, collectionName: null, collectionPosterUrl: null,
  releaseDate: null, certification: null, monitored: false, tmdbStatus: null, wanted: false, ignored: false, downloadStatus: null,
  hasFile: false, mediaFileId: null, qualityProfileId: null, createdAt: STAMP, updatedAt: STAMP, ...overrides,
});
export const trackFields = (id: string, title: string, overrides: Record<string, unknown> = {}) => ({
  __typename: "Track" as const,
  id, title, albumId: "album-a", libraryId: "library-a", trackNumber: 1, discNumber: 1, musicbrainzId: null, isrc: null, durationSecs: 180,
  explicit: false, artistName: "Artist", artistId: null, wanted: false, ignored: false, mediaFileId: `file-${id}`, createdAt: STAMP, updatedAt: STAMP,
  album: { __typename: "Album" as const, id: "album-a", name: "Record", coverUrl: null }, ...overrides,
});
/** A playlist entry as the server returns it. Each target is a full record (see `movieFields`, `trackFields`). */
export const personalEntry = (index: number, overrides: Record<string, unknown> = {}) => ({
  __typename: "ListEntry" as const,
  id: `entry-${index}`, userId: OWNER, listId: "list-a", position: index * 10,
  movieId: `movie-${index}`, showId: null, albumId: null, audiobookId: null, trackId: null, episodeId: null, chapterId: null,
  movie: null, show: null, album: null, audiobook: null, track: null, episode: null, chapter: null, ...overrides,
}) as unknown as PersonalEntryFragment & { __typename: "ListEntry" };
/** The playlist with its entry count and first entries, as the playlist queries return it. */
export const playlist = (overrides: Record<string, unknown> = {}, count = 0, covers: PlaylistCoverEntryFragment[] = []) => ({
  ...personalList(),
  entries: { __typename: "ListEntryConnection" as const, edges: covers.map((node) => ({ __typename: "ListEntryEdge" as const, node })), pageInfo: { __typename: "PageInfo" as const, totalCount: count } },
  ...overrides,
});
export const homeShelf = (overrides: Partial<HomeShelfFieldsFragment> = {}): HomeShelfFieldsFragment & { __typename: "HomeShelf" } => ({
  __typename: "HomeShelf",
  id: "shelf-a", userId: OWNER, shelfKey: "list:list-a", position: 8192, visible: true,
  listId: "list-a", libraryId: null, createdAt: STAMP, updatedAt: STAMP, ...overrides,
});
export const savedView = (overrides: Partial<SavedViewFieldsFragment> = {}): SavedViewFieldsFragment & { __typename: "SavedView" } => ({
  __typename: "SavedView",
  id: "view-a", userId: OWNER, name: "Night films", version: 1, contentType: "MOVIE", libraryId: "library-a",
  titleContains: "Night", availability: "AVAILABLE", sort: "TITLE", descending: false,
  createdAt: STAMP, updatedAt: STAMP, ...overrides,
});

export function connection(field: string, nodes: object[], hasNextPage = false, totalCount = nodes.length) {
  return { [field]: { edges: nodes.map((node) => ({ node })), pageInfo: { hasNextPage, hasPreviousPage: false, totalCount } } };
}
export function createdEntry(entry: PersonalEntryFragment = personalEntry(0)) {
  return { createListEntry: { success: true, error: null, listEntry: { ...entry, __typename: "ListEntry" as const, userId: OWNER, createdAt: STAMP, updatedAt: STAMP } } };
}

export function operationName(document: DocumentNode) {
  return getOperationAST(document)?.name?.value;
}
let schema: ReturnType<typeof buildSchema> | undefined;
export function personalSchema() {
  return schema ??= buildSchema(readFileSync(resolve(process.cwd(), "schema.graphql"), "utf8"));
}
/** Check the real generated operation and wire variables against the exported server SDL. */
export function expectValidOperation(operation: Operation) {
  const document = getOperationAST(operation.query)!;
  const contract = personalSchema();
  expect(validate(contract, operation.query).map((error) => error.message)).toEqual([]);
  expect(getVariableValues(contract, document.variableDefinitions ?? [], operation.variables).errors).toBeUndefined();
}

export interface PersonalRequest {
  operation: Operation;
  resolve: (data: Record<string, unknown>) => void;
  reject: (error: Error) => void;
}
/** Mock only the transport. Apollo, generated documents, complete loaders and Glass stay real. */
export function personalHarness(options: { live?: boolean } = {}) {
  const requests: PersonalRequest[] = [];
  const registry = new LiveRegistry(() => () => {}, { coalesceMs: 0, minGapMs: 0, concurrency: 4, maxRetries: 0 });
  registry.reset(Boolean(options.live));
  const client = new ApolloClient({ cache: new InMemoryCache(), link: new ApolloLink((operation) => new Observable((observer) => {
    requests.push({ operation, resolve: (data) => { observer.next({ data }); observer.complete(); }, reject: (error) => observer.error(error) });
  })) });
  return {
    requests, registry,
    async mount(ui: ReactElement) {
      const content = options.live ? <LiveContext.Provider value={{ registry, epoch: OWNER }}>{ui}</LiveContext.Provider> : ui;
      const router = createTestRouter(<ApolloProvider client={client}><TooltipProvider><ActionProvider>{content}</ActionProvider></TooltipProvider></ApolloProvider>);
      await router.load();
      return render(<TestRouterProvider router={router} />);
    },
    async next(document: DocumentNode, index = 0) {
      const matching = () => requests.filter(({ operation }) => operation.operationName === operationName(document));
      await waitFor(() => expect(matching().length).toBeGreaterThan(index));
      return matching()[index]!;
    },
    mutations() {
      return requests.filter(({ operation }) => getOperationAST(operation.query)?.operation === "mutation");
    },
  };
}
export async function respond(request: PersonalRequest, data: Record<string, unknown>) {
  await act(async () => { request.resolve(data); });
}

/** Supply only geometry that jsdom cannot measure; the actual Glass table remains mounted. */
export function installTableGeometry() {
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(() => ({
    width: 1200, height: 700, top: 0, left: 0, bottom: 700, right: 1200, x: 0, y: 0, toJSON: () => ({}),
  }));
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : 1200; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 700; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(700);
}

export function observeTableEnd() {
  const callbacks: IntersectionObserverCallback[] = [];
  vi.stubGlobal("IntersectionObserver", class {
    constructor(callback: IntersectionObserverCallback) { callbacks.push(callback); }
    observe() {}
    unobserve() {}
    disconnect() {}
  });
  return {
    intersect: () => act(() => {
      callbacks.at(-1)?.([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver);
    }),
    get count() { return callbacks.length; },
  };
}
