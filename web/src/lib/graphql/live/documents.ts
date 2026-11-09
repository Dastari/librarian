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
  Credit: G.EntityCreditChangedDocument,
  Device: G.EntityDeviceChangedDocument,
  Episode: G.EntityEpisodeChangedDocument,
  IdentificationEvent: G.EntityIdentificationEventChangedDocument,
  IdentifiedItem: G.EntityIdentifiedItemChangedDocument,
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
  NetworkServer: G.EntityNetworkServerChangedDocument,
  Notification: G.EntityNotificationChangedDocument,
  PendingFileMatch: G.EntityPendingFileMatchChangedDocument,
  Person: G.EntityPersonChangedDocument,
  PlaybackProgress: G.EntityPlaybackProgressChangedDocument,
  PlaybackSession: G.EntityPlaybackSessionChangedDocument,
  PlayHistory: G.EntityPlayHistoryChangedDocument,
  QualityProfile: G.EntityQualityProfileChangedDocument,
  RefreshToken: G.EntityRefreshTokenChangedDocument,
  ReleaseBlocklist: G.EntityReleaseBlocklistChangedDocument,
  ReleaseRecommendation: G.EntityReleaseRecommendationChangedDocument,
  RssFeedItem: G.EntityRssFeedItemChangedDocument,
  RssFeed: G.EntityRssFeedChangedDocument,
  ScheduleCache: G.EntityScheduleCacheChangedDocument,
  ScheduleSyncState: G.EntityScheduleSyncStateChangedDocument,
  Season: G.EntitySeasonChangedDocument,
  Show: G.EntityShowChangedDocument,
  SourcePriorityRule: G.EntitySourcePriorityRuleChangedDocument,
  StatsShare: G.EntityStatsShareChangedDocument,
  Source: G.EntitySourceChangedDocument,
  Subtitle: G.EntitySubtitleChangedDocument,
  TitleDetail: G.EntityTitleDetailChangedDocument,
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
  liveTorrents: runtimeTopics, liveTorrent: runtimeTopics,
  // Stored progress changes every few seconds, which is also when more files finish.
  torrentMatchPreview: ["Torrent", "TorrentAdded", "TorrentCompleted", "TorrentRemoved"],
  movieCollectionDetails: ["Collection", "Movie", "MediaFile"],
  backupCapabilities: ["Backup"], backupSnapshots: ["Backup"],
  availableSourceDefinitions: ["Source"], sourceSettingDefinitions: ["Source"],
  filesystemRoots: ["Filesystem", "AppSetting"], browseDirectory: ["Filesystem", "AppSetting"],
  libraryPathAvailability: ["Filesystem", "AppSetting"],
  // Computed over plays; a share given or taken back changes whose can be read.
  playStatistics: ["PlayHistory", "StatsShare"],
  statisticsPeople: ["StatsShare", "User"],
};
// A root query does not also watch its children, or anything that reaches it by propagation:
// each has `propagate_change = "up"` for creates and updates, and
// `maintain_summaries_and_parent_events` writes the surviving parent when the child is deleted or
// moved (propagation is suppressed for a deleted source and reads only the post-update row).
//   - MovieCastCredit, Episode, Track, Chapter, VideoStream, AudioStream, Subtitle, MediaChapter
//     and LibraryScanIssue reach their parents;
//   - MediaFile reaches the Movie, Episode, Track or Chapter it is linked to, and through them
//     the Show, Album, Artist, Audiobook and Collection above it;
//   - Album and Track reach their Artist, and Movie reaches its Collection.
// The entries below are the rest: a parent's own page reads the data of the entity named, and a
// parent never propagates down.
const directDependencies: Partial<Record<EntityTopic, readonly EntityTopic[]>> = {
  Episode: ["Show"],
  Track: ["Album", "Artist"],
  Chapter: ["Audiobook"],
  PlaybackProgress: ["MediaFile", "Movie", "Episode", "Track", "Chapter"],
  LibraryScanIssue: ["LibraryScanRun"],
};

export function operationKey(document: DocumentNode): string {
  return document.definitions.filter((definition) => definition.kind === Kind.OPERATION_DEFINITION).map((definition) => definition.name?.value ?? "anonymous").join(",");
}

const COMPUTED_COUNTS = /^(movieCount|showCount|albumCount|artistCount|audiobookCount|trackCount|totalSizeBytes|totalFiles)$/;
const CONNECTION_FIELDS = new Set(["edges", "node", "pageInfo"]);

/** True when a selection reads only the record's own persisted scalars (through connection
 * wrappers), so events propagated up from its children cannot change the result. */
function readsOwnScalars(selection: SelectionSetNode, fragments: ReadonlyMap<string, SelectionSetNode>, seen = new Set<string>()): boolean {
  return selection.selections.every((field) => {
    if (field.kind === Kind.INLINE_FRAGMENT) return readsOwnScalars(field.selectionSet, fragments, seen);
    if (field.kind === Kind.FRAGMENT_SPREAD) {
      const name = field.name.value;
      const target = fragments.get(name);
      return seen.has(name) || (target !== undefined && readsOwnScalars(target, fragments, new Set([...seen, name])));
    }
    if (COMPUTED_COUNTS.test(field.name.value)) return false;
    return !field.selectionSet || (CONNECTION_FIELDS.has(field.name.value) && readsOwnScalars(field.selectionSet, fragments, seen));
  });
}

/** Derive root/nested/fragment dependencies even when a view is currently empty.
 * Membership, sort/filter transitions and counts require broad entity topics;
 * only explicit single-ID roots can safely narrow their direct dependency. A dependency is
 * `direct` when every read of the topic is of its own scalars only. */
export function queryDependencies(document: DocumentNode, variables: Record<string, unknown> = {}): LiveDependency[] {
  const topics = new Map<LiveTopic, { ids?: readonly string[]; direct: boolean }>();
  const add = (topic: LiveTopic, ids?: readonly string[], direct = false) => {
    const current = topics.get(topic);
    if (!current) topics.set(topic, { ids, direct });
    else topics.set(topic, { ids: current.ids && ids ? [...new Set([...current.ids, ...ids])] : undefined, direct: current.direct && direct });
  };
  const fragments = new Map(document.definitions.filter((definition) => definition.kind === Kind.FRAGMENT_DEFINITION).map((definition) => [definition.name.value, definition.selectionSet]));
  const walk = (selection: SelectionSetNode, root: boolean, seen = new Set<string>()) => {
    for (const field of selection.selections) {
      if (field.kind === Kind.FRAGMENT_SPREAD) {
        const name = field.name.value;
        if (!seen.has(name) && fragments.has(name)) walk(fragments.get(name)!, false, new Set([...seen, name]));
      } else if (field.kind === Kind.INLINE_FRAGMENT) walk(field.selectionSet, false, seen);
      else {
        // A scalar that shares a root's name (`season` on Episode) is not that entity.
        const topic = field.selectionSet ? fieldTopics.get(field.name.value) : undefined;
        if (topic) {
          const id = root ? field.arguments?.find((argument) => argument.name.value === "id")?.value : undefined;
          const value = id?.kind === Kind.VARIABLE ? variables[id.name.value] : id?.kind === Kind.STRING ? id.value : undefined;
          add(topic, typeof value === "string" ? [value] : undefined, readsOwnScalars(field.selectionSet!, fragments));
          if (root) for (const dependency of directDependencies[topic] ?? []) add(dependency);
        }
        if (root) for (const dependency of extraRoots[field.name.value] ?? []) add(dependency);
        if (COMPUTED_COUNTS.test(field.name.value)) {
          for (const dependency of ["Movie", "Show", "Album", "Artist", "Audiobook", "Track", "MediaFile"] as const) add(dependency);
        }
        if (field.selectionSet) walk(field.selectionSet, false, seen);
      }
    }
  };
  for (const definition of document.definitions) if (definition.kind === Kind.OPERATION_DEFINITION) walk(definition.selectionSet, true);
  return [...topics].map(([topic, { ids, direct }]) => ({ topic, ids, ...(direct ? { direct } : {}) }));
}

/** How a generated `Changed` stream says it ended without delivering everything. */
const CONTINUITY_ERRORS = new Set(["SUBSCRIPTION_CONTINUITY_LOST", "SUBSCRIPTION_SOURCE_CLOSED"]);

const adminTopics = new Set<LiveTopic>(["AppLog", "AppSetting", "User", "InviteToken", "RefreshToken", "Source", "UsenetServer", "NetworkServer", "MetadataCache", "ScheduleSyncState", "Backup", "Filesystem"]);

export function apolloLiveSource(client: ApolloClient, isAdmin: boolean): LiveSource {
  const states = new Map<string, string>();
  return (topic, emit, interrupted, keys) => {
    // Apollo delivers every subscription failure through `next`, including transport ones.
    const failed = (error: unknown) => {
      states.clear();
      if (errorCodes(error as never).some((code) => CONTINUITY_ERRORS.has(code))) {
        // A healthy socket can still have missed a bounded ORM broadcast. The stream
        // ends here: resnapshot the affected topic and reattach through registry backoff.
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
        next: ({ data, error }) => {
          if (error) { failed(error); return; }
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
    const variables = keys && topic in entityDocuments ? { keys: [...keys] } : undefined;
    const subscription = client.subscribe<Record<string, unknown>>({ query: document, variables }).subscribe({
      next: ({ data, error }) => {
        if (error) { failed(error); return; }
        // Boolean false is a real capability update (including the initial
        // snapshot), so it must invalidate the query just like true.
        for (const payload of Object.values(data ?? {})) {
          if (topic === "TorrentRemoved" && payload && typeof payload === "object" && "infoHash" in payload && typeof payload.infoHash === "string") states.delete(payload.infoHash);
          const id = payload && typeof payload === "object" && "id" in payload ? String(payload.id) : undefined;
          const propagated = Boolean(payload && typeof payload === "object" && "changeKind" in payload && payload.changeKind === "PROPAGATED");
          const event: LiveEvent = { topic, id, ...(propagated ? { propagated } : {}) };
          emit(event);
        }
      }, error: failed, complete: () => interrupted(),
    });
    return () => subscription.unsubscribe();
  };
}
