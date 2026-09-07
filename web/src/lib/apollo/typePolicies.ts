import type { TypePolicies } from "@apollo/client";

/**
 * Cache normalisation. Every entity carries `id`; connections are keyed by their filter
 * arguments so distinct pages never overwrite each other. Singleton-like objects (`me`)
 * merge into one record.
 */
export const typePolicies: TypePolicies = {
  Query: {
    fields: {
      me: { merge: true },
    },
  },
  MeUser: { keyFields: ["id"] },
  AuthenticatedUser: { keyFields: ["id"] },
  LiveTorrent: { keyFields: ["infoHash"] },
  TorrentProgress: { keyFields: ["infoHash"] },
  LegacyCastDevice: { keyFields: ["id"] },
  LegacyCastSession: { keyFields: ["id"] },
  BackupSnapshotSummary: { keyFields: ["snapshotId"] },
  SourceDefinitionInfo: { keyFields: ["id"] },
  MovieSearchResult: { keyFields: ["provider", "providerId"] },
  TvShowSearchResult: { keyFields: ["provider", "providerId"] },
  AlbumSearchResult: { keyFields: ["provider", "providerId"] },
  AudiobookSearchResult: { keyFields: ["provider", "providerId"] },
  MovieCollectionSearchResult: { keyFields: ["provider", "collectionId"] },
  SourceReleaseInfo: { keyFields: ["guid"] },
  PageInfo: { keyFields: false },
  ContentStatusResult: { keyFields: ["contentType", "id"] },
};
