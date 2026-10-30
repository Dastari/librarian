import type { ApolloClient } from "@apollo/client";
import { Kind, type DocumentNode, type SelectionSetNode } from "graphql";

import * as G from "@/graphql/generated/graphql";
import { session } from "@/lib/auth/session";
import { errorCodes, isForbiddenError, isUnauthorizedError } from "@/lib/graphql/errors";
import type { LiveDependency, LiveEvent, LiveSource } from "./registry";

// Explicit typed bindings to generated metadata documents. No copied operations
// or payload types. Only dependencies of mounted consumers open these streams.
const entityDocuments = {
  Album: G.EntityAlbumChangedDocument,
  AppLog: G.EntityAppLogChangedDocument,
  AppSetting: G.EntityAppSettingChangedDocument,
  Artist: G.EntityArtistChangedDocument,
  AudioStream: G.EntityAudioStreamChangedDocument,
  Audiobook: G.EntityAudiobookChangedDocument,
  CastDevice: G.EntityCastDeviceChangedDocument,
  CastSession: G.EntityCastSessionChangedDocument,
  CastSetting: G.EntityCastSettingChangedDocument,
  Chapter: G.EntityChapterChangedDocument,
  Collection: G.EntityCollectionChangedDocument,
  Episode: G.EntityEpisodeChangedDocument,
  InviteToken: G.EntityInviteTokenChangedDocument,
  LibraryScanIssue: G.EntityLibraryScanIssueChangedDocument,
  LibraryScanRun: G.EntityLibraryScanRunChangedDocument,
  Library: G.EntityLibraryChangedDocument,
  PersonalList: G.EntityPersonalListChangedDocument,
  ListEntry: G.EntityListEntryChangedDocument,
  SavedView: G.EntitySavedViewChangedDocument,
  HomeShelf: G.EntityHomeShelfChangedDocument,
  LibraryPreference: G.EntityLibraryPreferenceChangedDocument,
  MediaChapter: G.EntityMediaChapterChangedDocument,
  MediaFile: G.EntityMediaFileChangedDocument,
  MetadataCache: G.EntityMetadataCacheChangedDocument,
  MovieCastCredit: G.EntityMovieCastCreditChangedDocument,
  Movie: G.EntityMovieChangedDocument,
  NamingPattern: G.EntityNamingPatternChangedDocument,
  Notification: G.EntityNotificationChangedDocument,
  PendingFileMatch: G.EntityPendingFileMatchChangedDocument,
  Person: G.EntityPersonChangedDocument,
  PlaybackProgress: G.EntityPlaybackProgressChangedDocument,
  PlaybackSession: G.EntityPlaybackSessionChangedDocument,
  QualityProfile: G.EntityQualityProfileChangedDocument,
  RefreshToken: G.EntityRefreshTokenChangedDocument,
  ReleaseBlocklist: G.EntityReleaseBlocklistChangedDocument,
  RssFeedItem: G.EntityRssFeedItemChangedDocument,
  RssFeed: G.EntityRssFeedChangedDocument,
  ScheduleCache: G.EntityScheduleCacheChangedDocument,
  ScheduleSyncState: G.EntityScheduleSyncStateChangedDocument,
  Show: G.EntityShowChangedDocument,
  SourcePriorityRule: G.EntitySourcePriorityRuleChangedDocument,
  Source: G.EntitySourceChangedDocument,
  Subtitle: G.EntitySubtitleChangedDocument,
  Torrent: G.EntityTorrentChangedDocument,
  TorrentFile: G.EntityTorrentFileChangedDocument,
  TorznabCategory: G.EntityTorznabCategoryChangedDocument,
  Track: G.EntityTrackChangedDocument,
  UsenetDownload: G.EntityUsenetDownloadChangedDocument,
  UsenetServer: G.EntityUsenetServerChangedDocument,
  User: G.EntityUserChangedDocument,
  VideoStream: G.EntityVideoStreamChangedDocument,
} satisfies Record<string, DocumentNode>;
export type EntityTopic = keyof typeof entityDocuments;
export type LiveTopic = EntityTopic | "TorrentProgress" | "TorrentAdded" | "TorrentCompleted" | "TorrentRemoved" | "EnabledSources" | "Backup" | "Filesystem";

function isDocument(value: unknown): value is DocumentNode {
  return Boolean(value && typeof value === "object" && "kind" in value && value.kind === Kind.DOCUMENT);
}
// Read/keyset discovery uses actual generated exports; every subscription
// below is required and checked against the generated module at compile time.
const exports = new Map<string, unknown>(Object.entries(G));
export const topicDocuments: Record<LiveTopic, DocumentNode> = {
  ...entityDocuments,
  EnabledSources: G.EnabledSourcesChangedDocument,
  Backup: G.BackupChangedDocument,
  Filesystem: G.FilesystemChangedDocument,
  TorrentProgress: G.TorrentProgressDocument,
  TorrentAdded: G.TorrentAddedDocument,
  TorrentCompleted: G.TorrentCompletedDocument,
  TorrentRemoved: G.TorrentRemovedDocument,
};

const fieldTopics = new Map<string, EntityTopic>();
for (const topic of Object.keys(entityDocuments) as EntityTopic[]) {
  for (const suffix of ["List", "Get", "Keyset"]) {
    const document = exports.get(`Entity${topic}${suffix}Document`);
    if (!isDocument(document)) continue;
    for (const definition of document.definitions) if (definition.kind === Kind.OPERATION_DEFINITION) {
      for (const field of definition.selectionSet.selections) if (field.kind === Kind.FIELD) fieldTopics.set(field.name.value, topic);
    }
  }
}

const statusTopics: readonly EntityTopic[] = ["Movie", "Episode", "Track", "Chapter", "MediaFile", "Library", "Torrent", "UsenetDownload", "PendingFileMatch", "QualityProfile", "LibraryScanIssue", "PlaybackProgress", "PlaybackSession"];
const runtimeTopics: readonly LiveTopic[] = ["Torrent", "TorrentProgress", "TorrentAdded", "TorrentCompleted", "TorrentRemoved"];
const extraRoots: Record<string, readonly LiveTopic[]> = {
  contentStatuses: statusTopics,
  // Member-safe existence projection. Never fall back to Source configuration
  // subscriptions: those expose a different, admin-only authorization surface.
  hasEnabledSources: ["EnabledSources"],
  resumeShelf: ["PlaybackProgress", "PlaybackSession", "MediaFile", "Movie", "Show", "Episode", "Album", "Artist", "Track", "Audiobook", "Chapter", "Library"],
  liveTorrents: runtimeTopics, liveTorrent: runtimeTopics, activeDownloadCount: runtimeTopics,
  movieCollectionDetails: ["Collection", "Movie", "MediaFile"],
  backupCapabilities: ["Backup"], backupSnapshots: ["Backup"],
  availableSourceDefinitions: ["Source"], sourceSettingDefinitions: ["Source"],
  filesystemRoots: ["Filesystem", "AppSetting"], browseDirectory: ["Filesystem", "AppSetting"],
  libraryPathAvailability: ["Filesystem", "AppSetting"],
};
const directDependencies: Partial<Record<EntityTopic, readonly EntityTopic[]>> = {
  // Retain direct children as well as parent streams until propagation is proven.
  Movie: ["MediaFile", "MovieCastCredit"], Show: ["Episode", "MediaFile"],
  Episode: ["Show", "MediaFile"], Album: ["Track", "MediaFile"],
  Artist: ["Album", "Track", "MediaFile"], Track: ["Album", "Artist", "MediaFile"],
  Audiobook: ["Chapter", "MediaFile"], Chapter: ["Audiobook", "MediaFile"],
  Collection: ["Movie", "MediaFile"],
  MediaFile: ["VideoStream", "AudioStream", "Subtitle", "MediaChapter"],
  PlaybackProgress: ["MediaFile", "Movie", "Episode", "Track", "Chapter"],
  LibraryScanRun: ["LibraryScanIssue"], LibraryScanIssue: ["LibraryScanRun"],
};

export function operationKey(document: DocumentNode): string {
  return document.definitions.filter((definition) => definition.kind === Kind.OPERATION_DEFINITION).map((definition) => definition.name?.value ?? "anonymous").join(",");
}

/** Derive root/nested/fragment dependencies even when a view is currently empty.
 * Membership, sort/filter transitions and counts require broad entity topics;
 * only explicit single-ID roots can safely narrow their direct dependency. */
export function queryDependencies(document: DocumentNode, variables: Record<string, unknown> = {}): LiveDependency[] {
  const topics = new Map<LiveTopic, readonly string[] | undefined>();
  const add = (topic: LiveTopic, ids?: readonly string[]) => {
    if (!topics.has(topic)) topics.set(topic, ids);
    else if (!topics.get(topic) || !ids) topics.set(topic, undefined);
    else topics.set(topic, [...new Set([...topics.get(topic)!, ...ids])]);
  };
  const fragments = new Map(document.definitions.filter((definition) => definition.kind === Kind.FRAGMENT_DEFINITION).map((definition) => [definition.name.value, definition.selectionSet]));
  const walk = (selection: SelectionSetNode, root: boolean, seen = new Set<string>()) => {
    for (const field of selection.selections) {
      if (field.kind === Kind.FRAGMENT_SPREAD) {
        const name = field.name.value;
        if (!seen.has(name) && fragments.has(name)) walk(fragments.get(name)!, false, new Set([...seen, name]));
      } else if (field.kind === Kind.INLINE_FRAGMENT) walk(field.selectionSet, false, seen);
      else {
        const topic = fieldTopics.get(field.name.value);
        if (topic) {
          const id = root ? field.arguments?.find((argument) => argument.name.value === "id")?.value : undefined;
          const value = id?.kind === Kind.VARIABLE ? variables[id.name.value] : id?.kind === Kind.STRING ? id.value : undefined;
          add(topic, typeof value === "string" ? [value] : undefined);
          if (root) for (const dependency of directDependencies[topic] ?? []) add(dependency);
        }
        if (root) for (const dependency of extraRoots[field.name.value] ?? []) add(dependency);
        if (/^(movieCount|showCount|albumCount|artistCount|audiobookCount|trackCount|totalSizeBytes|totalFiles)$/.test(field.name.value)) {
          for (const dependency of ["Movie", "Show", "Album", "Artist", "Audiobook", "Track", "MediaFile"] as const) add(dependency);
        }
        if (field.selectionSet) walk(field.selectionSet, false, seen);
      }
    }
  };
  for (const definition of document.definitions) if (definition.kind === Kind.OPERATION_DEFINITION) walk(definition.selectionSet, true);
  return [...topics].map(([topic, ids]) => ({ topic, ids }));
}

const adminTopics = new Set<LiveTopic>(["AppLog", "AppSetting", "User", "InviteToken", "RefreshToken", "Source", "UsenetServer", "MetadataCache", "ScheduleSyncState", "Backup", "Filesystem"]);

export function apolloLiveSource(client: ApolloClient, isAdmin: boolean): LiveSource {
  const states = new Map<string, string>();
  return (topic, emit, interrupted) => {
    const failed = (error: unknown) => {
      states.clear();
      if (errorCodes(error as never).includes("LIVE_RESYNC_REQUIRED")) {
        // A healthy socket can still have missed a bounded ORM broadcast.
        // Resnapshot the affected topic and reattach through registry backoff.
        interrupted();
        return;
      }
      if (isUnauthorizedError(error as never)) {
        void session.refresh().then(() => session.reconcile()).catch(() => {}).finally(() => interrupted());
      } else interrupted(!isForbiddenError(error as never));
    };
    if (adminTopics.has(topic) && !isAdmin) return () => {};
    if (topic === "TorrentProgress") {
      const subscription = client.subscribe({ query: G.TorrentProgressDocument }).subscribe({
        next: ({ data }) => {
          const update = data?.torrentProgress;
          if (!update) return;
          const id = client.cache.identify({ __typename: "LiveTorrent", infoHash: update.infoHash });
          if (id) client.cache.modify({
            id,
            fields: {
              progress: () => update.progress, downloadSpeed: () => update.downloadSpeed,
              uploadSpeed: () => update.uploadSpeed, peers: () => update.peers, state: () => update.state,
            },
          });
          // Count invalidation follows runtime state transitions, including
          // pause/resume. Progress bytes normalize without a query each second.
          if (states.get(update.infoHash) !== update.state) {
            states.set(update.infoHash, update.state);
            emit({ topic, id: String(update.id) });
          }
        },
        error: failed, complete: () => interrupted(),
      });
      return () => subscription.unsubscribe();
    }
    const document = topicDocuments[topic];
    const subscription = client.subscribe<Record<string, unknown>>({ query: document }).subscribe({
      next: ({ data }) => {
        // Boolean false is a real capability update (including the initial
        // snapshot), so it must invalidate the query just like true.
        for (const payload of Object.values(data ?? {})) {
          if (topic === "TorrentRemoved" && payload && typeof payload === "object" && "infoHash" in payload && typeof payload.infoHash === "string") states.delete(payload.infoHash);
          const id = payload && typeof payload === "object" && "id" in payload ? String(payload.id) : undefined;
          const event: LiveEvent = { topic, id };
          emit(event);
        }
      }, error: failed, complete: () => interrupted(),
    });
    return () => subscription.unsubscribe();
  };
}
