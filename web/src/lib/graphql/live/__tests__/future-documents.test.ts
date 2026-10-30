// @vitest-environment node
import { expect, it } from "vitest";
import { BackupChangedDocument, EnabledSourcesChangedDocument, EntityAppLogKeysetDocument, EntityNotificationKeysetDocument, EntityPlaybackProgressKeysetDocument, EntityTorrentChangedDocument, HasEnabledSourcesDocument, ResumeShelfDocument } from "@/graphql/generated/graphql";
import { apolloLiveSource, queryDependencies, topicDocuments } from "../documents";
const topics = (document: Parameters<typeof queryDependencies>[0]) => queryDependencies(document).map((dependency) => dependency.topic);
it("maps generated keyset roots and the authoritative resume shelf", () => {
  expect(topics(EntityAppLogKeysetDocument)).toContain("AppLog");
  expect(topics(EntityNotificationKeysetDocument)).toContain("Notification");
  expect(topics(EntityPlaybackProgressKeysetDocument)).toContain("PlaybackProgress");
  expect(topics(ResumeShelfDocument)).toEqual(expect.arrayContaining(["PlaybackProgress", "MediaFile", "Episode", "Show", "Movie", "Track", "Album", "Chapter", "Audiobook", "Library"]));
  expect(topicDocuments.Torrent).toBe(EntityTorrentChangedDocument);
  expect(topicDocuments.Backup).toBe(BackupChangedDocument);
});

it("maps member source existence to the boolean projection without admin Source topics", () => {
  expect(queryDependencies(HasEnabledSourcesDocument)).toEqual([{ topic: "EnabledSources", ids: undefined }]);
  expect(topicDocuments.EnabledSources).toBe(EnabledSourcesChangedDocument);
});

it("delivers both true and false capability snapshots through the member listener", () => {
  const delivered: string[] = [];
  let stopped = false;
  const client = {
    subscribe: ({ query }: { query: unknown }) => {
      expect(query).toBe(EnabledSourcesChangedDocument);
      return { subscribe: (observer: { next: (result: { data: { enabledSourcesChanged: boolean } }) => void }) => {
        observer.next({ data: { enabledSourcesChanged: true } });
        observer.next({ data: { enabledSourcesChanged: false } });
        return { unsubscribe: () => { stopped = true; } };
      } };
    },
  };
  const stop = apolloLiveSource(client as never, false)("EnabledSources", (event) => delivered.push(event.topic), () => {});
  expect(delivered).toEqual(["EnabledSources", "EnabledSources"]);
  stop();
  expect(stopped).toBe(true);
});
