// @vitest-environment node
import { describe, expect, it } from "vitest";
import {
  ActiveDownloadCountDocument, BackupOverviewDocument, ContentStatusesDocument, EntityMovieGetDocument,
  EntityMovieListDocument, LibrariesOverviewDocument, ShowEpisodesDocument,
} from "@/graphql/generated/graphql";
import { queryDependencies } from "../documents";

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
    expect(queryDependencies(EntityMovieGetDocument, { id: "one" })).toContainEqual({ topic: "Movie", ids: ["one"] });
    expect(queryDependencies(EntityMovieListDocument)).toContainEqual({ topic: "Movie", ids: undefined });
  });
  it("keeps computed status and runtime pause/resume dependencies", () => {
    expect(topics(ContentStatusesDocument)).not.toContain("AppSetting");
    expect(topics(ContentStatusesDocument)).toEqual(expect.arrayContaining(["Torrent", "PendingFileMatch", "MediaFile", "QualityProfile", "LibraryScanIssue", "PlaybackProgress"]));
    expect(topics(ActiveDownloadCountDocument)).toEqual(expect.arrayContaining(["Torrent", "TorrentProgress", "TorrentAdded", "TorrentCompleted", "TorrentRemoved"]));
  });
});

it("treats explicit ORM broadcast lag as resnapshot plus reattachment", async () => {
  const { CombinedGraphQLErrors } = await import("@apollo/client");
  const { GraphQLError } = await import("graphql");
  const { apolloLiveSource } = await import("../documents");
  let interrupted = 0;
  const error = new CombinedGraphQLErrors({ errors: [new GraphQLError("resnapshot", { extensions: { code: "LIVE_RESYNC_REQUIRED" } })] });
  const client = { subscribe: () => ({ subscribe: (observer: { error: (error: unknown) => void }) => {
    observer.error(error);
    return { unsubscribe() {} };
  } }) };
  apolloLiveSource(client as never, false)("Movie", () => {}, () => { interrupted++; });
  expect(interrupted).toBe(1);
});
