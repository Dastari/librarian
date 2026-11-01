// @vitest-environment node
import { parse } from "graphql";
import { describe, expect, it } from "vitest";
import {
  ActiveDownloadCountDocument, BackupOverviewDocument, ContentStatusesDocument, EntityMovieGetDocument,
  EntityMovieListDocument, EntitySeasonListDocument, EntitySeasonChangedDocument, LibrariesOverviewDocument, LiveTorrentsDocument, MovieDetailDocument, ShowEpisodesDocument,
} from "@/graphql/generated/graphql";
import { apolloLiveSource, queryDependencies, topicDocuments } from "../documents";

const topics = (document: Parameters<typeof queryDependencies>[0]) => queryDependencies(document).map((dependency) => dependency.topic);
describe("generated live dependencies", () => {
  it("subscribes backup snapshots through their real root fields", () => {
    expect(topics(BackupOverviewDocument)).toEqual(["Backup"]);
  });
  it("covers empty lists and membership/count changes without looking at returned rows", () => {
    expect(topics(EntityMovieListDocument)).toContain("Movie");
    expect(topics(LibrariesOverviewDocument)).toEqual(expect.arrayContaining(["Library", "Movie", "Show", "Album", "Audiobook", "MediaFile"]));
  });
  it("keeps direct child and parent fallbacks and narrows only explicit ID roots", () => {
    expect(topics(ShowEpisodesDocument)).toEqual(expect.arrayContaining(["Episode", "Show", "MediaFile"]));
    expect(queryDependencies(EntityMovieGetDocument, { id: "one" })).toContainEqual({ topic: "Movie", ids: ["one"], direct: true });
    expect(queryDependencies(EntityMovieListDocument)).toContainEqual({ topic: "Movie", ids: undefined, direct: true });
  });
  it("does not watch children whose changes reach the parent through propagation or a parent write", () => {
    const root = (field: string) => topics(parse(`query($id: String!) { ${field}(id: $id) { id } }`));
    expect(root("movie")).not.toContain("MovieCastCredit");
    expect(root("show")).not.toContain("Episode");
    expect(root("album")).not.toContain("Track");
    expect(root("audiobook")).not.toContain("Chapter");
    expect(root("mediaFile")).not.toEqual(expect.arrayContaining(["VideoStream"]));
    expect(root("mediaFile")).not.toEqual(expect.arrayContaining(["AudioStream"]));
    expect(root("mediaFile")).not.toEqual(expect.arrayContaining(["Subtitle"]));
    expect(root("mediaFile")).not.toEqual(expect.arrayContaining(["MediaChapter"]));
    expect(root("libraryScanRun")).not.toContain("LibraryScanIssue");
    expect(root("movie")).toContain("MediaFile");
    expect(root("episode")).toEqual(expect.arrayContaining(["Show", "MediaFile"]));
    expect(root("libraryScanIssue")).toContain("LibraryScanRun");
  });
  it("watches seasons so provider metadata refreshes reach the show page", () => {
    expect(topicDocuments.Season).toBe(EntitySeasonChangedDocument);
    expect(queryDependencies(EntitySeasonListDocument)).toContainEqual({ topic: "Season", ids: undefined, direct: true });
  });
  it("does not mistake a scalar named like a root for that entity", () => {
    expect(topics(ShowEpisodesDocument)).not.toContain("Season");
  });
  it("marks a dependency direct only when the query reads that record's own scalars", () => {
    const movie = queryDependencies(MovieDetailDocument, { id: "one" });
    expect(movie).toContainEqual({ topic: "Library", ids: undefined, direct: true });
    expect(movie).toContainEqual({ topic: "Movie", ids: ["one"] });
    expect(movie.find((dependency) => dependency.topic === "MediaFile")?.direct).toBeUndefined();
    expect(queryDependencies(LibrariesOverviewDocument).find((dependency) => dependency.topic === "Library")?.direct).toBeUndefined();
  });
  it("keeps computed status and runtime pause/resume dependencies", () => {
    expect(topics(ContentStatusesDocument)).not.toContain("AppSetting");
    expect(topics(ContentStatusesDocument)).toEqual(expect.arrayContaining(["Torrent", "PendingFileMatch", "MediaFile", "QualityProfile", "LibraryScanIssue", "PlaybackProgress"]));
    // The shell badge counts stored torrents, so it follows the generated Torrent stream.
    expect(topics(ActiveDownloadCountDocument)).toContain("Torrent");
    expect(topics(LiveTorrentsDocument)).toEqual(expect.arrayContaining(["Torrent", "TorrentProgress", "TorrentAdded", "TorrentCompleted", "TorrentRemoved"]));
  });
});

it.each(["SUBSCRIPTION_CONTINUITY_LOST", "SUBSCRIPTION_SOURCE_CLOSED"])("resnapshots and reattaches once when a stream ends with %s", async (code) => {
  const { CombinedGraphQLErrors } = await import("@apollo/client");
  const { GraphQLError } = await import("graphql");
  const { apolloLiveSource } = await import("../documents");
  const interrupted: Array<boolean | undefined> = [];
  const events: unknown[] = [];
  const error = new CombinedGraphQLErrors({ errors: [new GraphQLError("continuity", { extensions: { code } })] });
  // Apollo reports the error frame through `next`; `observer.error` never fires for it.
  const client = { subscribe: () => ({ subscribe: (observer: { next: (result: { data?: unknown; error?: unknown }) => void }) => {
    observer.next({ error });
    return { unsubscribe() {} };
  } }) };
  apolloLiveSource(client as never, false)("Movie", (event) => events.push(event), (retry) => { interrupted.push(retry); });
  expect(interrupted).toEqual([undefined]);
  expect(events).toEqual([]);
});

const changed = (extra: Record<string, unknown>) => ({ data: { movieChanged: { id: "m1", ...extra } } });
function capture(results: Array<ReturnType<typeof changed>> = []) {
  const calls: Array<{ query: unknown; variables?: Record<string, unknown> }> = [];
  const client = { subscribe: (call: (typeof calls)[number]) => {
    calls.push(call);
    return { subscribe: (observer: { next: (result: unknown) => void }) => { results.forEach((result) => observer.next(result)); return { unsubscribe() {} }; } };
  } };
  return { calls, client };
}

describe("keyed live streams", () => {
  it("opens an entity stream with its keys and an unkeyed one without", () => {
    const { calls, client } = capture();
    apolloLiveSource(client as never, false)("Movie", () => {}, () => {}, ["a", "b"]);
    apolloLiveSource(client as never, false)("Movie", () => {}, () => {});
    expect(calls[0]!.variables).toEqual({ keys: ["a", "b"] });
    expect(calls[1]!.variables).toBeUndefined();
  });
  it("never sends keys to a stream that does not take them", () => {
    const { calls, client } = capture();
    apolloLiveSource(client as never, false)("EnabledSources", () => {}, () => {}, ["a"]);
    expect(calls[0]!.variables).toBeUndefined();
  });
  it("flags propagated events so direct-only readers can ignore them", () => {
    const { client } = capture([changed({ changeKind: "PROPAGATED" }), changed({ changeKind: "DIRECT" })]);
    const events: unknown[] = [];
    apolloLiveSource(client as never, false)("Movie", (event) => events.push(event), () => {});
    expect(events).toEqual([{ topic: "Movie", id: "m1", propagated: true }, { topic: "Movie", id: "m1" }]);
  });
});
