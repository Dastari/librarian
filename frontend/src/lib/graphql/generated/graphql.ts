/** Internal type. DO NOT USE DIRECTLY. */
type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
/** Internal type. DO NOT USE DIRECTLY. */
export type Incremental<T> =
  | T
  | {
      [P in keyof T]?: P extends " $fragmentName" | "__typename" ? T[P] : never;
    };
import type { TypedDocumentNode as DocumentNode } from "@apollo/client";
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string };
  String: { input: string; output: string };
  Boolean: { input: boolean; output: boolean };
  Int: { input: number; output: number };
  Float: { input: number; output: number };
  JSON: { input: Record<string, unknown>; output: Record<string, unknown> };
};

export type AddAlbumInput = {
  LibraryId: Scalars["String"]["input"];
  MusicbrainzId: Scalars["String"]["input"];
};

export type AddAudiobookInput = {
  LibraryId: Scalars["String"]["input"];
  OpenlibraryId: Scalars["String"]["input"];
};

/** Input for adding/importing a movie collection from TMDB */
export type AddMovieCollectionInput = {
  /** TMDB collection ID */
  CollectionId: Scalars["Int"]["input"];
  /** Mark missing imported movies as wanted */
  WantedMissing?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Input for adding a movie from TMDB */
export type AddMovieInput = {
  /** Whether to monitor for releases (enables auto-download) */
  Monitored?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** TMDB movie ID */
  TmdbId: Scalars["Int"]["input"];
};

/** Input for adding a torrent */
export type AddTorrentInput = {
  Magnet?: InputMaybe<Scalars["String"]["input"]>;
  Url?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of add torrent mutation */
export type AddTorrentResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  Torrent?: Maybe<LiveTorrent>;
};

/** Input for adding a TV show from TVMaze */
export type AddTvShowInput = {
  /** Auto-download mode for episodes */
  AutoDownloadMode?: InputMaybe<AutoDownloadMode>;
  /** Optional path override for the show */
  Path?: InputMaybe<Scalars["String"]["input"]>;
  /** TVMaze show ID */
  TvmazeId: Scalars["Int"]["input"];
};

/** Album Entity */
export type Album = {
  AlbumType?: Maybe<Scalars["String"]["output"]>;
  ArtistId: Scalars["String"]["output"];
  AutoDownload: Scalars["Boolean"]["output"];
  AutoDownloadMode: AutoDownloadMode;
  Country?: Maybe<Scalars["String"]["output"]>;
  CoverUrl?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  DiscCount?: Maybe<Scalars["Int"]["output"]>;
  Genres: Array<Scalars["String"]["output"]>;
  HasFiles: Scalars["Boolean"]["output"];
  Id: Scalars["String"]["output"];
  Label?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  MusicbrainzId?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  Path?: Maybe<Scalars["String"]["output"]>;
  ReleaseDate?: Maybe<Scalars["String"]["output"]>;
  SizeBytes?: Maybe<Scalars["Int"]["output"]>;
  SortName?: Maybe<Scalars["String"]["output"]>;
  TotalDurationSecs?: Maybe<Scalars["Int"]["output"]>;
  TrackCount?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  Year?: Maybe<Scalars["Int"]["output"]>;
  /** Get related #graphql_name */
  library?: Maybe<Library>;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  tracks: TrackConnection;
};

/** Album Entity */
export type AlbumtracksArgs = {
  orderBy?: InputMaybe<TrackOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<TrackWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type AlbumChangedEvent = {
  action: ChangeAction;
  album?: Maybe<Album>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type AlbumConnection = {
  /** The edges in this connection */
  edges: Array<AlbumEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AlbumEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Album;
};

export type AlbumOperationResult = {
  Album?: Maybe<Album>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type AlbumOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  ReleaseDate?: InputMaybe<OrderDirection>;
  SizeBytes?: InputMaybe<OrderDirection>;
  SortName?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
  Year?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type AlbumResult = {
  album?: Maybe<Album>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Search result for MusicBrainz album search. */
export type AlbumSearchResult = {
  AlbumType?: Maybe<Scalars["String"]["output"]>;
  ArtistName?: Maybe<Scalars["String"]["output"]>;
  CoverUrl?: Maybe<Scalars["String"]["output"]>;
  Provider: Scalars["String"]["output"];
  ProviderId: Scalars["String"]["output"];
  Score?: Maybe<Scalars["Float"]["output"]>;
  Title: Scalars["String"]["output"];
  Year?: Maybe<Scalars["Int"]["output"]>;
};

export type AlbumWhereInput = {
  AlbumType?: InputMaybe<StringFilter>;
  ArtistId?: InputMaybe<StringFilter>;
  AutoDownload?: InputMaybe<BoolFilter>;
  Country?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DiscCount?: InputMaybe<IntFilter>;
  HasFiles?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  Label?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MusicbrainzId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  ReleaseDate?: InputMaybe<DateFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  TotalDurationSecs?: InputMaybe<IntFilter>;
  TrackCount?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Year?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<AlbumWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<AlbumWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<AlbumWhereInput>>;
};

export type AnalyzeMediaFileResult = {
  Message?: Maybe<Scalars["String"]["output"]>;
  Queued: Scalars["Boolean"]["output"];
  Success: Scalars["Boolean"]["output"];
};

export type AppLog = {
  CreatedAt: Scalars["String"]["output"];
  Fields?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  Level: Scalars["String"]["output"];
  Message: Scalars["String"]["output"];
  SpanId?: Maybe<Scalars["String"]["output"]>;
  SpanName?: Maybe<Scalars["String"]["output"]>;
  Target: Scalars["String"]["output"];
  Timestamp: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type AppLogChangedEvent = {
  action: ChangeAction;
  appLog?: Maybe<AppLog>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type AppLogConnection = {
  /** The edges in this connection */
  edges: Array<AppLogEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AppLogEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: AppLog;
};

export type AppLogOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  Level?: InputMaybe<OrderDirection>;
  Target?: InputMaybe<OrderDirection>;
  Timestamp?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type AppLogResult = {
  appLog?: Maybe<AppLog>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type AppLogWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  Level?: InputMaybe<StringFilter>;
  Message?: InputMaybe<StringFilter>;
  SpanId?: InputMaybe<StringFilter>;
  SpanName?: InputMaybe<StringFilter>;
  Target?: InputMaybe<StringFilter>;
  Timestamp?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<AppLogWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<AppLogWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<AppLogWhereInput>>;
};

export type AppSetting = {
  Category: Scalars["String"]["output"];
  CreatedAt: Scalars["String"]["output"];
  Description?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  Key: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  Value: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type AppSettingChangedEvent = {
  action: ChangeAction;
  appSetting?: Maybe<AppSetting>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type AppSettingConnection = {
  /** The edges in this connection */
  edges: Array<AppSettingEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AppSettingEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: AppSetting;
};

export type AppSettingOrderByInput = {
  Category?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  Key?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type AppSettingResult = {
  appSetting?: Maybe<AppSetting>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type AppSettingWhereInput = {
  Category?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  Key?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<AppSettingWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<AppSettingWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<AppSettingWhereInput>>;
};

export type Artist = {
  AlbumCount?: Maybe<Scalars["Int"]["output"]>;
  Bio?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Disambiguation?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  ImageUrl?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  MusicbrainzId?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  SortName?: Maybe<Scalars["String"]["output"]>;
  TotalDurationSecs?: Maybe<Scalars["Int"]["output"]>;
  TrackCount?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  albums: AlbumConnection;
};

export type ArtistalbumsArgs = {
  orderBy?: InputMaybe<AlbumOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AlbumWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type ArtistChangedEvent = {
  action: ChangeAction;
  artist?: Maybe<Artist>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type ArtistConnection = {
  /** The edges in this connection */
  edges: Array<ArtistEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ArtistEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Artist;
};

export type ArtistOrderByInput = {
  AlbumCount?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  SortName?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type ArtistResult = {
  artist?: Maybe<Artist>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type ArtistWhereInput = {
  AlbumCount?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MusicbrainzId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  TotalDurationSecs?: InputMaybe<IntFilter>;
  TrackCount?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<ArtistWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<ArtistWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<ArtistWhereInput>>;
};

export type ArtworkCache = {
  ArtworkType: Scalars["String"]["output"];
  ContentHash: Scalars["String"]["output"];
  CreatedAt: Scalars["String"]["output"];
  EntityId: Scalars["String"]["output"];
  EntityType: Scalars["String"]["output"];
  Height?: Maybe<Scalars["Int"]["output"]>;
  Id: Scalars["String"]["output"];
  MimeType: Scalars["String"]["output"];
  SizeBytes: Scalars["Int"]["output"];
  SourceUrl?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  Width?: Maybe<Scalars["Int"]["output"]>;
  data: Array<Scalars["Int"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type ArtworkCacheChangedEvent = {
  action: ChangeAction;
  artworkCache?: Maybe<ArtworkCache>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type ArtworkCacheConnection = {
  /** The edges in this connection */
  edges: Array<ArtworkCacheEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ArtworkCacheEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: ArtworkCache;
};

export type ArtworkCacheOrderByInput = {
  ArtworkType?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  EntityType?: InputMaybe<OrderDirection>;
  SizeBytes?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type ArtworkCacheResult = {
  artworkCache?: Maybe<ArtworkCache>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type ArtworkCacheWhereInput = {
  ArtworkType?: InputMaybe<StringFilter>;
  ContentHash?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EntityId?: InputMaybe<StringFilter>;
  EntityType?: InputMaybe<StringFilter>;
  Height?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MimeType?: InputMaybe<StringFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Width?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<ArtworkCacheWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<ArtworkCacheWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<ArtworkCacheWhereInput>>;
};

export type AudioStream = {
  BitDepth?: Maybe<Scalars["Int"]["output"]>;
  Bitrate?: Maybe<Scalars["Int"]["output"]>;
  ChannelLayout?: Maybe<Scalars["String"]["output"]>;
  Channels: Scalars["Int"]["output"];
  Codec: Scalars["String"]["output"];
  CodecLongName?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  IsCommentary: Scalars["Boolean"]["output"];
  IsDefault: Scalars["Boolean"]["output"];
  Language?: Maybe<Scalars["String"]["output"]>;
  MediaFileId: Scalars["String"]["output"];
  Metadata?: Maybe<Scalars["String"]["output"]>;
  SampleRate?: Maybe<Scalars["Int"]["output"]>;
  StreamIndex: Scalars["Int"]["output"];
  Title?: Maybe<Scalars["String"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type AudioStreamChangedEvent = {
  action: ChangeAction;
  audioStream?: Maybe<AudioStream>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type AudioStreamConnection = {
  /** The edges in this connection */
  edges: Array<AudioStreamEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AudioStreamEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: AudioStream;
};

export type AudioStreamOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  StreamIndex?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type AudioStreamResult = {
  audioStream?: Maybe<AudioStream>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type AudioStreamWhereInput = {
  BitDepth?: InputMaybe<IntFilter>;
  Bitrate?: InputMaybe<IntFilter>;
  Channels?: InputMaybe<IntFilter>;
  Codec?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IsCommentary?: InputMaybe<BoolFilter>;
  IsDefault?: InputMaybe<BoolFilter>;
  Language?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  SampleRate?: InputMaybe<IntFilter>;
  StreamIndex?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<AudioStreamWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<AudioStreamWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<AudioStreamWhereInput>>;
};

export type Audiobook = {
  Asin?: Maybe<Scalars["String"]["output"]>;
  AudibleId?: Maybe<Scalars["String"]["output"]>;
  AuthorName?: Maybe<Scalars["String"]["output"]>;
  AutoDownload: Scalars["Boolean"]["output"];
  AutoDownloadMode: AutoDownloadMode;
  ChapterCount?: Maybe<Scalars["Int"]["output"]>;
  CoverUrl?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Description?: Maybe<Scalars["String"]["output"]>;
  GoodreadsId?: Maybe<Scalars["String"]["output"]>;
  HasFiles: Scalars["Boolean"]["output"];
  Id: Scalars["String"]["output"];
  Isbn?: Maybe<Scalars["String"]["output"]>;
  Language?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  NarratorName?: Maybe<Scalars["String"]["output"]>;
  Narrators: Array<Scalars["String"]["output"]>;
  Path?: Maybe<Scalars["String"]["output"]>;
  PublishedDate?: Maybe<Scalars["String"]["output"]>;
  Publisher?: Maybe<Scalars["String"]["output"]>;
  SizeBytes?: Maybe<Scalars["Int"]["output"]>;
  SortTitle?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TotalDurationSecs?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  chapters: ChapterConnection;
};

export type AudiobookchaptersArgs = {
  orderBy?: InputMaybe<ChapterOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ChapterWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type AudiobookChangedEvent = {
  action: ChangeAction;
  audiobook?: Maybe<Audiobook>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type AudiobookConnection = {
  /** The edges in this connection */
  edges: Array<AudiobookEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AudiobookEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Audiobook;
};

export type AudiobookOperationResult = {
  Audiobook?: Maybe<Audiobook>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type AudiobookOrderByInput = {
  AuthorName?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  PublishedDate?: InputMaybe<OrderDirection>;
  SizeBytes?: InputMaybe<OrderDirection>;
  SortTitle?: InputMaybe<OrderDirection>;
  Title?: InputMaybe<OrderDirection>;
  TotalDurationSecs?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type AudiobookResult = {
  audiobook?: Maybe<Audiobook>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Search result for OpenLibrary audiobook search. */
export type AudiobookSearchResult = {
  AuthorName?: Maybe<Scalars["String"]["output"]>;
  CoverUrl?: Maybe<Scalars["String"]["output"]>;
  Description?: Maybe<Scalars["String"]["output"]>;
  Isbn?: Maybe<Scalars["String"]["output"]>;
  Provider: Scalars["String"]["output"];
  ProviderId: Scalars["String"]["output"];
  Title: Scalars["String"]["output"];
  Year?: Maybe<Scalars["Int"]["output"]>;
};

export type AudiobookWhereInput = {
  Asin?: InputMaybe<StringFilter>;
  AudibleId?: InputMaybe<StringFilter>;
  AuthorName?: InputMaybe<StringFilter>;
  AutoDownload?: InputMaybe<BoolFilter>;
  ChapterCount?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  GoodreadsId?: InputMaybe<StringFilter>;
  HasFiles?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  Isbn?: InputMaybe<StringFilter>;
  Language?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  NarratorName?: InputMaybe<StringFilter>;
  PublishedDate?: InputMaybe<DateFilter>;
  Publisher?: InputMaybe<StringFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  Title?: InputMaybe<StringFilter>;
  TotalDurationSecs?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<AudiobookWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<AudiobookWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<AudiobookWhereInput>>;
};

export type AuthPayload = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  Tokens?: Maybe<AuthTokens>;
  User?: Maybe<AuthenticatedUser>;
};

export type AuthTokens = {
  accessToken: Scalars["String"]["output"];
  expiresIn: Scalars["Int"]["output"];
  refreshToken: Scalars["String"]["output"];
  tokenType: Scalars["String"]["output"];
};

export type AuthenticatedUser = {
  displayName?: Maybe<Scalars["String"]["output"]>;
  email?: Maybe<Scalars["String"]["output"]>;
  id: Scalars["String"]["output"];
  role: Scalars["String"]["output"];
  username: Scalars["String"]["output"];
};

/** Auto-download mode for media items */
export const AutoDownloadMode = {
  /** Auto-download all items */
  ALL: "ALL",
  /** Do not auto-download */
  NONE: "NONE",
  /** Auto-download only wanted items */
  WANTED: "WANTED",
} as const;

export type AutoDownloadMode =
  (typeof AutoDownloadMode)[keyof typeof AutoDownloadMode];
export type BoolFilter = {
  eq?: InputMaybe<Scalars["Boolean"]["input"]>;
  isNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  ne?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** A single file or directory entry (PascalCase for GraphQL). */
export type BrowseDirectoryEntry = {
  IsDir: Scalars["Boolean"]["output"];
  MimeType?: Maybe<Scalars["String"]["output"]>;
  ModifiedAt?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  Path: Scalars["String"]["output"];
  Readable: Scalars["Boolean"]["output"];
  Size: Scalars["Int"]["output"];
  SizeFormatted: Scalars["String"]["output"];
  Writable: Scalars["Boolean"]["output"];
};

/** Input for the BrowseDirectory query (PascalCase for GraphQL). */
export type BrowseDirectoryInput = {
  /** Only show directories. */
  DirsOnly: Scalars["Boolean"]["input"];
  /** Path to browse (defaults to root or home). */
  Path?: InputMaybe<Scalars["String"]["input"]>;
  /** Include hidden entries (files/dirs starting with .). */
  ShowHidden: Scalars["Boolean"]["input"];
};

/** Result of browsing a directory (PascalCase for GraphQL). */
export type BrowseDirectoryResult = {
  CurrentPath: Scalars["String"]["output"];
  Entries: Array<BrowseDirectoryEntry>;
  IsLibraryPath: Scalars["Boolean"]["output"];
  LibraryId?: Maybe<Scalars["String"]["output"]>;
  ParentPath?: Maybe<Scalars["String"]["output"]>;
  QuickPaths: Array<BrowseQuickPath>;
};

/** Quick-access path shortcut (PascalCase for GraphQL). */
export type BrowseQuickPath = {
  Name: Scalars["String"]["output"];
  Path: Scalars["String"]["output"];
};

export type CastActionResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type CastDevice = {
  Address: Scalars["String"]["output"];
  CreatedAt: Scalars["String"]["output"];
  DeviceType: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  IsFavorite: Scalars["Boolean"]["output"];
  IsManual: Scalars["Boolean"]["output"];
  LastSeenAt?: Maybe<Scalars["String"]["output"]>;
  Model?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  Port: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type CastDeviceChangedEvent = {
  action: ChangeAction;
  castDevice?: Maybe<CastDevice>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type CastDeviceConnection = {
  /** The edges in this connection */
  edges: Array<CastDeviceEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type CastDeviceEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: CastDevice;
};

export type CastDeviceOperationResult = {
  device?: Maybe<LegacyCastDevice>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type CastDeviceOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  LastSeenAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type CastDeviceResult = {
  castDevice?: Maybe<CastDevice>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type CastDeviceWhereInput = {
  Address?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DeviceType?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  IsFavorite?: InputMaybe<BoolFilter>;
  IsManual?: InputMaybe<BoolFilter>;
  LastSeenAt?: InputMaybe<DateFilter>;
  Model?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  Port?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<CastDeviceWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<CastDeviceWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<CastDeviceWhereInput>>;
};

export type CastMediaInput = {
  deviceId: Scalars["String"]["input"];
  episodeId?: InputMaybe<Scalars["String"]["input"]>;
  mediaFileId: Scalars["String"]["input"];
  startPosition?: InputMaybe<Scalars["Float"]["input"]>;
};

export type CastSession = {
  CreatedAt: Scalars["String"]["output"];
  CurrentPosition: Scalars["Float"]["output"];
  DeviceId?: Maybe<Scalars["String"]["output"]>;
  Duration?: Maybe<Scalars["Float"]["output"]>;
  EndedAt?: Maybe<Scalars["String"]["output"]>;
  EpisodeId?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  IsMuted: Scalars["Boolean"]["output"];
  LastPosition?: Maybe<Scalars["Float"]["output"]>;
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  PlayerState: Scalars["String"]["output"];
  StartedAt: Scalars["String"]["output"];
  StreamUrl: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  Volume: Scalars["Float"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type CastSessionChangedEvent = {
  action: ChangeAction;
  castSession?: Maybe<CastSession>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type CastSessionConnection = {
  /** The edges in this connection */
  edges: Array<CastSessionEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type CastSessionEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: CastSession;
};

export type CastSessionOperationResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  session?: Maybe<LegacyCastSession>;
  success: Scalars["Boolean"]["output"];
};

export type CastSessionOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  StartedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type CastSessionResult = {
  castSession?: Maybe<CastSession>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type CastSessionWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  CurrentPosition?: InputMaybe<IntFilter>;
  DeviceId?: InputMaybe<StringFilter>;
  Duration?: InputMaybe<IntFilter>;
  EndedAt?: InputMaybe<DateFilter>;
  EpisodeId?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  IsMuted?: InputMaybe<BoolFilter>;
  LastPosition?: InputMaybe<IntFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  PlayerState?: InputMaybe<StringFilter>;
  StartedAt?: InputMaybe<DateFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Volume?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<CastSessionWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<CastSessionWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<CastSessionWhereInput>>;
};

export type CastSetting = {
  AutoDiscoveryEnabled: Scalars["Boolean"]["output"];
  CreatedAt: Scalars["String"]["output"];
  DefaultVolume: Scalars["Float"]["output"];
  DiscoveryIntervalSeconds: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  PreferredQuality?: Maybe<Scalars["String"]["output"]>;
  TranscodeIncompatible: Scalars["Boolean"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type CastSettingChangedEvent = {
  action: ChangeAction;
  castSetting?: Maybe<CastSetting>;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type CastSettingConnection = {
  /** The edges in this connection */
  edges: Array<CastSettingEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type CastSettingEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: CastSetting;
};

export type CastSettingOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type CastSettingResult = {
  castSetting?: Maybe<CastSetting>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type CastSettingWhereInput = {
  AutoDiscoveryEnabled?: InputMaybe<BoolFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DefaultVolume?: InputMaybe<IntFilter>;
  DiscoveryIntervalSeconds?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  PreferredQuality?: InputMaybe<StringFilter>;
  TranscodeIncompatible?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<CastSettingWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<CastSettingWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<CastSettingWhereInput>>;
};

export type CastSettingsOperationResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  settings?: Maybe<LegacyCastSettings>;
  success: Scalars["Boolean"]["output"];
};

export const ChangeAction = {
  CREATED: "CREATED",
  DELETED: "DELETED",
  UPDATED: "UPDATED",
} as const;

export type ChangeAction = (typeof ChangeAction)[keyof typeof ChangeAction];
export const ChangeKind = {
  DIRECT: "DIRECT",
  PROPAGATED: "PROPAGATED",
} as const;

export type ChangeKind = (typeof ChangeKind)[keyof typeof ChangeKind];
export type Chapter = {
  AudiobookId: Scalars["String"]["output"];
  ChapterNumber: Scalars["Int"]["output"];
  CreatedAt: Scalars["String"]["output"];
  DurationSecs?: Maybe<Scalars["Int"]["output"]>;
  EndTimeSecs?: Maybe<Scalars["Float"]["output"]>;
  Id: Scalars["String"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  StartTimeSecs: Scalars["Float"]["output"];
  Title?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  Wanted: Scalars["Boolean"]["output"];
  /** Get related #graphql_name */
  mediaFile?: Maybe<MediaFile>;
};

/** Event for #struct_name changes (subscriptions) */
export type ChapterChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  chapter?: Maybe<Chapter>;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type ChapterConnection = {
  /** The edges in this connection */
  edges: Array<ChapterEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ChapterEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Chapter;
};

export type ChapterOrderByInput = {
  ChapterNumber?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  Title?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type ChapterResult = {
  chapter?: Maybe<Chapter>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type ChapterWhereInput = {
  AudiobookId?: InputMaybe<StringFilter>;
  ChapterNumber?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DurationSecs?: InputMaybe<IntFilter>;
  EndTimeSecs?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  StartTimeSecs?: InputMaybe<IntFilter>;
  Title?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Wanted?: InputMaybe<BoolFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<ChapterWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<ChapterWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<ChapterWhereInput>>;
};

export type Collection = {
  BackdropUrl?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  LastSyncedAt?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  MovieCount: Scalars["Int"]["output"];
  Name: Scalars["String"]["output"];
  Overview?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  TmdbCollectionId: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  /** Get related #graphql_name */
  library?: Maybe<Library>;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  movies: MovieConnection;
};

export type CollectionmoviesArgs = {
  orderBy?: InputMaybe<MovieOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MovieWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type CollectionChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  collection?: Maybe<Collection>;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type CollectionConnection = {
  /** The edges in this connection */
  edges: Array<CollectionEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type CollectionEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Collection;
};

export type CollectionOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  LastSyncedAt?: InputMaybe<OrderDirection>;
  MovieCount?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type CollectionResult = {
  collection?: Maybe<Collection>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type CollectionWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastSyncedAt?: InputMaybe<DateFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MovieCount?: InputMaybe<IntFilter>;
  Name?: InputMaybe<StringFilter>;
  TmdbCollectionId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<CollectionWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<CollectionWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<CollectionWhereInput>>;
};

export type ConfigureNetworkPathInput = {
  AttemptConnect?: InputMaybe<Scalars["Boolean"]["input"]>;
  MountPoint?: InputMaybe<Scalars["String"]["input"]>;
  Password?: InputMaybe<Scalars["String"]["input"]>;
  Path: Scalars["String"]["input"];
  Persist?: InputMaybe<Scalars["Boolean"]["input"]>;
  Username?: InputMaybe<Scalars["String"]["input"]>;
};

export type CopyFilesInput = {
  Destination: Scalars["String"]["input"];
  Overwrite?: InputMaybe<Scalars["Boolean"]["input"]>;
  Sources: Array<Scalars["String"]["input"]>;
};

export type CreateAlbumInput = {
  AlbumType?: InputMaybe<Scalars["String"]["input"]>;
  ArtistId: Scalars["String"]["input"];
  AutoDownload: Scalars["Boolean"]["input"];
  AutoDownloadMode: AutoDownloadMode;
  Country?: InputMaybe<Scalars["String"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  DiscCount?: InputMaybe<Scalars["Int"]["input"]>;
  Genres: Scalars["JSON"]["input"];
  HasFiles: Scalars["Boolean"]["input"];
  Label?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  Path?: InputMaybe<Scalars["String"]["input"]>;
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  TrackCount?: InputMaybe<Scalars["Int"]["input"]>;
  UserId: Scalars["String"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

export type CreateAppLogInput = {
  Fields?: InputMaybe<Scalars["String"]["input"]>;
  Level: Scalars["String"]["input"];
  Message: Scalars["String"]["input"];
  SpanId?: InputMaybe<Scalars["String"]["input"]>;
  SpanName?: InputMaybe<Scalars["String"]["input"]>;
  Target: Scalars["String"]["input"];
  Timestamp: Scalars["String"]["input"];
};

export type CreateAppSettingInput = {
  Category: Scalars["String"]["input"];
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Key: Scalars["String"]["input"];
  Value: Scalars["String"]["input"];
};

export type CreateArtistInput = {
  AlbumCount?: InputMaybe<Scalars["Int"]["input"]>;
  Bio?: InputMaybe<Scalars["String"]["input"]>;
  Disambiguation?: InputMaybe<Scalars["String"]["input"]>;
  ImageUrl?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  TrackCount?: InputMaybe<Scalars["Int"]["input"]>;
  UserId: Scalars["String"]["input"];
};

export type CreateArtworkCacheInput = {
  ArtworkType: Scalars["String"]["input"];
  ContentHash: Scalars["String"]["input"];
  EntityId: Scalars["String"]["input"];
  EntityType: Scalars["String"]["input"];
  Height?: InputMaybe<Scalars["Int"]["input"]>;
  MimeType: Scalars["String"]["input"];
  SizeBytes: Scalars["Int"]["input"];
  SourceUrl?: InputMaybe<Scalars["String"]["input"]>;
  Width?: InputMaybe<Scalars["Int"]["input"]>;
};

export type CreateAudioStreamInput = {
  BitDepth?: InputMaybe<Scalars["Int"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
  ChannelLayout?: InputMaybe<Scalars["String"]["input"]>;
  Channels: Scalars["Int"]["input"];
  Codec: Scalars["String"]["input"];
  CodecLongName?: InputMaybe<Scalars["String"]["input"]>;
  IsCommentary: Scalars["Boolean"]["input"];
  IsDefault: Scalars["Boolean"]["input"];
  Language?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId: Scalars["String"]["input"];
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  SampleRate?: InputMaybe<Scalars["Int"]["input"]>;
  StreamIndex: Scalars["Int"]["input"];
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreateAudiobookInput = {
  Asin?: InputMaybe<Scalars["String"]["input"]>;
  AudibleId?: InputMaybe<Scalars["String"]["input"]>;
  AuthorName?: InputMaybe<Scalars["String"]["input"]>;
  AutoDownload: Scalars["Boolean"]["input"];
  AutoDownloadMode: AutoDownloadMode;
  ChapterCount?: InputMaybe<Scalars["Int"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  Description?: InputMaybe<Scalars["String"]["input"]>;
  GoodreadsId?: InputMaybe<Scalars["String"]["input"]>;
  HasFiles: Scalars["Boolean"]["input"];
  Isbn?: InputMaybe<Scalars["String"]["input"]>;
  Language?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  NarratorName?: InputMaybe<Scalars["String"]["input"]>;
  Narrators: Scalars["JSON"]["input"];
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PublishedDate?: InputMaybe<Scalars["String"]["input"]>;
  Publisher?: InputMaybe<Scalars["String"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  UserId: Scalars["String"]["input"];
};

export type CreateCastDeviceInput = {
  Address: Scalars["String"]["input"];
  DeviceType: Scalars["String"]["input"];
  IsFavorite: Scalars["Boolean"]["input"];
  IsManual: Scalars["Boolean"]["input"];
  LastSeenAt?: InputMaybe<Scalars["String"]["input"]>;
  Model?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  Port: Scalars["Int"]["input"];
};

export type CreateCastSessionInput = {
  CurrentPosition: Scalars["Float"]["input"];
  DeviceId?: InputMaybe<Scalars["String"]["input"]>;
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  EndedAt?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  IsMuted: Scalars["Boolean"]["input"];
  LastPosition?: InputMaybe<Scalars["Float"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  PlayerState: Scalars["String"]["input"];
  StartedAt: Scalars["String"]["input"];
  StreamUrl: Scalars["String"]["input"];
  Volume: Scalars["Float"]["input"];
};

export type CreateCastSettingInput = {
  AutoDiscoveryEnabled: Scalars["Boolean"]["input"];
  DefaultVolume: Scalars["Float"]["input"];
  DiscoveryIntervalSeconds: Scalars["Int"]["input"];
  PreferredQuality?: InputMaybe<Scalars["String"]["input"]>;
  TranscodeIncompatible: Scalars["Boolean"]["input"];
};

export type CreateChapterInput = {
  AudiobookId: Scalars["String"]["input"];
  ChapterNumber: Scalars["Int"]["input"];
  DurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  EndTimeSecs?: InputMaybe<Scalars["Float"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  StartTimeSecs: Scalars["Float"]["input"];
  Title?: InputMaybe<Scalars["String"]["input"]>;
  Wanted: Scalars["Boolean"]["input"];
};

export type CreateCollectionInput = {
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  LastSyncedAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MovieCount: Scalars["Int"]["input"];
  Name: Scalars["String"]["input"];
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  TmdbCollectionId: Scalars["Int"]["input"];
  UserId: Scalars["String"]["input"];
};

export type CreateDirectoryInput = {
  Path: Scalars["String"]["input"];
};

export type CreateEpisodeInput = {
  AbsoluteNumber?: InputMaybe<Scalars["Int"]["input"]>;
  AirDate?: InputMaybe<Scalars["String"]["input"]>;
  Episode: Scalars["Int"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  Season: Scalars["Int"]["input"];
  ShowId: Scalars["String"]["input"];
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeId?: InputMaybe<Scalars["Int"]["input"]>;
  Wanted: Scalars["Boolean"]["input"];
};

export type CreateInviteTokenInput = {
  AccessLevel: Scalars["String"]["input"];
  ApplyRestrictions: Scalars["Boolean"]["input"];
  CreatedBy: Scalars["String"]["input"];
  ExpiresAt?: InputMaybe<Scalars["String"]["input"]>;
  IsActive: Scalars["Boolean"]["input"];
  LibraryIds: Scalars["JSON"]["input"];
  MaxUses?: InputMaybe<Scalars["Int"]["input"]>;
  RestrictionsTemplate?: InputMaybe<Scalars["String"]["input"]>;
  Role: Scalars["String"]["input"];
  Token: Scalars["String"]["input"];
  UseCount: Scalars["Int"]["input"];
};

export type CreateLibraryInput = {
  AutoOrganize: Scalars["Boolean"]["input"];
  AutoScan: Scalars["Boolean"]["input"];
  Color?: InputMaybe<Scalars["String"]["input"]>;
  Icon?: InputMaybe<Scalars["String"]["input"]>;
  LastScannedAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  NamingPattern: Scalars["String"]["input"];
  Path: Scalars["String"]["input"];
  ScanIntervalMinutes: Scalars["Int"]["input"];
  Scanning: Scalars["Boolean"]["input"];
  UserId: Scalars["String"]["input"];
  WatchForChanges: Scalars["Boolean"]["input"];
};

export type CreateMediaChapterInput = {
  ChapterIndex: Scalars["Int"]["input"];
  EndSecs: Scalars["Float"]["input"];
  MediaFileId: Scalars["String"]["input"];
  StartSecs: Scalars["Float"]["input"];
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreateMediaFileInput = {
  AddedAt: Scalars["String"]["input"];
  AnalyzedAt?: InputMaybe<Scalars["String"]["input"]>;
  AudioChannels?: InputMaybe<Scalars["String"]["input"]>;
  AudioCodec?: InputMaybe<Scalars["String"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
  ChapterId?: InputMaybe<Scalars["String"]["input"]>;
  Container?: InputMaybe<Scalars["String"]["input"]>;
  ContentType?: InputMaybe<Scalars["String"]["input"]>;
  Duration?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  HdrType?: InputMaybe<Scalars["String"]["input"]>;
  Height?: InputMaybe<Scalars["Int"]["input"]>;
  IsHdr: Scalars["Boolean"]["input"];
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  OriginalName?: InputMaybe<Scalars["String"]["input"]>;
  Path: Scalars["String"]["input"];
  RelativePath?: InputMaybe<Scalars["String"]["input"]>;
  Resolution?: InputMaybe<Scalars["String"]["input"]>;
  Size: Scalars["Int"]["input"];
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
  VideoCodec?: InputMaybe<Scalars["String"]["input"]>;
  Width?: InputMaybe<Scalars["Int"]["input"]>;
};

export type CreateMetadataCacheInput = {
  CacheKey: Scalars["String"]["input"];
  FetchedAt: Scalars["String"]["input"];
  Operation: Scalars["String"]["input"];
  Payload: Scalars["String"]["input"];
  PayloadVersion: Scalars["Int"]["input"];
  Provider: Scalars["String"]["input"];
};

export type CreateMovieCastCreditInput = {
  CastOrder?: InputMaybe<Scalars["Int"]["input"]>;
  CharacterName?: InputMaybe<Scalars["String"]["input"]>;
  MovieId: Scalars["String"]["input"];
  PersonId: Scalars["String"]["input"];
};

export type CreateMovieInput = {
  CastNames: Scalars["JSON"]["input"];
  Certification?: InputMaybe<Scalars["String"]["input"]>;
  CollectionId?: InputMaybe<Scalars["Int"]["input"]>;
  CollectionName?: InputMaybe<Scalars["String"]["input"]>;
  CollectionPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Director?: InputMaybe<Scalars["String"]["input"]>;
  DownloadStatus?: InputMaybe<Scalars["String"]["input"]>;
  Genres: Scalars["JSON"]["input"];
  HasFile: Scalars["Boolean"]["input"];
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Monitored: Scalars["Boolean"]["input"];
  OriginalTitle?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  ProductionCountries: Scalars["JSON"]["input"];
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  SpokenLanguages: Scalars["JSON"]["input"];
  Tagline?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TmdbRating?: InputMaybe<Scalars["String"]["input"]>;
  TmdbStatus?: InputMaybe<Scalars["String"]["input"]>;
  TmdbVoteCount?: InputMaybe<Scalars["Int"]["input"]>;
  UserId: Scalars["String"]["input"];
  Wanted: Scalars["Boolean"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

export type CreateNamingPatternInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  IsDefault: Scalars["Boolean"]["input"];
  IsSystem: Scalars["Boolean"]["input"];
  LibraryType: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Pattern: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

export type CreateNotificationInput = {
  ActionData?: InputMaybe<Scalars["String"]["input"]>;
  ActionType?: InputMaybe<Scalars["String"]["input"]>;
  Category: Scalars["String"]["input"];
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Message: Scalars["String"]["input"];
  NotificationType: Scalars["String"]["input"];
  PendingMatchId?: InputMaybe<Scalars["String"]["input"]>;
  ReadAt?: InputMaybe<Scalars["String"]["input"]>;
  Resolution?: InputMaybe<Scalars["String"]["input"]>;
  ResolvedAt?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TorrentId?: InputMaybe<Scalars["String"]["input"]>;
  UserId: Scalars["String"]["input"];
};

export type CreatePendingFileMatchInput = {
  ChapterId?: InputMaybe<Scalars["String"]["input"]>;
  CopiedAt?: InputMaybe<Scalars["String"]["input"]>;
  CopyAttempts: Scalars["Int"]["input"];
  CopyError?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  FileSize: Scalars["Int"]["input"];
  MatchAttempts: Scalars["Int"]["input"];
  MatchConfidence?: InputMaybe<Scalars["Float"]["input"]>;
  MatchType?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  ParsedAudio?: InputMaybe<Scalars["String"]["input"]>;
  ParsedCodec?: InputMaybe<Scalars["String"]["input"]>;
  ParsedResolution?: InputMaybe<Scalars["String"]["input"]>;
  ParsedSource?: InputMaybe<Scalars["String"]["input"]>;
  SourceFileIndex?: InputMaybe<Scalars["Int"]["input"]>;
  SourceId?: InputMaybe<Scalars["String"]["input"]>;
  SourcePath: Scalars["String"]["input"];
  SourceType: Scalars["String"]["input"];
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
  UnmatchedReason?: InputMaybe<Scalars["String"]["input"]>;
  UserId: Scalars["String"]["input"];
  VerificationReason?: InputMaybe<Scalars["String"]["input"]>;
  VerificationStatus?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreatePersonInput = {
  Name: Scalars["String"]["input"];
  ProfileUrl?: InputMaybe<Scalars["String"]["input"]>;
  TmdbPersonId: Scalars["Int"]["input"];
};

export type CreatePlaybackProgressInput = {
  CurrentPosition: Scalars["Float"]["input"];
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  IsWatched: Scalars["Boolean"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  ProgressPercent: Scalars["Float"]["input"];
  UserId: Scalars["String"]["input"];
  WatchedAt?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreatePlaybackSessionInput = {
  AlbumId?: InputMaybe<Scalars["String"]["input"]>;
  AudiobookId?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  ContentType?: InputMaybe<Scalars["String"]["input"]>;
  CurrentPosition: Scalars["Float"]["input"];
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  IsMuted: Scalars["Boolean"]["input"];
  IsPlaying: Scalars["Boolean"]["input"];
  LastUpdatedAt: Scalars["String"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  StartedAt: Scalars["String"]["input"];
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
  TvShowId?: InputMaybe<Scalars["String"]["input"]>;
  UserId: Scalars["String"]["input"];
  Volume: Scalars["Float"]["input"];
};

export type CreateRefreshTokenInput = {
  ExpiresAt: Scalars["String"]["input"];
  Id: Scalars["String"]["input"];
  IpAddress?: InputMaybe<Scalars["String"]["input"]>;
  LastUsedAt?: InputMaybe<Scalars["String"]["input"]>;
  ReplacedByTokenId?: InputMaybe<Scalars["String"]["input"]>;
  RevocationReason?: InputMaybe<Scalars["String"]["input"]>;
  RevokedAt?: InputMaybe<Scalars["String"]["input"]>;
  Scopes: Scalars["JSON"]["input"];
  Session: Scalars["String"]["input"];
  SessionFamilyId: Scalars["String"]["input"];
  SessionId: Scalars["String"]["input"];
  TokenHash: Scalars["String"]["input"];
  UserAgent?: InputMaybe<Scalars["String"]["input"]>;
  UserId: Scalars["String"]["input"];
};

export type CreateRssFeedInput = {
  ConsecutiveFailures?: InputMaybe<Scalars["Int"]["input"]>;
  Enabled: Scalars["Boolean"]["input"];
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastPolledAt?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessfulAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  PollIntervalMinutes: Scalars["Int"]["input"];
  PostDownloadAction?: InputMaybe<Scalars["String"]["input"]>;
  Url: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

export type CreateRssFeedItemInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  FeedId: Scalars["String"]["input"];
  Guid?: InputMaybe<Scalars["String"]["input"]>;
  Link: Scalars["String"]["input"];
  LinkHash: Scalars["String"]["input"];
  ParsedAudio?: InputMaybe<Scalars["String"]["input"]>;
  ParsedCodec?: InputMaybe<Scalars["String"]["input"]>;
  ParsedEpisode?: InputMaybe<Scalars["Int"]["input"]>;
  ParsedHdr?: InputMaybe<Scalars["String"]["input"]>;
  ParsedResolution?: InputMaybe<Scalars["String"]["input"]>;
  ParsedSeason?: InputMaybe<Scalars["Int"]["input"]>;
  ParsedShowName?: InputMaybe<Scalars["String"]["input"]>;
  ParsedSource?: InputMaybe<Scalars["String"]["input"]>;
  Processed: Scalars["Boolean"]["input"];
  PubDate?: InputMaybe<Scalars["String"]["input"]>;
  SeenAt: Scalars["String"]["input"];
  SkippedReason?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TitleHash: Scalars["String"]["input"];
  TorrentId?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreateScheduleCacheInput = {
  AirDate: Scalars["String"]["input"];
  AirStamp?: InputMaybe<Scalars["String"]["input"]>;
  AirTime?: InputMaybe<Scalars["String"]["input"]>;
  CountryCode: Scalars["String"]["input"];
  EpisodeImageUrl?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeName: Scalars["String"]["input"];
  EpisodeNumber: Scalars["Int"]["input"];
  EpisodeType?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  Season: Scalars["Int"]["input"];
  ShowGenres: Scalars["JSON"]["input"];
  ShowName: Scalars["String"]["input"];
  ShowNetwork?: InputMaybe<Scalars["String"]["input"]>;
  ShowPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Summary?: InputMaybe<Scalars["String"]["input"]>;
  TvmazeEpisodeId: Scalars["Int"]["input"];
  TvmazeShowId: Scalars["Int"]["input"];
};

export type CreateScheduleSyncStateInput = {
  CountryCode: Scalars["String"]["input"];
  LastSyncDays: Scalars["Int"]["input"];
  LastSyncedAt: Scalars["String"]["input"];
  SyncError?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreateShowInput = {
  AutoDownload: Scalars["Boolean"]["input"];
  AutoDownloadMode: AutoDownloadMode;
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  ContentRating?: InputMaybe<Scalars["String"]["input"]>;
  Genres: Scalars["JSON"]["input"];
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Network?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeId?: InputMaybe<Scalars["Int"]["input"]>;
  UserId: Scalars["String"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

export type CreateSourceInput = {
  DefinitionId: Scalars["String"]["input"];
  Enabled: Scalars["Boolean"]["input"];
  ErrorCount: Scalars["Int"]["input"];
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastErrorAt?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessAt?: InputMaybe<Scalars["String"]["input"]>;
  MediaTypes: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Priority: Scalars["Int"]["input"];
  Settings?: InputMaybe<Scalars["String"]["input"]>;
  SiteUrl?: InputMaybe<Scalars["String"]["input"]>;
  SourceType: Scalars["String"]["input"];
  SupportsBookSearch: Scalars["Boolean"]["input"];
  SupportsMovieSearch: Scalars["Boolean"]["input"];
  SupportsMusicSearch: Scalars["Boolean"]["input"];
  SupportsSearch: Scalars["Boolean"]["input"];
  SupportsTvSearch: Scalars["Boolean"]["input"];
  credentials: Scalars["String"]["input"];
};

export type CreateSourcePriorityRuleInput = {
  Enabled: Scalars["Boolean"]["input"];
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  PriorityOrder: Scalars["JSON"]["input"];
  SearchAllSources: Scalars["Boolean"]["input"];
  UserId: Scalars["String"]["input"];
};

export type CreateSubtitleInput = {
  Codec?: InputMaybe<Scalars["String"]["input"]>;
  CodecLongName?: InputMaybe<Scalars["String"]["input"]>;
  DownloadedAt?: InputMaybe<Scalars["String"]["input"]>;
  FilePath?: InputMaybe<Scalars["String"]["input"]>;
  IsDefault: Scalars["Boolean"]["input"];
  IsForced: Scalars["Boolean"]["input"];
  IsHearingImpaired: Scalars["Boolean"]["input"];
  Language?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId: Scalars["String"]["input"];
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  OpensubtitlesId?: InputMaybe<Scalars["String"]["input"]>;
  SourceType: Scalars["String"]["input"];
  StreamIndex?: InputMaybe<Scalars["Int"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreateTorrentFileInput = {
  DownloadedBytes: Scalars["Int"]["input"];
  FileIndex: Scalars["Int"]["input"];
  FilePath: Scalars["String"]["input"];
  FileSize: Scalars["Int"]["input"];
  IsExcluded: Scalars["Boolean"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Progress: Scalars["Float"]["input"];
  RelativePath: Scalars["String"]["input"];
  TorrentId: Scalars["String"]["input"];
};

export type CreateTorrentInput = {
  AddedAt: Scalars["String"]["input"];
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  DownloadPath?: InputMaybe<Scalars["String"]["input"]>;
  DownloadedBytes: Scalars["Int"]["input"];
  ExcludedFiles: Scalars["JSON"]["input"];
  InfoHash: Scalars["String"]["input"];
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MagnetUri?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  PostProcessError?: InputMaybe<Scalars["String"]["input"]>;
  PostProcessStatus?: InputMaybe<Scalars["String"]["input"]>;
  ProcessedAt?: InputMaybe<Scalars["String"]["input"]>;
  Progress: Scalars["Float"]["input"];
  SavePath: Scalars["String"]["input"];
  SourceFeedId?: InputMaybe<Scalars["String"]["input"]>;
  SourceIndexerId?: InputMaybe<Scalars["String"]["input"]>;
  SourceUrl?: InputMaybe<Scalars["String"]["input"]>;
  State: Scalars["String"]["input"];
  TotalBytes: Scalars["Int"]["input"];
  UploadedBytes: Scalars["Int"]["input"];
  UserId: Scalars["String"]["input"];
};

export type CreateTorznabCategoryInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  ParentId?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreateTrackInput = {
  AlbumId: Scalars["String"]["input"];
  ArtistId?: InputMaybe<Scalars["String"]["input"]>;
  ArtistName?: InputMaybe<Scalars["String"]["input"]>;
  DiscNumber?: InputMaybe<Scalars["Int"]["input"]>;
  DurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  Explicit: Scalars["Boolean"]["input"];
  Isrc?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TrackNumber: Scalars["Int"]["input"];
  Wanted: Scalars["Boolean"]["input"];
};

export type CreateUsenetDownloadInput = {
  AlbumId?: InputMaybe<Scalars["String"]["input"]>;
  AudiobookId?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  DownloadPath?: InputMaybe<Scalars["String"]["input"]>;
  DownloadSpeed?: InputMaybe<Scalars["Int"]["input"]>;
  DownloadedBytes?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  ErrorMessage?: InputMaybe<Scalars["String"]["input"]>;
  EtaSeconds?: InputMaybe<Scalars["Int"]["input"]>;
  IndexerId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  NzbData?: InputMaybe<Scalars["String"]["input"]>;
  NzbHash?: InputMaybe<Scalars["String"]["input"]>;
  NzbName: Scalars["String"]["input"];
  NzbUrl?: InputMaybe<Scalars["String"]["input"]>;
  PostProcessStatus?: InputMaybe<Scalars["String"]["input"]>;
  Progress?: InputMaybe<Scalars["String"]["input"]>;
  RetryCount: Scalars["Int"]["input"];
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  State: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

export type CreateUsenetServerInput = {
  Connections: Scalars["Int"]["input"];
  Enabled: Scalars["Boolean"]["input"];
  EncryptedPassword?: InputMaybe<Scalars["String"]["input"]>;
  ErrorCount: Scalars["Int"]["input"];
  Host: Scalars["String"]["input"];
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessAt?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  PasswordNonce?: InputMaybe<Scalars["String"]["input"]>;
  Port: Scalars["Int"]["input"];
  Priority: Scalars["Int"]["input"];
  RetentionDays?: InputMaybe<Scalars["Int"]["input"]>;
  UseSsl: Scalars["Boolean"]["input"];
  UserId: Scalars["String"]["input"];
  Username?: InputMaybe<Scalars["String"]["input"]>;
};

export type CreateUserInput = {
  AvatarUrl?: InputMaybe<Scalars["String"]["input"]>;
  DisplayName?: InputMaybe<Scalars["String"]["input"]>;
  Email?: InputMaybe<Scalars["String"]["input"]>;
  IsActive: Scalars["Boolean"]["input"];
  LastLoginAt?: InputMaybe<Scalars["String"]["input"]>;
  Role: Scalars["String"]["input"];
  Username: Scalars["String"]["input"];
};

export type CreateVideoStreamInput = {
  AspectRatio?: InputMaybe<Scalars["String"]["input"]>;
  AvgFrameRate?: InputMaybe<Scalars["String"]["input"]>;
  BitDepth?: InputMaybe<Scalars["Int"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
  Codec: Scalars["String"]["input"];
  CodecLongName?: InputMaybe<Scalars["String"]["input"]>;
  ColorPrimaries?: InputMaybe<Scalars["String"]["input"]>;
  ColorSpace?: InputMaybe<Scalars["String"]["input"]>;
  ColorTransfer?: InputMaybe<Scalars["String"]["input"]>;
  FrameRate?: InputMaybe<Scalars["String"]["input"]>;
  HdrType?: InputMaybe<Scalars["String"]["input"]>;
  Height: Scalars["Int"]["input"];
  IsDefault: Scalars["Boolean"]["input"];
  Language?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId: Scalars["String"]["input"];
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  PixelFormat?: InputMaybe<Scalars["String"]["input"]>;
  StreamIndex: Scalars["Int"]["input"];
  Title?: InputMaybe<Scalars["String"]["input"]>;
  Width: Scalars["Int"]["input"];
};

export type DateFilter = {
  between?: InputMaybe<DateRangeInput>;
  eq?: InputMaybe<Scalars["String"]["input"]>;
  gt?: InputMaybe<Scalars["String"]["input"]>;
  gte?: InputMaybe<Scalars["String"]["input"]>;
  gteRelative?: InputMaybe<RelativeDateInput>;
  inFuture?: InputMaybe<Scalars["Boolean"]["input"]>;
  inPast?: InputMaybe<Scalars["Boolean"]["input"]>;
  isNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  isToday?: InputMaybe<Scalars["Boolean"]["input"]>;
  lt?: InputMaybe<Scalars["String"]["input"]>;
  lte?: InputMaybe<Scalars["String"]["input"]>;
  lteRelative?: InputMaybe<RelativeDateInput>;
  ne?: InputMaybe<Scalars["String"]["input"]>;
  recentDays?: InputMaybe<Scalars["Int"]["input"]>;
  withinDays?: InputMaybe<Scalars["Int"]["input"]>;
};

export type DateRangeInput = {
  end?: InputMaybe<Scalars["String"]["input"]>;
  start?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk delete by Where filter */
export type DeleteAlbumsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAppLogsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAppSettingsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteArtistsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteArtworkCachesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAudioStreamsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAudiobooksResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteCastDevicesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteCastSessionsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteCastSettingsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteChaptersResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteCollectionsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteEpisodesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type DeleteFilesInput = {
  Paths: Array<Scalars["String"]["input"]>;
  Recursive?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Result of bulk delete by Where filter */
export type DeleteInviteTokensResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteLibrariesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMediaChaptersResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMediaFilesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMetadataCachesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMovieCastCreditsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMoviesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteNamingPatternsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteNotificationsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeletePendingFileMatchesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeletePeopleResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeletePlaybackProgressesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeletePlaybackSessionsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteRefreshTokensResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteRssFeedItemsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteRssFeedsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteScheduleCachesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteScheduleSyncStatesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteShowsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteSourcePriorityRulesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteSourcesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteSubtitlesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTorrentFilesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTorrentsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTorznabCategoriesResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTracksResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteUsenetDownloadsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteUsenetServersResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteUsersResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteVideoStreamsResult = {
  deletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type Episode = {
  AbsoluteNumber?: Maybe<Scalars["Int"]["output"]>;
  AirDate?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Episode: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  Season: Scalars["Int"]["output"];
  ShowId: Scalars["String"]["output"];
  Title?: Maybe<Scalars["String"]["output"]>;
  TmdbId?: Maybe<Scalars["Int"]["output"]>;
  TvdbId?: Maybe<Scalars["Int"]["output"]>;
  TvmazeId?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  Wanted: Scalars["Boolean"]["output"];
  /** Get related #graphql_name */
  mediaFile?: Maybe<MediaFile>;
};

/** Event for #struct_name changes (subscriptions) */
export type EpisodeChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  episode?: Maybe<Episode>;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type EpisodeConnection = {
  /** The edges in this connection */
  edges: Array<EpisodeEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type EpisodeEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Episode;
};

export type EpisodeOrderByInput = {
  AirDate?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  Episode?: InputMaybe<OrderDirection>;
  Season?: InputMaybe<OrderDirection>;
  Title?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type EpisodeResult = {
  episode?: Maybe<Episode>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type EpisodeWhereInput = {
  AbsoluteNumber?: InputMaybe<IntFilter>;
  AirDate?: InputMaybe<DateFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Episode?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  Runtime?: InputMaybe<IntFilter>;
  Season?: InputMaybe<IntFilter>;
  ShowId?: InputMaybe<StringFilter>;
  Title?: InputMaybe<StringFilter>;
  TmdbId?: InputMaybe<IntFilter>;
  TvdbId?: InputMaybe<IntFilter>;
  TvmazeId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Wanted?: InputMaybe<BoolFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<EpisodeWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<EpisodeWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<EpisodeWhereInput>>;
};

export type FileOperationPayload = {
  AffectedCount: Scalars["Int"]["output"];
  Error?: Maybe<Scalars["String"]["output"]>;
  Messages: Array<Scalars["String"]["output"]>;
  Path?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

/** Event emitted when a filesystem mutation completes (PascalCase for GraphQL). */
export type FilesystemChangeEvent = {
  ChangeType: Scalars["String"]["output"];
  Name?: Maybe<Scalars["String"]["output"]>;
  NewName?: Maybe<Scalars["String"]["output"]>;
  Path: Scalars["String"]["output"];
  Timestamp: Scalars["String"]["output"];
};

/** Runtime filesystem/network capabilities exposed to frontend. */
export type FilesystemRuntimeInfo = {
  DefaultLinuxMountBase?: Maybe<Scalars["String"]["output"]>;
  Platform: Scalars["String"]["output"];
  SupportsSambaMount: Scalars["Boolean"]["output"];
  SupportsUncCredentials: Scalars["Boolean"]["output"];
};

export type IntFilter = {
  eq?: InputMaybe<Scalars["Int"]["input"]>;
  gt?: InputMaybe<Scalars["Int"]["input"]>;
  gte?: InputMaybe<Scalars["Int"]["input"]>;
  inList?: InputMaybe<Array<Scalars["Int"]["input"]>>;
  isNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  lt?: InputMaybe<Scalars["Int"]["input"]>;
  lte?: InputMaybe<Scalars["Int"]["input"]>;
  ne?: InputMaybe<Scalars["Int"]["input"]>;
  notIn?: InputMaybe<Array<Scalars["Int"]["input"]>>;
};

export type InviteToken = {
  AccessLevel: Scalars["String"]["output"];
  ApplyRestrictions: Scalars["Boolean"]["output"];
  CreatedAt: Scalars["String"]["output"];
  CreatedBy: Scalars["String"]["output"];
  ExpiresAt?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  IsActive: Scalars["Boolean"]["output"];
  LibraryIds: Array<Scalars["String"]["output"]>;
  MaxUses?: Maybe<Scalars["Int"]["output"]>;
  RestrictionsTemplate?: Maybe<Scalars["String"]["output"]>;
  Role: Scalars["String"]["output"];
  Token: Scalars["String"]["output"];
  UseCount: Scalars["Int"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type InviteTokenChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  inviteToken?: Maybe<InviteToken>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type InviteTokenConnection = {
  /** The edges in this connection */
  edges: Array<InviteTokenEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type InviteTokenEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: InviteToken;
};

export type InviteTokenOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type InviteTokenResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  inviteToken?: Maybe<InviteToken>;
  success: Scalars["Boolean"]["output"];
};

export type InviteTokenWhereInput = {
  AccessLevel?: InputMaybe<StringFilter>;
  ApplyRestrictions?: InputMaybe<BoolFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  CreatedBy?: InputMaybe<StringFilter>;
  ExpiresAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IsActive?: InputMaybe<BoolFilter>;
  MaxUses?: InputMaybe<IntFilter>;
  Role?: InputMaybe<StringFilter>;
  Token?: InputMaybe<StringFilter>;
  UseCount?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<InviteTokenWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<InviteTokenWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<InviteTokenWhereInput>>;
};

export type LegacyAddCastDeviceInput = {
  address: Scalars["String"]["input"];
  name?: InputMaybe<Scalars["String"]["input"]>;
  port?: InputMaybe<Scalars["Int"]["input"]>;
};

export type LegacyCastDevice = {
  address: Scalars["String"]["output"];
  deviceType: Scalars["String"]["output"];
  id: Scalars["String"]["output"];
  isConnected: Scalars["Boolean"]["output"];
  isFavorite: Scalars["Boolean"]["output"];
  isManual: Scalars["Boolean"]["output"];
  lastSeenAt?: Maybe<Scalars["String"]["output"]>;
  model?: Maybe<Scalars["String"]["output"]>;
  name: Scalars["String"]["output"];
  port: Scalars["Int"]["output"];
};

export type LegacyCastSession = {
  currentTime: Scalars["Float"]["output"];
  deviceId?: Maybe<Scalars["String"]["output"]>;
  deviceName?: Maybe<Scalars["String"]["output"]>;
  duration?: Maybe<Scalars["Float"]["output"]>;
  episodeId?: Maybe<Scalars["String"]["output"]>;
  id: Scalars["String"]["output"];
  isMuted: Scalars["Boolean"]["output"];
  mediaFileId?: Maybe<Scalars["String"]["output"]>;
  playerState: Scalars["String"]["output"];
  startedAt: Scalars["String"]["output"];
  streamUrl: Scalars["String"]["output"];
  volume: Scalars["Float"]["output"];
};

export type LegacyCastSettings = {
  autoDiscoveryEnabled: Scalars["Boolean"]["output"];
  defaultVolume: Scalars["Float"]["output"];
  discoveryIntervalSeconds: Scalars["Int"]["output"];
  preferredQuality?: Maybe<Scalars["String"]["output"]>;
  transcodeIncompatible: Scalars["Boolean"]["output"];
};

export type LegacyUpdateCastDeviceInput = {
  address?: InputMaybe<Scalars["String"]["input"]>;
  deviceType?: InputMaybe<Scalars["String"]["input"]>;
  isFavorite?: InputMaybe<Scalars["Boolean"]["input"]>;
  isManual?: InputMaybe<Scalars["Boolean"]["input"]>;
  model?: InputMaybe<Scalars["String"]["input"]>;
  name?: InputMaybe<Scalars["String"]["input"]>;
  port?: InputMaybe<Scalars["Int"]["input"]>;
};

export type LegacyUpdateCastSettingsInput = {
  autoDiscoveryEnabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  defaultVolume?: InputMaybe<Scalars["Float"]["input"]>;
  discoveryIntervalSeconds?: InputMaybe<Scalars["Int"]["input"]>;
  preferredQuality?: InputMaybe<Scalars["String"]["input"]>;
  transcodeIncompatible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type Library = {
  AutoOrganize: Scalars["Boolean"]["output"];
  AutoScan: Scalars["Boolean"]["output"];
  Color?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Icon?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  LastScannedAt?: Maybe<Scalars["String"]["output"]>;
  LibraryType: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  NamingPattern: Scalars["String"]["output"];
  Path: Scalars["String"]["output"];
  ScanIntervalMinutes: Scalars["Int"]["output"];
  Scanning: Scalars["Boolean"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  WatchForChanges: Scalars["Boolean"]["output"];
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  albums: AlbumConnection;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  audiobooks: AudiobookConnection;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  collections: CollectionConnection;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  mediaFiles: MediaFileConnection;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  movies: MovieConnection;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  shows: ShowConnection;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryalbumsArgs = {
  orderBy?: InputMaybe<AlbumOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AlbumWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryaudiobooksArgs = {
  orderBy?: InputMaybe<AudiobookOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AudiobookWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibrarycollectionsArgs = {
  orderBy?: InputMaybe<CollectionOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<CollectionWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibrarymediaFilesArgs = {
  orderBy?: InputMaybe<MediaFileOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MediaFileWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibrarymoviesArgs = {
  orderBy?: InputMaybe<MovieOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MovieWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryshowsArgs = {
  orderBy?: InputMaybe<ShowOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ShowWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type LibraryChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  library?: Maybe<Library>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type LibraryConnection = {
  /** The edges in this connection */
  edges: Array<LibraryEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type LibraryEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Library;
};

export type LibraryOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  Id?: InputMaybe<OrderDirection>;
  LastScannedAt?: InputMaybe<OrderDirection>;
  LibraryType?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

export type LibraryPathAvailability = {
  Exists: Scalars["Boolean"]["output"];
  IsDirectory: Scalars["Boolean"]["output"];
  Message?: Maybe<Scalars["String"]["output"]>;
  NeedsReconnect: Scalars["Boolean"]["output"];
  Path: Scalars["String"]["output"];
  Reachable: Scalars["Boolean"]["output"];
  ReconnectAttempted: Scalars["Boolean"]["output"];
  ReconnectSucceeded: Scalars["Boolean"]["output"];
};

export type LibraryPathAvailabilityInput = {
  AttemptReconnect?: InputMaybe<Scalars["Boolean"]["input"]>;
  Paths: Array<Scalars["String"]["input"]>;
};

/** Result type for #struct_name mutations */
export type LibraryResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  library?: Maybe<Library>;
  success: Scalars["Boolean"]["output"];
};

export type LibraryWhereInput = {
  AutoOrganize?: InputMaybe<BoolFilter>;
  AutoScan?: InputMaybe<BoolFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastScannedAt?: InputMaybe<DateFilter>;
  LibraryType?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  NamingPattern?: InputMaybe<StringFilter>;
  Path?: InputMaybe<StringFilter>;
  ScanIntervalMinutes?: InputMaybe<IntFilter>;
  Scanning?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  WatchForChanges?: InputMaybe<BoolFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<LibraryWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<LibraryWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<LibraryWhereInput>>;
};

/** Live torrent (from torrent client, not DB) */
export type LiveTorrent = {
  DownloadSpeed: Scalars["Int"]["output"];
  Downloaded: Scalars["Int"]["output"];
  Files: Array<LiveTorrentFile>;
  Id: Scalars["Int"]["output"];
  InfoHash: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  Peers: Scalars["Int"]["output"];
  Progress: Scalars["Float"]["output"];
  SavePath: Scalars["String"]["output"];
  Size: Scalars["Int"]["output"];
  State: Scalars["String"]["output"];
  UploadSpeed: Scalars["Int"]["output"];
  Uploaded: Scalars["Int"]["output"];
};

/** Live torrent file (from torrent client) */
export type LiveTorrentFile = {
  Index: Scalars["Int"]["output"];
  Path: Scalars["String"]["output"];
  Progress: Scalars["Float"]["output"];
  Size: Scalars["Int"]["output"];
};

/** GraphQL input for login (username or email + password). */
export type LoginInput = {
  Password: Scalars["String"]["input"];
  UsernameOrEmail: Scalars["String"]["input"];
};

/** GraphQL input for logout (refresh token to invalidate). */
export type LogoutInput = {
  RefreshToken: Scalars["String"]["input"];
};

export type LogoutPayload = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type MatchCandidate = {
  Reason?: Maybe<Scalars["String"]["output"]>;
  Score: Scalars["Float"]["output"];
  TargetId: Scalars["String"]["output"];
  TargetName?: Maybe<Scalars["String"]["output"]>;
  TargetType: Scalars["String"]["output"];
  Wanted?: Maybe<Scalars["Boolean"]["output"]>;
};

export type MatchMediaFileInput = {
  AllowProviderFallback?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoMatch?: InputMaybe<Scalars["Boolean"]["input"]>;
  CandidateLimit?: InputMaybe<Scalars["Int"]["input"]>;
  ChapterId?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  Force?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId: Scalars["String"]["input"];
  Methods?: InputMaybe<Array<MatchMethod>>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
  WantedPolicy?: InputMaybe<MatchWantedPolicy>;
};

export type MatchMediaFileResult = {
  AlreadyMatched: Scalars["Boolean"]["output"];
  AutoMatched: Scalars["Boolean"]["output"];
  Candidates: Array<MatchCandidate>;
  Confidence: Scalars["Float"]["output"];
  MatchedId?: Maybe<Scalars["String"]["output"]>;
  MatchedType?: Maybe<Scalars["String"]["output"]>;
  Reason?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export const MatchMethod = {
  FILENAME: "FILENAME",
  METADATA: "METADATA",
  OLLAMA: "OLLAMA",
} as const;

export type MatchMethod = (typeof MatchMethod)[keyof typeof MatchMethod];
export const MatchWantedPolicy = {
  ALL: "ALL",
  PREFER_WANTED: "PREFER_WANTED",
  WANTED_ONLY: "WANTED_ONLY",
} as const;

export type MatchWantedPolicy =
  (typeof MatchWantedPolicy)[keyof typeof MatchWantedPolicy];
/** Current user info returned by Me query (PascalCase). */
export type MeUser = {
  DisplayName?: Maybe<Scalars["String"]["output"]>;
  Email?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  Role: Scalars["String"]["output"];
  Username: Scalars["String"]["output"];
};

export type MediaChapter = {
  ChapterIndex: Scalars["Int"]["output"];
  CreatedAt: Scalars["String"]["output"];
  EndSecs: Scalars["Float"]["output"];
  Id: Scalars["String"]["output"];
  MediaFileId: Scalars["String"]["output"];
  StartSecs: Scalars["Float"]["output"];
  Title?: Maybe<Scalars["String"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type MediaChapterChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  mediaChapter?: Maybe<MediaChapter>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type MediaChapterConnection = {
  /** The edges in this connection */
  edges: Array<MediaChapterEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MediaChapterEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: MediaChapter;
};

export type MediaChapterOrderByInput = {
  ChapterIndex?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type MediaChapterResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  mediaChapter?: Maybe<MediaChapter>;
  success: Scalars["Boolean"]["output"];
};

export type MediaChapterWhereInput = {
  ChapterIndex?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EndSecs?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  StartSecs?: InputMaybe<IntFilter>;
  Title?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<MediaChapterWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<MediaChapterWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<MediaChapterWhereInput>>;
};

export type MediaFile = {
  AddedAt: Scalars["String"]["output"];
  AnalyzedAt?: Maybe<Scalars["String"]["output"]>;
  AudioChannels?: Maybe<Scalars["String"]["output"]>;
  AudioCodec?: Maybe<Scalars["String"]["output"]>;
  Bitrate?: Maybe<Scalars["Int"]["output"]>;
  ChapterId?: Maybe<Scalars["String"]["output"]>;
  Container?: Maybe<Scalars["String"]["output"]>;
  ContentType?: Maybe<Scalars["String"]["output"]>;
  Duration?: Maybe<Scalars["Int"]["output"]>;
  EpisodeId?: Maybe<Scalars["String"]["output"]>;
  HdrType?: Maybe<Scalars["String"]["output"]>;
  Height?: Maybe<Scalars["Int"]["output"]>;
  Id: Scalars["String"]["output"];
  IsHdr: Scalars["Boolean"]["output"];
  LibraryId?: Maybe<Scalars["String"]["output"]>;
  Metadata?: Maybe<Scalars["String"]["output"]>;
  MovieId?: Maybe<Scalars["String"]["output"]>;
  OriginalName?: Maybe<Scalars["String"]["output"]>;
  Path: Scalars["String"]["output"];
  RelativePath?: Maybe<Scalars["String"]["output"]>;
  Resolution?: Maybe<Scalars["String"]["output"]>;
  Size: Scalars["Int"]["output"];
  TrackId?: Maybe<Scalars["String"]["output"]>;
  VideoCodec?: Maybe<Scalars["String"]["output"]>;
  Width?: Maybe<Scalars["Int"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type MediaFileChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  mediaFile?: Maybe<MediaFile>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type MediaFileConnection = {
  /** The edges in this connection */
  edges: Array<MediaFileEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MediaFileEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: MediaFile;
};

export type MediaFileOrderByInput = {
  AddedAt?: InputMaybe<OrderDirection>;
  AnalyzedAt?: InputMaybe<OrderDirection>;
  Duration?: InputMaybe<OrderDirection>;
  Path?: InputMaybe<OrderDirection>;
  Resolution?: InputMaybe<OrderDirection>;
  Size?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type MediaFileResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  mediaFile?: Maybe<MediaFile>;
  success: Scalars["Boolean"]["output"];
};

export type MediaFileWhereInput = {
  AddedAt?: InputMaybe<DateFilter>;
  AnalyzedAt?: InputMaybe<DateFilter>;
  AudioChannels?: InputMaybe<StringFilter>;
  AudioCodec?: InputMaybe<StringFilter>;
  Bitrate?: InputMaybe<IntFilter>;
  ChapterId?: InputMaybe<StringFilter>;
  Container?: InputMaybe<StringFilter>;
  ContentType?: InputMaybe<StringFilter>;
  Duration?: InputMaybe<IntFilter>;
  EpisodeId?: InputMaybe<StringFilter>;
  HdrType?: InputMaybe<StringFilter>;
  Height?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IsHdr?: InputMaybe<BoolFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MovieId?: InputMaybe<StringFilter>;
  Path?: InputMaybe<StringFilter>;
  Resolution?: InputMaybe<StringFilter>;
  Size?: InputMaybe<IntFilter>;
  TrackId?: InputMaybe<StringFilter>;
  VideoCodec?: InputMaybe<StringFilter>;
  Width?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<MediaFileWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<MediaFileWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<MediaFileWhereInput>>;
};

export type MetadataCache = {
  CacheKey: Scalars["String"]["output"];
  CreatedAt: Scalars["String"]["output"];
  FetchedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  Operation: Scalars["String"]["output"];
  Payload: Scalars["String"]["output"];
  PayloadVersion: Scalars["Int"]["output"];
  Provider: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type MetadataCacheChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  metadataCache?: Maybe<MetadataCache>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type MetadataCacheConnection = {
  /** The edges in this connection */
  edges: Array<MetadataCacheEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MetadataCacheEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: MetadataCache;
};

export type MetadataCacheOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  FetchedAt?: InputMaybe<OrderDirection>;
  Operation?: InputMaybe<OrderDirection>;
  Provider?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type MetadataCacheResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  metadataCache?: Maybe<MetadataCache>;
  success: Scalars["Boolean"]["output"];
};

export type MetadataCacheWhereInput = {
  CacheKey?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  FetchedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  Operation?: InputMaybe<StringFilter>;
  PayloadVersion?: InputMaybe<IntFilter>;
  Provider?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<MetadataCacheWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<MetadataCacheWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<MetadataCacheWhereInput>>;
};

export type MoveFilesInput = {
  Destination: Scalars["String"]["input"];
  Overwrite?: InputMaybe<Scalars["Boolean"]["input"]>;
  Sources: Array<Scalars["String"]["input"]>;
};

export type Movie = {
  CastNames: Array<Scalars["String"]["output"]>;
  Certification?: Maybe<Scalars["String"]["output"]>;
  CollectionId?: Maybe<Scalars["Int"]["output"]>;
  CollectionName?: Maybe<Scalars["String"]["output"]>;
  CollectionPosterUrl?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Director?: Maybe<Scalars["String"]["output"]>;
  DownloadStatus?: Maybe<Scalars["String"]["output"]>;
  Genres: Array<Scalars["String"]["output"]>;
  HasFile: Scalars["Boolean"]["output"];
  Id: Scalars["String"]["output"];
  ImdbId?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  Monitored: Scalars["Boolean"]["output"];
  OriginalTitle?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  ProductionCountries: Array<Scalars["String"]["output"]>;
  ReleaseDate?: Maybe<Scalars["String"]["output"]>;
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  SortTitle?: Maybe<Scalars["String"]["output"]>;
  SpokenLanguages: Array<Scalars["String"]["output"]>;
  Tagline?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TmdbId?: Maybe<Scalars["Int"]["output"]>;
  TmdbRating?: Maybe<Scalars["String"]["output"]>;
  TmdbStatus?: Maybe<Scalars["String"]["output"]>;
  TmdbVoteCount?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  Wanted: Scalars["Boolean"]["output"];
  Year?: Maybe<Scalars["Int"]["output"]>;
  /** Get related #graphql_name */
  mediaFile?: Maybe<MediaFile>;
};

export type MovieCastCredit = {
  CastOrder?: Maybe<Scalars["Int"]["output"]>;
  CharacterName?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  MovieId: Scalars["String"]["output"];
  PersonId: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  /** Get related #graphql_name */
  movie?: Maybe<Movie>;
  /** Get related #graphql_name */
  person?: Maybe<Person>;
};

/** Event for #struct_name changes (subscriptions) */
export type MovieCastCreditChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  movieCastCredit?: Maybe<MovieCastCredit>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type MovieCastCreditConnection = {
  /** The edges in this connection */
  edges: Array<MovieCastCreditEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MovieCastCreditEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: MovieCastCredit;
};

export type MovieCastCreditOrderByInput = {
  CastOrder?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type MovieCastCreditResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  movieCastCredit?: Maybe<MovieCastCredit>;
  success: Scalars["Boolean"]["output"];
};

export type MovieCastCreditWhereInput = {
  CastOrder?: InputMaybe<IntFilter>;
  CharacterName?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  MovieId?: InputMaybe<StringFilter>;
  PersonId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<MovieCastCreditWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<MovieCastCreditWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<MovieCastCreditWhereInput>>;
};

/** Event for #struct_name changes (subscriptions) */
export type MovieChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  movie?: Maybe<Movie>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Full TMDB collection details with local overlay */
export type MovieCollectionDetails = {
  BackdropUrl?: Maybe<Scalars["String"]["output"]>;
  CollectionId: Scalars["Int"]["output"];
  Movies: Array<MovieCollectionMovieDetails>;
  Name: Scalars["String"]["output"];
  Overview?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
};

/** Movie row in a collection detail response */
export type MovieCollectionMovieDetails = {
  AudioChannels?: Maybe<Scalars["String"]["output"]>;
  AudioCodec?: Maybe<Scalars["String"]["output"]>;
  FileSizeBytes?: Maybe<Scalars["Int"]["output"]>;
  LibraryMovieId?: Maybe<Scalars["String"]["output"]>;
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  Resolution?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TmdbId: Scalars["Int"]["output"];
  VideoCodec?: Maybe<Scalars["String"]["output"]>;
  Wanted: Scalars["Boolean"]["output"];
  Year?: Maybe<Scalars["Int"]["output"]>;
};

/** Result of importing a movie collection */
export type MovieCollectionOperationResult = {
  CollectionId?: Maybe<Scalars["Int"]["output"]>;
  CollectionName?: Maybe<Scalars["String"]["output"]>;
  Error?: Maybe<Scalars["String"]["output"]>;
  ExistingCount: Scalars["Int"]["output"];
  ImportedCount: Scalars["Int"]["output"];
  Success: Scalars["Boolean"]["output"];
  WantedUpdatedCount: Scalars["Int"]["output"];
};

/** Movie collection search result from TMDB */
export type MovieCollectionSearchResult = {
  BackdropUrl?: Maybe<Scalars["String"]["output"]>;
  CollectionId: Scalars["Int"]["output"];
  Name: Scalars["String"]["output"];
  Overview?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  Provider: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type MovieConnection = {
  /** The edges in this connection */
  edges: Array<MovieEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MovieEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Movie;
};

/** Result of movie operations */
export type MovieOperationResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Movie?: Maybe<Movie>;
  Success: Scalars["Boolean"]["output"];
};

export type MovieOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  ReleaseDate?: InputMaybe<OrderDirection>;
  Runtime?: InputMaybe<OrderDirection>;
  SortTitle?: InputMaybe<OrderDirection>;
  Title?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
  Year?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type MovieResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  movie?: Maybe<Movie>;
  success: Scalars["Boolean"]["output"];
};

/** Movie search result from TMDB */
export type MovieSearchResult = {
  BackdropUrl?: Maybe<Scalars["String"]["output"]>;
  ImdbId?: Maybe<Scalars["String"]["output"]>;
  OriginalTitle?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  Popularity?: Maybe<Scalars["Float"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  Provider: Scalars["String"]["output"];
  ProviderId: Scalars["Int"]["output"];
  Title: Scalars["String"]["output"];
  VoteAverage?: Maybe<Scalars["Float"]["output"]>;
  Year?: Maybe<Scalars["Int"]["output"]>;
};

export type MovieWhereInput = {
  Certification?: InputMaybe<StringFilter>;
  CollectionId?: InputMaybe<IntFilter>;
  CollectionName?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Director?: InputMaybe<StringFilter>;
  DownloadStatus?: InputMaybe<StringFilter>;
  HasFile?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  ImdbId?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  Monitored?: InputMaybe<BoolFilter>;
  ReleaseDate?: InputMaybe<DateFilter>;
  Runtime?: InputMaybe<IntFilter>;
  Title?: InputMaybe<StringFilter>;
  TmdbId?: InputMaybe<IntFilter>;
  TmdbStatus?: InputMaybe<StringFilter>;
  TmdbVoteCount?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Wanted?: InputMaybe<BoolFilter>;
  Year?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<MovieWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<MovieWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<MovieWhereInput>>;
};

export type MutationRoot = {
  /** Add an album to a library by fetching metadata from MusicBrainz. */
  AddAlbum: AlbumOperationResult;
  /** Add an audiobook to a library by fetching metadata from OpenLibrary. */
  AddAudiobook: AudiobookOperationResult;
  AddCastDevice: CastDeviceOperationResult;
  /** Add a movie to a library by fetching metadata from TMDB */
  AddMovie: MovieOperationResult;
  /** Add/import all movies from a TMDB collection into a library. */
  AddMovieCollection: MovieCollectionOperationResult;
  /** Add a torrent from a magnet link or URL */
  AddTorrent: AddTorrentResult;
  /** Add a TV show to a library by fetching metadata from TVMaze */
  AddTvShow: TvShowOperationResult;
  AnalyzeMediaFile: AnalyzeMediaFileResult;
  CastMedia: CastSessionOperationResult;
  CastPause: CastSessionOperationResult;
  CastPlay: CastSessionOperationResult;
  CastSeek: CastSessionOperationResult;
  CastSetMuted: CastSessionOperationResult;
  CastSetVolume: CastSessionOperationResult;
  CastStop: CastActionResult;
  ConfigureNetworkPath: NetworkPathConfigPayload;
  CopyFiles: FileOperationPayload;
  CreateDirectory: FileOperationPayload;
  DeleteFiles: FileOperationPayload;
  DiscoverCastDevices: Array<LegacyCastDevice>;
  Login: AuthPayload;
  Logout: LogoutPayload;
  MatchMediaFile: MatchMediaFileResult;
  MoveFiles: FileOperationPayload;
  OrganizeMediaFile: OrganizeMediaFileResult;
  /** Pause a torrent */
  PauseTorrent: TorrentActionResult;
  PauseTorrentByInfoHash: TorrentActionResult;
  /**
   * Process pending file matches for a source.
   * Note: full processing pipeline from legacy code is being re-implemented.
   */
  ProcessSource: ProcessSourceResult;
  /** Recache artwork for all movies (runs in background) */
  RecacheAllMovieArtwork: Scalars["Int"]["output"];
  /** Recache artwork for a specific movie */
  RecacheMovieArtwork: Scalars["Boolean"]["output"];
  ReconnectLibraryPath: NetworkPathConfigPayload;
  /** Refresh a movie's metadata and artwork from TMDB. */
  RefreshMovie: MovieOperationResult;
  /** Refresh a show's metadata and artwork from TVMaze. */
  RefreshShow: TvShowOperationResult;
  RefreshToken: AuthPayload;
  Register: AuthPayload;
  /**
   * Re-run matching for files from a source.
   * Note: full matching pipeline from legacy code is being re-implemented.
   */
  RematchSource: RematchSourceResult;
  RemoveCastDevice: CastActionResult;
  /** Remove a torrent */
  RemoveTorrent: TorrentActionResult;
  RemoveTorrentByInfoHash: TorrentActionResult;
  RenameFile: FileOperationPayload;
  /** Resume a paused torrent */
  ResumeTorrent: TorrentActionResult;
  ResumeTorrentByInfoHash: TorrentActionResult;
  ScanLibrary: ScanLibraryResult;
  /** Test a source connection */
  TestSource: SourceTestConnectionResult;
  UnmatchMediaFile: UnmatchMediaFileResult;
  UpdateCastDevice: CastDeviceOperationResult;
  UpdateCastSettings: CastSettingsOperationResult;
  /** Update source priorities (reorder) */
  UpdateSourcePriorities: SourceMutationResult;
  /** Create a new #struct_name_str */
  createAlbum: AlbumResult;
  /** Create a new #struct_name_str */
  createAppLog: AppLogResult;
  /** Create a new #struct_name_str */
  createAppSetting: AppSettingResult;
  /** Create a new #struct_name_str */
  createArtist: ArtistResult;
  /** Create a new #struct_name_str */
  createArtworkCache: ArtworkCacheResult;
  /** Create a new #struct_name_str */
  createAudioStream: AudioStreamResult;
  /** Create a new #struct_name_str */
  createAudiobook: AudiobookResult;
  /** Create a new #struct_name_str */
  createCastDevice: CastDeviceResult;
  /** Create a new #struct_name_str */
  createCastSession: CastSessionResult;
  /** Create a new #struct_name_str */
  createCastSetting: CastSettingResult;
  /** Create a new #struct_name_str */
  createChapter: ChapterResult;
  /** Create a new #struct_name_str */
  createCollection: CollectionResult;
  /** Create a new #struct_name_str */
  createEpisode: EpisodeResult;
  /** Create a new #struct_name_str */
  createInviteToken: InviteTokenResult;
  /** Create a new #struct_name_str */
  createLibrary: LibraryResult;
  /** Create a new #struct_name_str */
  createMediaChapter: MediaChapterResult;
  /** Create a new #struct_name_str */
  createMediaFile: MediaFileResult;
  /** Create a new #struct_name_str */
  createMetadataCache: MetadataCacheResult;
  /** Create a new #struct_name_str */
  createMovie: MovieResult;
  /** Create a new #struct_name_str */
  createMovieCastCredit: MovieCastCreditResult;
  /** Create a new #struct_name_str */
  createNamingPattern: NamingPatternResult;
  /** Create a new #struct_name_str */
  createNotification: NotificationResult;
  /** Create a new #struct_name_str */
  createPendingFileMatch: PendingFileMatchResult;
  /** Create a new #struct_name_str */
  createPerson: PersonResult;
  /** Create a new #struct_name_str */
  createPlaybackProgress: PlaybackProgressResult;
  /** Create a new #struct_name_str */
  createPlaybackSession: PlaybackSessionResult;
  /** Create a new #struct_name_str */
  createRefreshToken: RefreshTokenResult;
  /** Create a new #struct_name_str */
  createRssFeed: RssFeedResult;
  /** Create a new #struct_name_str */
  createRssFeedItem: RssFeedItemResult;
  /** Create a new #struct_name_str */
  createScheduleCache: ScheduleCacheResult;
  /** Create a new #struct_name_str */
  createScheduleSyncState: ScheduleSyncStateResult;
  /** Create a new #struct_name_str */
  createShow: ShowResult;
  /** Create a new #struct_name_str */
  createSource: SourceResult;
  /** Create a new #struct_name_str */
  createSourcePriorityRule: SourcePriorityRuleResult;
  /** Create a new #struct_name_str */
  createSubtitle: SubtitleResult;
  /** Create a new #struct_name_str */
  createTorrent: TorrentResult;
  /** Create a new #struct_name_str */
  createTorrentFile: TorrentFileResult;
  /** Create a new #struct_name_str */
  createTorznabCategory: TorznabCategoryResult;
  /** Create a new #struct_name_str */
  createTrack: TrackResult;
  /** Create a new #struct_name_str */
  createUsenetDownload: UsenetDownloadResult;
  /** Create a new #struct_name_str */
  createUsenetServer: UsenetServerResult;
  /** Create a new #struct_name_str */
  createUser: UserResult;
  /** Create a new #struct_name_str */
  createVideoStream: VideoStreamResult;
  /** Delete a #struct_name_str */
  deleteAlbum: AlbumResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteAlbums: DeleteAlbumsResult;
  /** Delete a #struct_name_str */
  deleteAppLog: AppLogResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteAppLogs: DeleteAppLogsResult;
  /** Delete a #struct_name_str */
  deleteAppSetting: AppSettingResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteAppSettings: DeleteAppSettingsResult;
  /** Delete a #struct_name_str */
  deleteArtist: ArtistResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteArtists: DeleteArtistsResult;
  /** Delete a #struct_name_str */
  deleteArtworkCache: ArtworkCacheResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteArtworkCaches: DeleteArtworkCachesResult;
  /** Delete a #struct_name_str */
  deleteAudioStream: AudioStreamResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteAudioStreams: DeleteAudioStreamsResult;
  /** Delete a #struct_name_str */
  deleteAudiobook: AudiobookResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteAudiobooks: DeleteAudiobooksResult;
  /** Delete a #struct_name_str */
  deleteCastDevice: CastDeviceResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteCastDevices: DeleteCastDevicesResult;
  /** Delete a #struct_name_str */
  deleteCastSession: CastSessionResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteCastSessions: DeleteCastSessionsResult;
  /** Delete a #struct_name_str */
  deleteCastSetting: CastSettingResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteCastSettings: DeleteCastSettingsResult;
  /** Delete a #struct_name_str */
  deleteChapter: ChapterResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteChapters: DeleteChaptersResult;
  /** Delete a #struct_name_str */
  deleteCollection: CollectionResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteCollections: DeleteCollectionsResult;
  /** Delete a #struct_name_str */
  deleteEpisode: EpisodeResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteEpisodes: DeleteEpisodesResult;
  /** Delete a #struct_name_str */
  deleteInviteToken: InviteTokenResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteInviteTokens: DeleteInviteTokensResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteLibraries: DeleteLibrariesResult;
  /** Delete a #struct_name_str */
  deleteLibrary: LibraryResult;
  /** Delete a #struct_name_str */
  deleteMediaChapter: MediaChapterResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteMediaChapters: DeleteMediaChaptersResult;
  /** Delete a #struct_name_str */
  deleteMediaFile: MediaFileResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteMediaFiles: DeleteMediaFilesResult;
  /** Delete a #struct_name_str */
  deleteMetadataCache: MetadataCacheResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteMetadataCaches: DeleteMetadataCachesResult;
  /** Delete a #struct_name_str */
  deleteMovie: MovieResult;
  /** Delete a #struct_name_str */
  deleteMovieCastCredit: MovieCastCreditResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteMovieCastCredits: DeleteMovieCastCreditsResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteMovies: DeleteMoviesResult;
  /** Delete a #struct_name_str */
  deleteNamingPattern: NamingPatternResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteNamingPatterns: DeleteNamingPatternsResult;
  /** Delete a #struct_name_str */
  deleteNotification: NotificationResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteNotifications: DeleteNotificationsResult;
  /** Delete a #struct_name_str */
  deletePendingFileMatch: PendingFileMatchResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deletePendingFileMatches: DeletePendingFileMatchesResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deletePeople: DeletePeopleResult;
  /** Delete a #struct_name_str */
  deletePerson: PersonResult;
  /** Delete a #struct_name_str */
  deletePlaybackProgress: PlaybackProgressResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deletePlaybackProgresses: DeletePlaybackProgressesResult;
  /** Delete a #struct_name_str */
  deletePlaybackSession: PlaybackSessionResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deletePlaybackSessions: DeletePlaybackSessionsResult;
  /** Delete a #struct_name_str */
  deleteRefreshToken: RefreshTokenResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteRefreshTokens: DeleteRefreshTokensResult;
  /** Delete a #struct_name_str */
  deleteRssFeed: RssFeedResult;
  /** Delete a #struct_name_str */
  deleteRssFeedItem: RssFeedItemResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteRssFeedItems: DeleteRssFeedItemsResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteRssFeeds: DeleteRssFeedsResult;
  /** Delete a #struct_name_str */
  deleteScheduleCache: ScheduleCacheResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteScheduleCaches: DeleteScheduleCachesResult;
  /** Delete a #struct_name_str */
  deleteScheduleSyncState: ScheduleSyncStateResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteScheduleSyncStates: DeleteScheduleSyncStatesResult;
  /** Delete a #struct_name_str */
  deleteShow: ShowResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteShows: DeleteShowsResult;
  /** Delete a #struct_name_str */
  deleteSource: SourceResult;
  /** Delete a #struct_name_str */
  deleteSourcePriorityRule: SourcePriorityRuleResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteSourcePriorityRules: DeleteSourcePriorityRulesResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteSources: DeleteSourcesResult;
  /** Delete a #struct_name_str */
  deleteSubtitle: SubtitleResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteSubtitles: DeleteSubtitlesResult;
  /** Delete a #struct_name_str */
  deleteTorrent: TorrentResult;
  /** Delete a #struct_name_str */
  deleteTorrentFile: TorrentFileResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteTorrentFiles: DeleteTorrentFilesResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteTorrents: DeleteTorrentsResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteTorznabCategories: DeleteTorznabCategoriesResult;
  /** Delete a #struct_name_str */
  deleteTorznabCategory: TorznabCategoryResult;
  /** Delete a #struct_name_str */
  deleteTrack: TrackResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteTracks: DeleteTracksResult;
  /** Delete a #struct_name_str */
  deleteUsenetDownload: UsenetDownloadResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteUsenetDownloads: DeleteUsenetDownloadsResult;
  /** Delete a #struct_name_str */
  deleteUsenetServer: UsenetServerResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteUsenetServers: DeleteUsenetServersResult;
  /** Delete a #struct_name_str */
  deleteUser: UserResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteUsers: DeleteUsersResult;
  /** Delete a #struct_name_str */
  deleteVideoStream: VideoStreamResult;
  /** Delete multiple #plural_name matching the given Where filter */
  deleteVideoStreams: DeleteVideoStreamsResult;
  /** Update an existing #struct_name_str */
  updateAlbum: AlbumResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateAlbums: UpdateAlbumsResult;
  /** Update an existing #struct_name_str */
  updateAppLog: AppLogResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateAppLogs: UpdateAppLogsResult;
  /** Update an existing #struct_name_str */
  updateAppSetting: AppSettingResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateAppSettings: UpdateAppSettingsResult;
  /** Update an existing #struct_name_str */
  updateArtist: ArtistResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateArtists: UpdateArtistsResult;
  /** Update an existing #struct_name_str */
  updateArtworkCache: ArtworkCacheResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateArtworkCaches: UpdateArtworkCachesResult;
  /** Update an existing #struct_name_str */
  updateAudioStream: AudioStreamResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateAudioStreams: UpdateAudioStreamsResult;
  /** Update an existing #struct_name_str */
  updateAudiobook: AudiobookResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateAudiobooks: UpdateAudiobooksResult;
  /** Update an existing #struct_name_str */
  updateCastDevice: CastDeviceResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateCastDevices: UpdateCastDevicesResult;
  /** Update an existing #struct_name_str */
  updateCastSession: CastSessionResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateCastSessions: UpdateCastSessionsResult;
  /** Update an existing #struct_name_str */
  updateCastSetting: CastSettingResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateCastSettings: UpdateCastSettingsResult;
  /** Update an existing #struct_name_str */
  updateChapter: ChapterResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateChapters: UpdateChaptersResult;
  /** Update an existing #struct_name_str */
  updateCollection: CollectionResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateCollections: UpdateCollectionsResult;
  /** Update an existing #struct_name_str */
  updateEpisode: EpisodeResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateEpisodes: UpdateEpisodesResult;
  /** Update an existing #struct_name_str */
  updateInviteToken: InviteTokenResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateInviteTokens: UpdateInviteTokensResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateLibraries: UpdateLibrariesResult;
  /** Update an existing #struct_name_str */
  updateLibrary: LibraryResult;
  /** Update an existing #struct_name_str */
  updateMediaChapter: MediaChapterResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateMediaChapters: UpdateMediaChaptersResult;
  /** Update an existing #struct_name_str */
  updateMediaFile: MediaFileResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateMediaFiles: UpdateMediaFilesResult;
  /** Update an existing #struct_name_str */
  updateMetadataCache: MetadataCacheResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateMetadataCaches: UpdateMetadataCachesResult;
  /** Update an existing #struct_name_str */
  updateMovie: MovieResult;
  /** Update an existing #struct_name_str */
  updateMovieCastCredit: MovieCastCreditResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateMovieCastCredits: UpdateMovieCastCreditsResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateMovies: UpdateMoviesResult;
  /** Update an existing #struct_name_str */
  updateNamingPattern: NamingPatternResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateNamingPatterns: UpdateNamingPatternsResult;
  /** Update an existing #struct_name_str */
  updateNotification: NotificationResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateNotifications: UpdateNotificationsResult;
  /** Update an existing #struct_name_str */
  updatePendingFileMatch: PendingFileMatchResult;
  /** Update multiple #plural_name matching the given Where filter */
  updatePendingFileMatches: UpdatePendingFileMatchesResult;
  /** Update multiple #plural_name matching the given Where filter */
  updatePeople: UpdatePeopleResult;
  /** Update an existing #struct_name_str */
  updatePerson: PersonResult;
  /** Update an existing #struct_name_str */
  updatePlaybackProgress: PlaybackProgressResult;
  /** Update multiple #plural_name matching the given Where filter */
  updatePlaybackProgresses: UpdatePlaybackProgressesResult;
  /** Update an existing #struct_name_str */
  updatePlaybackSession: PlaybackSessionResult;
  /** Update multiple #plural_name matching the given Where filter */
  updatePlaybackSessions: UpdatePlaybackSessionsResult;
  /** Update an existing #struct_name_str */
  updateRefreshToken: RefreshTokenResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateRefreshTokens: UpdateRefreshTokensResult;
  /** Update an existing #struct_name_str */
  updateRssFeed: RssFeedResult;
  /** Update an existing #struct_name_str */
  updateRssFeedItem: RssFeedItemResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateRssFeedItems: UpdateRssFeedItemsResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateRssFeeds: UpdateRssFeedsResult;
  /** Update an existing #struct_name_str */
  updateScheduleCache: ScheduleCacheResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateScheduleCaches: UpdateScheduleCachesResult;
  /** Update an existing #struct_name_str */
  updateScheduleSyncState: ScheduleSyncStateResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateScheduleSyncStates: UpdateScheduleSyncStatesResult;
  /** Update an existing #struct_name_str */
  updateShow: ShowResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateShows: UpdateShowsResult;
  /** Update an existing #struct_name_str */
  updateSource: SourceResult;
  /** Update an existing #struct_name_str */
  updateSourcePriorityRule: SourcePriorityRuleResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateSourcePriorityRules: UpdateSourcePriorityRulesResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateSources: UpdateSourcesResult;
  /** Update an existing #struct_name_str */
  updateSubtitle: SubtitleResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateSubtitles: UpdateSubtitlesResult;
  /** Update an existing #struct_name_str */
  updateTorrent: TorrentResult;
  /** Update an existing #struct_name_str */
  updateTorrentFile: TorrentFileResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateTorrentFiles: UpdateTorrentFilesResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateTorrents: UpdateTorrentsResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateTorznabCategories: UpdateTorznabCategoriesResult;
  /** Update an existing #struct_name_str */
  updateTorznabCategory: TorznabCategoryResult;
  /** Update an existing #struct_name_str */
  updateTrack: TrackResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateTracks: UpdateTracksResult;
  /** Update an existing #struct_name_str */
  updateUsenetDownload: UsenetDownloadResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateUsenetDownloads: UpdateUsenetDownloadsResult;
  /** Update an existing #struct_name_str */
  updateUsenetServer: UsenetServerResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateUsenetServers: UpdateUsenetServersResult;
  /** Update an existing #struct_name_str */
  updateUser: UserResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateUsers: UpdateUsersResult;
  /** Update an existing #struct_name_str */
  updateVideoStream: VideoStreamResult;
  /** Update multiple #plural_name matching the given Where filter */
  updateVideoStreams: UpdateVideoStreamsResult;
};

export type MutationRootAddAlbumArgs = {
  Input: AddAlbumInput;
};

export type MutationRootAddAudiobookArgs = {
  Input: AddAudiobookInput;
};

export type MutationRootAddCastDeviceArgs = {
  input: LegacyAddCastDeviceInput;
};

export type MutationRootAddMovieArgs = {
  Input: AddMovieInput;
  LibraryId: Scalars["String"]["input"];
};

export type MutationRootAddMovieCollectionArgs = {
  Input: AddMovieCollectionInput;
  LibraryId: Scalars["String"]["input"];
};

export type MutationRootAddTorrentArgs = {
  Input: AddTorrentInput;
};

export type MutationRootAddTvShowArgs = {
  Input: AddTvShowInput;
  LibraryId: Scalars["String"]["input"];
};

export type MutationRootAnalyzeMediaFileArgs = {
  MediaFileId: Scalars["String"]["input"];
  Path: Scalars["String"]["input"];
};

export type MutationRootCastMediaArgs = {
  input: CastMediaInput;
};

export type MutationRootCastPauseArgs = {
  sessionId: Scalars["String"]["input"];
};

export type MutationRootCastPlayArgs = {
  sessionId: Scalars["String"]["input"];
};

export type MutationRootCastSeekArgs = {
  position: Scalars["Float"]["input"];
  sessionId: Scalars["String"]["input"];
};

export type MutationRootCastSetMutedArgs = {
  muted: Scalars["Boolean"]["input"];
  sessionId: Scalars["String"]["input"];
};

export type MutationRootCastSetVolumeArgs = {
  sessionId: Scalars["String"]["input"];
  volume: Scalars["Float"]["input"];
};

export type MutationRootCastStopArgs = {
  sessionId: Scalars["String"]["input"];
};

export type MutationRootConfigureNetworkPathArgs = {
  Input: ConfigureNetworkPathInput;
};

export type MutationRootCopyFilesArgs = {
  Input: CopyFilesInput;
};

export type MutationRootCreateDirectoryArgs = {
  Input: CreateDirectoryInput;
};

export type MutationRootDeleteFilesArgs = {
  Input: DeleteFilesInput;
};

export type MutationRootLoginArgs = {
  Input: LoginInput;
};

export type MutationRootLogoutArgs = {
  Input: LogoutInput;
};

export type MutationRootMatchMediaFileArgs = {
  Input: MatchMediaFileInput;
};

export type MutationRootMoveFilesArgs = {
  Input: MoveFilesInput;
};

export type MutationRootOrganizeMediaFileArgs = {
  Input: OrganizeMediaFileInput;
};

export type MutationRootPauseTorrentArgs = {
  Id: Scalars["Int"]["input"];
};

export type MutationRootPauseTorrentByInfoHashArgs = {
  InfoHash: Scalars["String"]["input"];
};

export type MutationRootProcessSourceArgs = {
  SourceId: Scalars["String"]["input"];
  SourceType: Scalars["String"]["input"];
};

export type MutationRootRecacheMovieArtworkArgs = {
  MovieId: Scalars["String"]["input"];
};

export type MutationRootReconnectLibraryPathArgs = {
  Path: Scalars["String"]["input"];
};

export type MutationRootRefreshMovieArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootRefreshShowArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootRefreshTokenArgs = {
  Input: RefreshTokenInput;
};

export type MutationRootRegisterArgs = {
  Input: RegisterUserInput;
};

export type MutationRootRematchSourceArgs = {
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  SourceId: Scalars["String"]["input"];
  SourceType: Scalars["String"]["input"];
};

export type MutationRootRemoveCastDeviceArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootRemoveTorrentArgs = {
  DeleteFiles?: Scalars["Boolean"]["input"];
  Id: Scalars["Int"]["input"];
};

export type MutationRootRemoveTorrentByInfoHashArgs = {
  DeleteFiles?: Scalars["Boolean"]["input"];
  InfoHash: Scalars["String"]["input"];
};

export type MutationRootRenameFileArgs = {
  Input: RenameFileInput;
};

export type MutationRootResumeTorrentArgs = {
  Id: Scalars["Int"]["input"];
};

export type MutationRootResumeTorrentByInfoHashArgs = {
  InfoHash: Scalars["String"]["input"];
};

export type MutationRootScanLibraryArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootTestSourceArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootUnmatchMediaFileArgs = {
  MediaFileId: Scalars["String"]["input"];
};

export type MutationRootUpdateCastDeviceArgs = {
  id: Scalars["String"]["input"];
  input: LegacyUpdateCastDeviceInput;
};

export type MutationRootUpdateCastSettingsArgs = {
  input: LegacyUpdateCastSettingsInput;
};

export type MutationRootUpdateSourcePrioritiesArgs = {
  Input: UpdateSourcePrioritiesInput;
};

export type MutationRootcreateAlbumArgs = {
  input: CreateAlbumInput;
};

export type MutationRootcreateAppLogArgs = {
  input: CreateAppLogInput;
};

export type MutationRootcreateAppSettingArgs = {
  input: CreateAppSettingInput;
};

export type MutationRootcreateArtistArgs = {
  input: CreateArtistInput;
};

export type MutationRootcreateArtworkCacheArgs = {
  input: CreateArtworkCacheInput;
};

export type MutationRootcreateAudioStreamArgs = {
  input: CreateAudioStreamInput;
};

export type MutationRootcreateAudiobookArgs = {
  input: CreateAudiobookInput;
};

export type MutationRootcreateCastDeviceArgs = {
  input: CreateCastDeviceInput;
};

export type MutationRootcreateCastSessionArgs = {
  input: CreateCastSessionInput;
};

export type MutationRootcreateCastSettingArgs = {
  input: CreateCastSettingInput;
};

export type MutationRootcreateChapterArgs = {
  input: CreateChapterInput;
};

export type MutationRootcreateCollectionArgs = {
  input: CreateCollectionInput;
};

export type MutationRootcreateEpisodeArgs = {
  input: CreateEpisodeInput;
};

export type MutationRootcreateInviteTokenArgs = {
  input: CreateInviteTokenInput;
};

export type MutationRootcreateLibraryArgs = {
  input: CreateLibraryInput;
};

export type MutationRootcreateMediaChapterArgs = {
  input: CreateMediaChapterInput;
};

export type MutationRootcreateMediaFileArgs = {
  input: CreateMediaFileInput;
};

export type MutationRootcreateMetadataCacheArgs = {
  input: CreateMetadataCacheInput;
};

export type MutationRootcreateMovieArgs = {
  input: CreateMovieInput;
};

export type MutationRootcreateMovieCastCreditArgs = {
  input: CreateMovieCastCreditInput;
};

export type MutationRootcreateNamingPatternArgs = {
  input: CreateNamingPatternInput;
};

export type MutationRootcreateNotificationArgs = {
  input: CreateNotificationInput;
};

export type MutationRootcreatePendingFileMatchArgs = {
  input: CreatePendingFileMatchInput;
};

export type MutationRootcreatePersonArgs = {
  input: CreatePersonInput;
};

export type MutationRootcreatePlaybackProgressArgs = {
  input: CreatePlaybackProgressInput;
};

export type MutationRootcreatePlaybackSessionArgs = {
  input: CreatePlaybackSessionInput;
};

export type MutationRootcreateRefreshTokenArgs = {
  input: CreateRefreshTokenInput;
};

export type MutationRootcreateRssFeedArgs = {
  input: CreateRssFeedInput;
};

export type MutationRootcreateRssFeedItemArgs = {
  input: CreateRssFeedItemInput;
};

export type MutationRootcreateScheduleCacheArgs = {
  input: CreateScheduleCacheInput;
};

export type MutationRootcreateScheduleSyncStateArgs = {
  input: CreateScheduleSyncStateInput;
};

export type MutationRootcreateShowArgs = {
  input: CreateShowInput;
};

export type MutationRootcreateSourceArgs = {
  input: CreateSourceInput;
};

export type MutationRootcreateSourcePriorityRuleArgs = {
  input: CreateSourcePriorityRuleInput;
};

export type MutationRootcreateSubtitleArgs = {
  input: CreateSubtitleInput;
};

export type MutationRootcreateTorrentArgs = {
  input: CreateTorrentInput;
};

export type MutationRootcreateTorrentFileArgs = {
  input: CreateTorrentFileInput;
};

export type MutationRootcreateTorznabCategoryArgs = {
  input: CreateTorznabCategoryInput;
};

export type MutationRootcreateTrackArgs = {
  input: CreateTrackInput;
};

export type MutationRootcreateUsenetDownloadArgs = {
  input: CreateUsenetDownloadInput;
};

export type MutationRootcreateUsenetServerArgs = {
  input: CreateUsenetServerInput;
};

export type MutationRootcreateUserArgs = {
  input: CreateUserInput;
};

export type MutationRootcreateVideoStreamArgs = {
  input: CreateVideoStreamInput;
};

export type MutationRootdeleteAlbumArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteAlbumsArgs = {
  where?: InputMaybe<AlbumWhereInput>;
};

export type MutationRootdeleteAppLogArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteAppLogsArgs = {
  where?: InputMaybe<AppLogWhereInput>;
};

export type MutationRootdeleteAppSettingArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteAppSettingsArgs = {
  where?: InputMaybe<AppSettingWhereInput>;
};

export type MutationRootdeleteArtistArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteArtistsArgs = {
  where?: InputMaybe<ArtistWhereInput>;
};

export type MutationRootdeleteArtworkCacheArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteArtworkCachesArgs = {
  where?: InputMaybe<ArtworkCacheWhereInput>;
};

export type MutationRootdeleteAudioStreamArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteAudioStreamsArgs = {
  where?: InputMaybe<AudioStreamWhereInput>;
};

export type MutationRootdeleteAudiobookArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteAudiobooksArgs = {
  where?: InputMaybe<AudiobookWhereInput>;
};

export type MutationRootdeleteCastDeviceArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteCastDevicesArgs = {
  where?: InputMaybe<CastDeviceWhereInput>;
};

export type MutationRootdeleteCastSessionArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteCastSessionsArgs = {
  where?: InputMaybe<CastSessionWhereInput>;
};

export type MutationRootdeleteCastSettingArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteCastSettingsArgs = {
  where?: InputMaybe<CastSettingWhereInput>;
};

export type MutationRootdeleteChapterArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteChaptersArgs = {
  where?: InputMaybe<ChapterWhereInput>;
};

export type MutationRootdeleteCollectionArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteCollectionsArgs = {
  where?: InputMaybe<CollectionWhereInput>;
};

export type MutationRootdeleteEpisodeArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteEpisodesArgs = {
  where?: InputMaybe<EpisodeWhereInput>;
};

export type MutationRootdeleteInviteTokenArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteInviteTokensArgs = {
  where?: InputMaybe<InviteTokenWhereInput>;
};

export type MutationRootdeleteLibrariesArgs = {
  where?: InputMaybe<LibraryWhereInput>;
};

export type MutationRootdeleteLibraryArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteMediaChapterArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteMediaChaptersArgs = {
  where?: InputMaybe<MediaChapterWhereInput>;
};

export type MutationRootdeleteMediaFileArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteMediaFilesArgs = {
  where?: InputMaybe<MediaFileWhereInput>;
};

export type MutationRootdeleteMetadataCacheArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteMetadataCachesArgs = {
  where?: InputMaybe<MetadataCacheWhereInput>;
};

export type MutationRootdeleteMovieArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteMovieCastCreditArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteMovieCastCreditsArgs = {
  where?: InputMaybe<MovieCastCreditWhereInput>;
};

export type MutationRootdeleteMoviesArgs = {
  where?: InputMaybe<MovieWhereInput>;
};

export type MutationRootdeleteNamingPatternArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteNamingPatternsArgs = {
  where?: InputMaybe<NamingPatternWhereInput>;
};

export type MutationRootdeleteNotificationArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteNotificationsArgs = {
  where?: InputMaybe<NotificationWhereInput>;
};

export type MutationRootdeletePendingFileMatchArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeletePendingFileMatchesArgs = {
  where?: InputMaybe<PendingFileMatchWhereInput>;
};

export type MutationRootdeletePeopleArgs = {
  where?: InputMaybe<PersonWhereInput>;
};

export type MutationRootdeletePersonArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeletePlaybackProgressArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeletePlaybackProgressesArgs = {
  where?: InputMaybe<PlaybackProgressWhereInput>;
};

export type MutationRootdeletePlaybackSessionArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeletePlaybackSessionsArgs = {
  where?: InputMaybe<PlaybackSessionWhereInput>;
};

export type MutationRootdeleteRefreshTokenArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteRefreshTokensArgs = {
  where?: InputMaybe<RefreshTokenWhereInput>;
};

export type MutationRootdeleteRssFeedArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteRssFeedItemArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteRssFeedItemsArgs = {
  where?: InputMaybe<RssFeedItemWhereInput>;
};

export type MutationRootdeleteRssFeedsArgs = {
  where?: InputMaybe<RssFeedWhereInput>;
};

export type MutationRootdeleteScheduleCacheArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteScheduleCachesArgs = {
  where?: InputMaybe<ScheduleCacheWhereInput>;
};

export type MutationRootdeleteScheduleSyncStateArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteScheduleSyncStatesArgs = {
  where?: InputMaybe<ScheduleSyncStateWhereInput>;
};

export type MutationRootdeleteShowArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteShowsArgs = {
  where?: InputMaybe<ShowWhereInput>;
};

export type MutationRootdeleteSourceArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteSourcePriorityRuleArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteSourcePriorityRulesArgs = {
  where?: InputMaybe<SourcePriorityRuleWhereInput>;
};

export type MutationRootdeleteSourcesArgs = {
  where?: InputMaybe<SourceWhereInput>;
};

export type MutationRootdeleteSubtitleArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteSubtitlesArgs = {
  where?: InputMaybe<SubtitleWhereInput>;
};

export type MutationRootdeleteTorrentArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteTorrentFileArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteTorrentFilesArgs = {
  where?: InputMaybe<TorrentFileWhereInput>;
};

export type MutationRootdeleteTorrentsArgs = {
  where?: InputMaybe<TorrentWhereInput>;
};

export type MutationRootdeleteTorznabCategoriesArgs = {
  where?: InputMaybe<TorznabCategoryWhereInput>;
};

export type MutationRootdeleteTorznabCategoryArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteTrackArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteTracksArgs = {
  where?: InputMaybe<TrackWhereInput>;
};

export type MutationRootdeleteUsenetDownloadArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteUsenetDownloadsArgs = {
  where?: InputMaybe<UsenetDownloadWhereInput>;
};

export type MutationRootdeleteUsenetServerArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteUsenetServersArgs = {
  where?: InputMaybe<UsenetServerWhereInput>;
};

export type MutationRootdeleteUserArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteUsersArgs = {
  where?: InputMaybe<UserWhereInput>;
};

export type MutationRootdeleteVideoStreamArgs = {
  id: Scalars["String"]["input"];
};

export type MutationRootdeleteVideoStreamsArgs = {
  where?: InputMaybe<VideoStreamWhereInput>;
};

export type MutationRootupdateAlbumArgs = {
  id: Scalars["String"]["input"];
  input: UpdateAlbumInput;
};

export type MutationRootupdateAlbumsArgs = {
  input: UpdateAlbumInput;
  where?: InputMaybe<AlbumWhereInput>;
};

export type MutationRootupdateAppLogArgs = {
  id: Scalars["String"]["input"];
  input: UpdateAppLogInput;
};

export type MutationRootupdateAppLogsArgs = {
  input: UpdateAppLogInput;
  where?: InputMaybe<AppLogWhereInput>;
};

export type MutationRootupdateAppSettingArgs = {
  id: Scalars["String"]["input"];
  input: UpdateAppSettingInput;
};

export type MutationRootupdateAppSettingsArgs = {
  input: UpdateAppSettingInput;
  where?: InputMaybe<AppSettingWhereInput>;
};

export type MutationRootupdateArtistArgs = {
  id: Scalars["String"]["input"];
  input: UpdateArtistInput;
};

export type MutationRootupdateArtistsArgs = {
  input: UpdateArtistInput;
  where?: InputMaybe<ArtistWhereInput>;
};

export type MutationRootupdateArtworkCacheArgs = {
  id: Scalars["String"]["input"];
  input: UpdateArtworkCacheInput;
};

export type MutationRootupdateArtworkCachesArgs = {
  input: UpdateArtworkCacheInput;
  where?: InputMaybe<ArtworkCacheWhereInput>;
};

export type MutationRootupdateAudioStreamArgs = {
  id: Scalars["String"]["input"];
  input: UpdateAudioStreamInput;
};

export type MutationRootupdateAudioStreamsArgs = {
  input: UpdateAudioStreamInput;
  where?: InputMaybe<AudioStreamWhereInput>;
};

export type MutationRootupdateAudiobookArgs = {
  id: Scalars["String"]["input"];
  input: UpdateAudiobookInput;
};

export type MutationRootupdateAudiobooksArgs = {
  input: UpdateAudiobookInput;
  where?: InputMaybe<AudiobookWhereInput>;
};

export type MutationRootupdateCastDeviceArgs = {
  id: Scalars["String"]["input"];
  input: UpdateCastDeviceInput;
};

export type MutationRootupdateCastDevicesArgs = {
  input: UpdateCastDeviceInput;
  where?: InputMaybe<CastDeviceWhereInput>;
};

export type MutationRootupdateCastSessionArgs = {
  id: Scalars["String"]["input"];
  input: UpdateCastSessionInput;
};

export type MutationRootupdateCastSessionsArgs = {
  input: UpdateCastSessionInput;
  where?: InputMaybe<CastSessionWhereInput>;
};

export type MutationRootupdateCastSettingArgs = {
  id: Scalars["String"]["input"];
  input: UpdateCastSettingInput;
};

export type MutationRootupdateCastSettingsArgs = {
  input: UpdateCastSettingInput;
  where?: InputMaybe<CastSettingWhereInput>;
};

export type MutationRootupdateChapterArgs = {
  id: Scalars["String"]["input"];
  input: UpdateChapterInput;
};

export type MutationRootupdateChaptersArgs = {
  input: UpdateChapterInput;
  where?: InputMaybe<ChapterWhereInput>;
};

export type MutationRootupdateCollectionArgs = {
  id: Scalars["String"]["input"];
  input: UpdateCollectionInput;
};

export type MutationRootupdateCollectionsArgs = {
  input: UpdateCollectionInput;
  where?: InputMaybe<CollectionWhereInput>;
};

export type MutationRootupdateEpisodeArgs = {
  id: Scalars["String"]["input"];
  input: UpdateEpisodeInput;
};

export type MutationRootupdateEpisodesArgs = {
  input: UpdateEpisodeInput;
  where?: InputMaybe<EpisodeWhereInput>;
};

export type MutationRootupdateInviteTokenArgs = {
  id: Scalars["String"]["input"];
  input: UpdateInviteTokenInput;
};

export type MutationRootupdateInviteTokensArgs = {
  input: UpdateInviteTokenInput;
  where?: InputMaybe<InviteTokenWhereInput>;
};

export type MutationRootupdateLibrariesArgs = {
  input: UpdateLibraryInput;
  where?: InputMaybe<LibraryWhereInput>;
};

export type MutationRootupdateLibraryArgs = {
  id: Scalars["String"]["input"];
  input: UpdateLibraryInput;
};

export type MutationRootupdateMediaChapterArgs = {
  id: Scalars["String"]["input"];
  input: UpdateMediaChapterInput;
};

export type MutationRootupdateMediaChaptersArgs = {
  input: UpdateMediaChapterInput;
  where?: InputMaybe<MediaChapterWhereInput>;
};

export type MutationRootupdateMediaFileArgs = {
  id: Scalars["String"]["input"];
  input: UpdateMediaFileInput;
};

export type MutationRootupdateMediaFilesArgs = {
  input: UpdateMediaFileInput;
  where?: InputMaybe<MediaFileWhereInput>;
};

export type MutationRootupdateMetadataCacheArgs = {
  id: Scalars["String"]["input"];
  input: UpdateMetadataCacheInput;
};

export type MutationRootupdateMetadataCachesArgs = {
  input: UpdateMetadataCacheInput;
  where?: InputMaybe<MetadataCacheWhereInput>;
};

export type MutationRootupdateMovieArgs = {
  id: Scalars["String"]["input"];
  input: UpdateMovieInput;
};

export type MutationRootupdateMovieCastCreditArgs = {
  id: Scalars["String"]["input"];
  input: UpdateMovieCastCreditInput;
};

export type MutationRootupdateMovieCastCreditsArgs = {
  input: UpdateMovieCastCreditInput;
  where?: InputMaybe<MovieCastCreditWhereInput>;
};

export type MutationRootupdateMoviesArgs = {
  input: UpdateMovieInput;
  where?: InputMaybe<MovieWhereInput>;
};

export type MutationRootupdateNamingPatternArgs = {
  id: Scalars["String"]["input"];
  input: UpdateNamingPatternInput;
};

export type MutationRootupdateNamingPatternsArgs = {
  input: UpdateNamingPatternInput;
  where?: InputMaybe<NamingPatternWhereInput>;
};

export type MutationRootupdateNotificationArgs = {
  id: Scalars["String"]["input"];
  input: UpdateNotificationInput;
};

export type MutationRootupdateNotificationsArgs = {
  input: UpdateNotificationInput;
  where?: InputMaybe<NotificationWhereInput>;
};

export type MutationRootupdatePendingFileMatchArgs = {
  id: Scalars["String"]["input"];
  input: UpdatePendingFileMatchInput;
};

export type MutationRootupdatePendingFileMatchesArgs = {
  input: UpdatePendingFileMatchInput;
  where?: InputMaybe<PendingFileMatchWhereInput>;
};

export type MutationRootupdatePeopleArgs = {
  input: UpdatePersonInput;
  where?: InputMaybe<PersonWhereInput>;
};

export type MutationRootupdatePersonArgs = {
  id: Scalars["String"]["input"];
  input: UpdatePersonInput;
};

export type MutationRootupdatePlaybackProgressArgs = {
  id: Scalars["String"]["input"];
  input: UpdatePlaybackProgressInput;
};

export type MutationRootupdatePlaybackProgressesArgs = {
  input: UpdatePlaybackProgressInput;
  where?: InputMaybe<PlaybackProgressWhereInput>;
};

export type MutationRootupdatePlaybackSessionArgs = {
  id: Scalars["String"]["input"];
  input: UpdatePlaybackSessionInput;
};

export type MutationRootupdatePlaybackSessionsArgs = {
  input: UpdatePlaybackSessionInput;
  where?: InputMaybe<PlaybackSessionWhereInput>;
};

export type MutationRootupdateRefreshTokenArgs = {
  id: Scalars["String"]["input"];
  input: UpdateRefreshTokenInput;
};

export type MutationRootupdateRefreshTokensArgs = {
  input: UpdateRefreshTokenInput;
  where?: InputMaybe<RefreshTokenWhereInput>;
};

export type MutationRootupdateRssFeedArgs = {
  id: Scalars["String"]["input"];
  input: UpdateRssFeedInput;
};

export type MutationRootupdateRssFeedItemArgs = {
  id: Scalars["String"]["input"];
  input: UpdateRssFeedItemInput;
};

export type MutationRootupdateRssFeedItemsArgs = {
  input: UpdateRssFeedItemInput;
  where?: InputMaybe<RssFeedItemWhereInput>;
};

export type MutationRootupdateRssFeedsArgs = {
  input: UpdateRssFeedInput;
  where?: InputMaybe<RssFeedWhereInput>;
};

export type MutationRootupdateScheduleCacheArgs = {
  id: Scalars["String"]["input"];
  input: UpdateScheduleCacheInput;
};

export type MutationRootupdateScheduleCachesArgs = {
  input: UpdateScheduleCacheInput;
  where?: InputMaybe<ScheduleCacheWhereInput>;
};

export type MutationRootupdateScheduleSyncStateArgs = {
  id: Scalars["String"]["input"];
  input: UpdateScheduleSyncStateInput;
};

export type MutationRootupdateScheduleSyncStatesArgs = {
  input: UpdateScheduleSyncStateInput;
  where?: InputMaybe<ScheduleSyncStateWhereInput>;
};

export type MutationRootupdateShowArgs = {
  id: Scalars["String"]["input"];
  input: UpdateShowInput;
};

export type MutationRootupdateShowsArgs = {
  input: UpdateShowInput;
  where?: InputMaybe<ShowWhereInput>;
};

export type MutationRootupdateSourceArgs = {
  id: Scalars["String"]["input"];
  input: UpdateSourceInput;
};

export type MutationRootupdateSourcePriorityRuleArgs = {
  id: Scalars["String"]["input"];
  input: UpdateSourcePriorityRuleInput;
};

export type MutationRootupdateSourcePriorityRulesArgs = {
  input: UpdateSourcePriorityRuleInput;
  where?: InputMaybe<SourcePriorityRuleWhereInput>;
};

export type MutationRootupdateSourcesArgs = {
  input: UpdateSourceInput;
  where?: InputMaybe<SourceWhereInput>;
};

export type MutationRootupdateSubtitleArgs = {
  id: Scalars["String"]["input"];
  input: UpdateSubtitleInput;
};

export type MutationRootupdateSubtitlesArgs = {
  input: UpdateSubtitleInput;
  where?: InputMaybe<SubtitleWhereInput>;
};

export type MutationRootupdateTorrentArgs = {
  id: Scalars["String"]["input"];
  input: UpdateTorrentInput;
};

export type MutationRootupdateTorrentFileArgs = {
  id: Scalars["String"]["input"];
  input: UpdateTorrentFileInput;
};

export type MutationRootupdateTorrentFilesArgs = {
  input: UpdateTorrentFileInput;
  where?: InputMaybe<TorrentFileWhereInput>;
};

export type MutationRootupdateTorrentsArgs = {
  input: UpdateTorrentInput;
  where?: InputMaybe<TorrentWhereInput>;
};

export type MutationRootupdateTorznabCategoriesArgs = {
  input: UpdateTorznabCategoryInput;
  where?: InputMaybe<TorznabCategoryWhereInput>;
};

export type MutationRootupdateTorznabCategoryArgs = {
  id: Scalars["String"]["input"];
  input: UpdateTorznabCategoryInput;
};

export type MutationRootupdateTrackArgs = {
  id: Scalars["String"]["input"];
  input: UpdateTrackInput;
};

export type MutationRootupdateTracksArgs = {
  input: UpdateTrackInput;
  where?: InputMaybe<TrackWhereInput>;
};

export type MutationRootupdateUsenetDownloadArgs = {
  id: Scalars["String"]["input"];
  input: UpdateUsenetDownloadInput;
};

export type MutationRootupdateUsenetDownloadsArgs = {
  input: UpdateUsenetDownloadInput;
  where?: InputMaybe<UsenetDownloadWhereInput>;
};

export type MutationRootupdateUsenetServerArgs = {
  id: Scalars["String"]["input"];
  input: UpdateUsenetServerInput;
};

export type MutationRootupdateUsenetServersArgs = {
  input: UpdateUsenetServerInput;
  where?: InputMaybe<UsenetServerWhereInput>;
};

export type MutationRootupdateUserArgs = {
  id: Scalars["String"]["input"];
  input: UpdateUserInput;
};

export type MutationRootupdateUsersArgs = {
  input: UpdateUserInput;
  where?: InputMaybe<UserWhereInput>;
};

export type MutationRootupdateVideoStreamArgs = {
  id: Scalars["String"]["input"];
  input: UpdateVideoStreamInput;
};

export type MutationRootupdateVideoStreamsArgs = {
  input: UpdateVideoStreamInput;
  where?: InputMaybe<VideoStreamWhereInput>;
};

export type NamingPattern = {
  CreatedAt: Scalars["String"]["output"];
  Description?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  IsDefault: Scalars["Boolean"]["output"];
  IsSystem: Scalars["Boolean"]["output"];
  LibraryType: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  Pattern: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type NamingPatternChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  namingPattern?: Maybe<NamingPattern>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type NamingPatternConnection = {
  /** The edges in this connection */
  edges: Array<NamingPatternEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type NamingPatternEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: NamingPattern;
};

export type NamingPatternOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  LibraryType?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type NamingPatternResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  namingPattern?: Maybe<NamingPattern>;
  success: Scalars["Boolean"]["output"];
};

export type NamingPatternWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IsDefault?: InputMaybe<BoolFilter>;
  IsSystem?: InputMaybe<BoolFilter>;
  LibraryType?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<NamingPatternWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<NamingPatternWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<NamingPatternWhereInput>>;
};

export type NetworkPathConfigPayload = {
  Connected: Scalars["Boolean"]["output"];
  Error?: Maybe<Scalars["String"]["output"]>;
  Message?: Maybe<Scalars["String"]["output"]>;
  ResolvedPath: Scalars["String"]["output"];
  Stored: Scalars["Boolean"]["output"];
  Success: Scalars["Boolean"]["output"];
};

export type Notification = {
  ActionData?: Maybe<Scalars["String"]["output"]>;
  ActionType?: Maybe<Scalars["String"]["output"]>;
  Category: Scalars["String"]["output"];
  CreatedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  LibraryId?: Maybe<Scalars["String"]["output"]>;
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  Message: Scalars["String"]["output"];
  NotificationType: Scalars["String"]["output"];
  PendingMatchId?: Maybe<Scalars["String"]["output"]>;
  ReadAt?: Maybe<Scalars["String"]["output"]>;
  Resolution?: Maybe<Scalars["String"]["output"]>;
  ResolvedAt?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TorrentId?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type NotificationChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  notification?: Maybe<Notification>;
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type NotificationConnection = {
  /** The edges in this connection */
  edges: Array<NotificationEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type NotificationEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Notification;
};

export type NotificationOrderByInput = {
  Category?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  NotificationType?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type NotificationResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  notification?: Maybe<Notification>;
  success: Scalars["Boolean"]["output"];
};

export type NotificationWhereInput = {
  ActionType?: InputMaybe<StringFilter>;
  Category?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  NotificationType?: InputMaybe<StringFilter>;
  PendingMatchId?: InputMaybe<StringFilter>;
  ReadAt?: InputMaybe<DateFilter>;
  Resolution?: InputMaybe<StringFilter>;
  ResolvedAt?: InputMaybe<DateFilter>;
  Title?: InputMaybe<StringFilter>;
  TorrentId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<NotificationWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<NotificationWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<NotificationWhereInput>>;
};

export const OrderDirection = {
  ASC: "ASC",
  DESC: "DESC",
} as const;

export type OrderDirection =
  (typeof OrderDirection)[keyof typeof OrderDirection];
export type OrganizeMediaFileInput = {
  MediaFileId: Scalars["String"]["input"];
};

export type OrganizeMediaFileResult = {
  NewPath?: Maybe<Scalars["String"]["output"]>;
  OldPath?: Maybe<Scalars["String"]["output"]>;
  Reason?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type PageInfo = {
  endCursor?: Maybe<Scalars["String"]["output"]>;
  hasNextPage: Scalars["Boolean"]["output"];
  hasPreviousPage: Scalars["Boolean"]["output"];
  startCursor?: Maybe<Scalars["String"]["output"]>;
  totalCount?: Maybe<Scalars["Int"]["output"]>;
};

export type PageInput = {
  limit?: InputMaybe<Scalars["Int"]["input"]>;
  offset?: InputMaybe<Scalars["Int"]["input"]>;
};

export type PendingFileMatch = {
  ChapterId?: Maybe<Scalars["String"]["output"]>;
  CopiedAt?: Maybe<Scalars["String"]["output"]>;
  CopyAttempts: Scalars["Int"]["output"];
  CopyError?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  EpisodeId?: Maybe<Scalars["String"]["output"]>;
  FileSize: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  MatchAttempts: Scalars["Int"]["output"];
  MatchConfidence?: Maybe<Scalars["Float"]["output"]>;
  MatchType?: Maybe<Scalars["String"]["output"]>;
  MovieId?: Maybe<Scalars["String"]["output"]>;
  ParsedAudio?: Maybe<Scalars["String"]["output"]>;
  ParsedCodec?: Maybe<Scalars["String"]["output"]>;
  ParsedResolution?: Maybe<Scalars["String"]["output"]>;
  ParsedSource?: Maybe<Scalars["String"]["output"]>;
  SourceFileIndex?: Maybe<Scalars["Int"]["output"]>;
  SourceId?: Maybe<Scalars["String"]["output"]>;
  SourcePath: Scalars["String"]["output"];
  SourceType: Scalars["String"]["output"];
  TrackId?: Maybe<Scalars["String"]["output"]>;
  UnmatchedReason?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  VerificationReason?: Maybe<Scalars["String"]["output"]>;
  VerificationStatus?: Maybe<Scalars["String"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type PendingFileMatchChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  pendingFileMatch?: Maybe<PendingFileMatch>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type PendingFileMatchConnection = {
  /** The edges in this connection */
  edges: Array<PendingFileMatchEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type PendingFileMatchEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: PendingFileMatch;
};

export type PendingFileMatchOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  FileSize?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type PendingFileMatchResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  pendingFileMatch?: Maybe<PendingFileMatch>;
  success: Scalars["Boolean"]["output"];
};

export type PendingFileMatchWhereInput = {
  ChapterId?: InputMaybe<StringFilter>;
  CopiedAt?: InputMaybe<DateFilter>;
  CopyAttempts?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EpisodeId?: InputMaybe<StringFilter>;
  FileSize?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MatchAttempts?: InputMaybe<IntFilter>;
  MatchConfidence?: InputMaybe<IntFilter>;
  MatchType?: InputMaybe<StringFilter>;
  MovieId?: InputMaybe<StringFilter>;
  ParsedAudio?: InputMaybe<StringFilter>;
  ParsedCodec?: InputMaybe<StringFilter>;
  ParsedResolution?: InputMaybe<StringFilter>;
  ParsedSource?: InputMaybe<StringFilter>;
  SourceFileIndex?: InputMaybe<IntFilter>;
  SourceId?: InputMaybe<StringFilter>;
  SourcePath?: InputMaybe<StringFilter>;
  SourceType?: InputMaybe<StringFilter>;
  TrackId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  VerificationStatus?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<PendingFileMatchWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<PendingFileMatchWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<PendingFileMatchWhereInput>>;
};

export type Person = {
  CreatedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  ProfileUrl?: Maybe<Scalars["String"]["output"]>;
  TmdbPersonId: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type PersonChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  person?: Maybe<Person>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type PersonConnection = {
  /** The edges in this connection */
  edges: Array<PersonEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type PersonEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Person;
};

export type PersonOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type PersonResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  person?: Maybe<Person>;
  success: Scalars["Boolean"]["output"];
};

export type PersonWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  TmdbPersonId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<PersonWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<PersonWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<PersonWhereInput>>;
};

export type PlaybackProgress = {
  CreatedAt: Scalars["String"]["output"];
  CurrentPosition: Scalars["Float"]["output"];
  Duration?: Maybe<Scalars["Float"]["output"]>;
  Id: Scalars["String"]["output"];
  IsWatched: Scalars["Boolean"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  ProgressPercent: Scalars["Float"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  WatchedAt?: Maybe<Scalars["String"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type PlaybackProgressChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  playbackProgress?: Maybe<PlaybackProgress>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type PlaybackProgressConnection = {
  /** The edges in this connection */
  edges: Array<PlaybackProgressEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type PlaybackProgressEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: PlaybackProgress;
};

export type PlaybackProgressOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type PlaybackProgressResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  playbackProgress?: Maybe<PlaybackProgress>;
  success: Scalars["Boolean"]["output"];
};

export type PlaybackProgressWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  CurrentPosition?: InputMaybe<IntFilter>;
  Duration?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IsWatched?: InputMaybe<BoolFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  ProgressPercent?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  WatchedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<PlaybackProgressWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<PlaybackProgressWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<PlaybackProgressWhereInput>>;
};

export type PlaybackSession = {
  AlbumId?: Maybe<Scalars["String"]["output"]>;
  AudiobookId?: Maybe<Scalars["String"]["output"]>;
  CompletedAt?: Maybe<Scalars["String"]["output"]>;
  ContentType?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  CurrentPosition: Scalars["Float"]["output"];
  Duration?: Maybe<Scalars["Float"]["output"]>;
  EpisodeId?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  IsMuted: Scalars["Boolean"]["output"];
  IsPlaying: Scalars["Boolean"]["output"];
  LastUpdatedAt: Scalars["String"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  MovieId?: Maybe<Scalars["String"]["output"]>;
  StartedAt: Scalars["String"]["output"];
  TrackId?: Maybe<Scalars["String"]["output"]>;
  TvShowId?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  Volume: Scalars["Float"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type PlaybackSessionChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  playbackSession?: Maybe<PlaybackSession>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type PlaybackSessionConnection = {
  /** The edges in this connection */
  edges: Array<PlaybackSessionEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type PlaybackSessionEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: PlaybackSession;
};

export type PlaybackSessionOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  LastUpdatedAt?: InputMaybe<OrderDirection>;
  StartedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type PlaybackSessionResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  playbackSession?: Maybe<PlaybackSession>;
  success: Scalars["Boolean"]["output"];
};

export type PlaybackSessionWhereInput = {
  AlbumId?: InputMaybe<StringFilter>;
  AudiobookId?: InputMaybe<StringFilter>;
  CompletedAt?: InputMaybe<DateFilter>;
  ContentType?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  CurrentPosition?: InputMaybe<IntFilter>;
  Duration?: InputMaybe<IntFilter>;
  EpisodeId?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  IsMuted?: InputMaybe<BoolFilter>;
  IsPlaying?: InputMaybe<BoolFilter>;
  LastUpdatedAt?: InputMaybe<DateFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  MovieId?: InputMaybe<StringFilter>;
  StartedAt?: InputMaybe<DateFilter>;
  TrackId?: InputMaybe<StringFilter>;
  TvShowId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Volume?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<PlaybackSessionWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<PlaybackSessionWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<PlaybackSessionWhereInput>>;
};

/** Result of processing matched files from a source */
export type ProcessSourceResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  FilesFailed: Scalars["Int"]["output"];
  FilesProcessed: Scalars["Int"]["output"];
  Messages: Array<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type QueryRoot = {
  /** Count of active (downloading/checking) torrents */
  ActiveDownloadCount: Scalars["Int"]["output"];
  /** Get available source definitions (e.g., IPTorrents, Newznab, etc.) */
  AvailableSourceDefinitions: Array<SourceDefinitionInfo>;
  /** Browse a directory on the server. Requires authentication. */
  BrowseDirectory: BrowseDirectoryResult;
  FilesystemRuntimeInfo: FilesystemRuntimeInfo;
  LibraryPathAvailability: Array<LibraryPathAvailability>;
  /** Get a single live torrent by numeric id */
  LiveTorrent?: Maybe<LiveTorrent>;
  /** Get all torrents with live state from the torrent client */
  LiveTorrents: Array<LiveTorrent>;
  /** Current authenticated user (requires valid JWT). Returns null if not authenticated. */
  Me?: Maybe<MeUser>;
  /** Get full collection details from TMDB with library state overlay. */
  MovieCollectionDetails: MovieCollectionDetails;
  /** True if no admin user exists yet (first-time setup required). */
  NeedsSetup: Scalars["Boolean"]["output"];
  /** Search albums on MusicBrainz. */
  SearchAlbums: Array<AlbumSearchResult>;
  /** Search audiobooks on OpenLibrary. */
  SearchAudiobooks: Array<AudiobookSearchResult>;
  /** Search for movie collections on TMDB */
  SearchMovieCollections: Array<MovieCollectionSearchResult>;
  /** Search for movies on TMDB */
  SearchMovies: Array<MovieSearchResult>;
  /** Search across all enabled sources */
  SearchSources: SourceSearchResultSet;
  /** Search for TV shows on TVMaze */
  SearchTvShows: Array<TvShowSearchResult>;
  /** Get setting definitions for a source definition */
  SourceSettingDefinitions: Array<SourceSettingDefinition>;
  /** Get a single #struct_name_str by ID */
  album?: Maybe<Album>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  albums: AlbumConnection;
  /** Get a single #struct_name_str by ID */
  appLog?: Maybe<AppLog>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  appLogs: AppLogConnection;
  /** Get a single #struct_name_str by ID */
  appSetting?: Maybe<AppSetting>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  appSettings: AppSettingConnection;
  /** Get a single #struct_name_str by ID */
  artist?: Maybe<Artist>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  artists: ArtistConnection;
  /** Get a single #struct_name_str by ID */
  artworkCache?: Maybe<ArtworkCache>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  artworkCaches: ArtworkCacheConnection;
  /** Get a single #struct_name_str by ID */
  audioStream?: Maybe<AudioStream>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  audioStreams: AudioStreamConnection;
  /** Get a single #struct_name_str by ID */
  audiobook?: Maybe<Audiobook>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  audiobooks: AudiobookConnection;
  /** Get a single #struct_name_str by ID */
  castDevice?: Maybe<CastDevice>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  castDevices: CastDeviceConnection;
  /** Get a single #struct_name_str by ID */
  castSession?: Maybe<CastSession>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  castSessions: CastSessionConnection;
  /** Get a single #struct_name_str by ID */
  castSetting?: Maybe<CastSetting>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  castSettings: CastSettingConnection;
  /** Get a single #struct_name_str by ID */
  chapter?: Maybe<Chapter>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  chapters: ChapterConnection;
  /** Get a single #struct_name_str by ID */
  collection?: Maybe<Collection>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  collections: CollectionConnection;
  /** Get a single #struct_name_str by ID */
  episode?: Maybe<Episode>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  episodes: EpisodeConnection;
  /** Get a single #struct_name_str by ID */
  inviteToken?: Maybe<InviteToken>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  inviteTokens: InviteTokenConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  libraries: LibraryConnection;
  /** Get a single #struct_name_str by ID */
  library?: Maybe<Library>;
  /** Get a single #struct_name_str by ID */
  mediaChapter?: Maybe<MediaChapter>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  mediaChapters: MediaChapterConnection;
  /** Get a single #struct_name_str by ID */
  mediaFile?: Maybe<MediaFile>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  mediaFiles: MediaFileConnection;
  /** Get a single #struct_name_str by ID */
  metadataCache?: Maybe<MetadataCache>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  metadataCaches: MetadataCacheConnection;
  /** Get a single #struct_name_str by ID */
  movie?: Maybe<Movie>;
  /** Get a single #struct_name_str by ID */
  movieCastCredit?: Maybe<MovieCastCredit>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  movieCastCredits: MovieCastCreditConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  movies: MovieConnection;
  /** Get a single #struct_name_str by ID */
  namingPattern?: Maybe<NamingPattern>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  namingPatterns: NamingPatternConnection;
  /** Get a single #struct_name_str by ID */
  notification?: Maybe<Notification>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  notifications: NotificationConnection;
  /** Get a single #struct_name_str by ID */
  pendingFileMatch?: Maybe<PendingFileMatch>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  pendingFileMatches: PendingFileMatchConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  people: PersonConnection;
  /** Get a single #struct_name_str by ID */
  person?: Maybe<Person>;
  /** Get a single #struct_name_str by ID */
  playbackProgress?: Maybe<PlaybackProgress>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  playbackProgresses: PlaybackProgressConnection;
  /** Get a single #struct_name_str by ID */
  playbackSession?: Maybe<PlaybackSession>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  playbackSessions: PlaybackSessionConnection;
  /** Get a single #struct_name_str by ID */
  refreshToken?: Maybe<RefreshToken>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  refreshTokens: RefreshTokenConnection;
  /** Get a single #struct_name_str by ID */
  rssFeed?: Maybe<RssFeed>;
  /** Get a single #struct_name_str by ID */
  rssFeedItem?: Maybe<RssFeedItem>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  rssFeedItems: RssFeedItemConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  rssFeeds: RssFeedConnection;
  /** Get a single #struct_name_str by ID */
  scheduleCache?: Maybe<ScheduleCache>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  scheduleCaches: ScheduleCacheConnection;
  /** Get a single #struct_name_str by ID */
  scheduleSyncState?: Maybe<ScheduleSyncState>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  scheduleSyncStates: ScheduleSyncStateConnection;
  /** Get a single #struct_name_str by ID */
  show?: Maybe<Show>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  shows: ShowConnection;
  /** Get a single #struct_name_str by ID */
  source?: Maybe<Source>;
  /** Get a single #struct_name_str by ID */
  sourcePriorityRule?: Maybe<SourcePriorityRule>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  sourcePriorityRules: SourcePriorityRuleConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  sources: SourceConnection;
  /** Get a single #struct_name_str by ID */
  subtitle?: Maybe<Subtitle>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  subtitles: SubtitleConnection;
  /** Get a single #struct_name_str by ID */
  torrent?: Maybe<Torrent>;
  /** Get a single #struct_name_str by ID */
  torrentFile?: Maybe<TorrentFile>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  torrentFiles: TorrentFileConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  torrents: TorrentConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  torznabCategories: TorznabCategoryConnection;
  /** Get a single #struct_name_str by ID */
  torznabCategory?: Maybe<TorznabCategory>;
  /** Get a single #struct_name_str by ID */
  track?: Maybe<Track>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  tracks: TrackConnection;
  /** Get a single #struct_name_str by ID */
  usenetDownload?: Maybe<UsenetDownload>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  usenetDownloads: UsenetDownloadConnection;
  /** Get a single #struct_name_str by ID */
  usenetServer?: Maybe<UsenetServer>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  usenetServers: UsenetServerConnection;
  /** Get a single #struct_name_str by ID */
  user?: Maybe<User>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  users: UserConnection;
  /** Get a single #struct_name_str by ID */
  videoStream?: Maybe<VideoStream>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  videoStreams: VideoStreamConnection;
};

export type QueryRootBrowseDirectoryArgs = {
  Input?: InputMaybe<BrowseDirectoryInput>;
};

export type QueryRootLibraryPathAvailabilityArgs = {
  Input: LibraryPathAvailabilityInput;
};

export type QueryRootLiveTorrentArgs = {
  Id: Scalars["Int"]["input"];
};

export type QueryRootMovieCollectionDetailsArgs = {
  CollectionId: Scalars["Int"]["input"];
  LibraryId: Scalars["String"]["input"];
};

export type QueryRootSearchAlbumsArgs = {
  IncludeCompilations?: Scalars["Boolean"]["input"];
  IncludeEps?: Scalars["Boolean"]["input"];
  IncludeLive?: Scalars["Boolean"]["input"];
  IncludeSingles?: Scalars["Boolean"]["input"];
  IncludeSoundtracks?: Scalars["Boolean"]["input"];
  Query: Scalars["String"]["input"];
};

export type QueryRootSearchAudiobooksArgs = {
  Query: Scalars["String"]["input"];
};

export type QueryRootSearchMovieCollectionsArgs = {
  Query: Scalars["String"]["input"];
};

export type QueryRootSearchMoviesArgs = {
  Query: Scalars["String"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

export type QueryRootSearchSourcesArgs = {
  Input: SearchSourcesInput;
};

export type QueryRootSearchTvShowsArgs = {
  Query: Scalars["String"]["input"];
};

export type QueryRootSourceSettingDefinitionsArgs = {
  DefinitionId: Scalars["String"]["input"];
};

export type QueryRootalbumArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootalbumsArgs = {
  orderBy?: InputMaybe<Array<AlbumOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AlbumWhereInput>;
};

export type QueryRootappLogArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootappLogsArgs = {
  orderBy?: InputMaybe<Array<AppLogOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AppLogWhereInput>;
};

export type QueryRootappSettingArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootappSettingsArgs = {
  orderBy?: InputMaybe<Array<AppSettingOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AppSettingWhereInput>;
};

export type QueryRootartistArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootartistsArgs = {
  orderBy?: InputMaybe<Array<ArtistOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ArtistWhereInput>;
};

export type QueryRootartworkCacheArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootartworkCachesArgs = {
  orderBy?: InputMaybe<Array<ArtworkCacheOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ArtworkCacheWhereInput>;
};

export type QueryRootaudioStreamArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootaudioStreamsArgs = {
  orderBy?: InputMaybe<Array<AudioStreamOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AudioStreamWhereInput>;
};

export type QueryRootaudiobookArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootaudiobooksArgs = {
  orderBy?: InputMaybe<Array<AudiobookOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<AudiobookWhereInput>;
};

export type QueryRootcastDeviceArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootcastDevicesArgs = {
  orderBy?: InputMaybe<Array<CastDeviceOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<CastDeviceWhereInput>;
};

export type QueryRootcastSessionArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootcastSessionsArgs = {
  orderBy?: InputMaybe<Array<CastSessionOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<CastSessionWhereInput>;
};

export type QueryRootcastSettingArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootcastSettingsArgs = {
  orderBy?: InputMaybe<Array<CastSettingOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<CastSettingWhereInput>;
};

export type QueryRootchapterArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootchaptersArgs = {
  orderBy?: InputMaybe<Array<ChapterOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ChapterWhereInput>;
};

export type QueryRootcollectionArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootcollectionsArgs = {
  orderBy?: InputMaybe<Array<CollectionOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<CollectionWhereInput>;
};

export type QueryRootepisodeArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootepisodesArgs = {
  orderBy?: InputMaybe<Array<EpisodeOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<EpisodeWhereInput>;
};

export type QueryRootinviteTokenArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootinviteTokensArgs = {
  orderBy?: InputMaybe<Array<InviteTokenOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<InviteTokenWhereInput>;
};

export type QueryRootlibrariesArgs = {
  orderBy?: InputMaybe<Array<LibraryOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<LibraryWhereInput>;
};

export type QueryRootlibraryArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootmediaChapterArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootmediaChaptersArgs = {
  orderBy?: InputMaybe<Array<MediaChapterOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MediaChapterWhereInput>;
};

export type QueryRootmediaFileArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootmediaFilesArgs = {
  orderBy?: InputMaybe<Array<MediaFileOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MediaFileWhereInput>;
};

export type QueryRootmetadataCacheArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootmetadataCachesArgs = {
  orderBy?: InputMaybe<Array<MetadataCacheOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MetadataCacheWhereInput>;
};

export type QueryRootmovieArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootmovieCastCreditArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootmovieCastCreditsArgs = {
  orderBy?: InputMaybe<Array<MovieCastCreditOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MovieCastCreditWhereInput>;
};

export type QueryRootmoviesArgs = {
  orderBy?: InputMaybe<Array<MovieOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<MovieWhereInput>;
};

export type QueryRootnamingPatternArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootnamingPatternsArgs = {
  orderBy?: InputMaybe<Array<NamingPatternOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<NamingPatternWhereInput>;
};

export type QueryRootnotificationArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootnotificationsArgs = {
  orderBy?: InputMaybe<Array<NotificationOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<NotificationWhereInput>;
};

export type QueryRootpendingFileMatchArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootpendingFileMatchesArgs = {
  orderBy?: InputMaybe<Array<PendingFileMatchOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<PendingFileMatchWhereInput>;
};

export type QueryRootpeopleArgs = {
  orderBy?: InputMaybe<Array<PersonOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<PersonWhereInput>;
};

export type QueryRootpersonArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootplaybackProgressArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootplaybackProgressesArgs = {
  orderBy?: InputMaybe<Array<PlaybackProgressOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<PlaybackProgressWhereInput>;
};

export type QueryRootplaybackSessionArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootplaybackSessionsArgs = {
  orderBy?: InputMaybe<Array<PlaybackSessionOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<PlaybackSessionWhereInput>;
};

export type QueryRootrefreshTokenArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootrefreshTokensArgs = {
  orderBy?: InputMaybe<Array<RefreshTokenOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<RefreshTokenWhereInput>;
};

export type QueryRootrssFeedArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootrssFeedItemArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootrssFeedItemsArgs = {
  orderBy?: InputMaybe<Array<RssFeedItemOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<RssFeedItemWhereInput>;
};

export type QueryRootrssFeedsArgs = {
  orderBy?: InputMaybe<Array<RssFeedOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<RssFeedWhereInput>;
};

export type QueryRootscheduleCacheArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootscheduleCachesArgs = {
  orderBy?: InputMaybe<Array<ScheduleCacheOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ScheduleCacheWhereInput>;
};

export type QueryRootscheduleSyncStateArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootscheduleSyncStatesArgs = {
  orderBy?: InputMaybe<Array<ScheduleSyncStateOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ScheduleSyncStateWhereInput>;
};

export type QueryRootshowArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootshowsArgs = {
  orderBy?: InputMaybe<Array<ShowOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<ShowWhereInput>;
};

export type QueryRootsourceArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootsourcePriorityRuleArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootsourcePriorityRulesArgs = {
  orderBy?: InputMaybe<Array<SourcePriorityRuleOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<SourcePriorityRuleWhereInput>;
};

export type QueryRootsourcesArgs = {
  orderBy?: InputMaybe<Array<SourceOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<SourceWhereInput>;
};

export type QueryRootsubtitleArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootsubtitlesArgs = {
  orderBy?: InputMaybe<Array<SubtitleOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<SubtitleWhereInput>;
};

export type QueryRoottorrentArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRoottorrentFileArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRoottorrentFilesArgs = {
  orderBy?: InputMaybe<Array<TorrentFileOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<TorrentFileWhereInput>;
};

export type QueryRoottorrentsArgs = {
  orderBy?: InputMaybe<Array<TorrentOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<TorrentWhereInput>;
};

export type QueryRoottorznabCategoriesArgs = {
  orderBy?: InputMaybe<Array<TorznabCategoryOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<TorznabCategoryWhereInput>;
};

export type QueryRoottorznabCategoryArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRoottrackArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRoottracksArgs = {
  orderBy?: InputMaybe<Array<TrackOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<TrackWhereInput>;
};

export type QueryRootusenetDownloadArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootusenetDownloadsArgs = {
  orderBy?: InputMaybe<Array<UsenetDownloadOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<UsenetDownloadWhereInput>;
};

export type QueryRootusenetServerArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootusenetServersArgs = {
  orderBy?: InputMaybe<Array<UsenetServerOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<UsenetServerWhereInput>;
};

export type QueryRootuserArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootusersArgs = {
  orderBy?: InputMaybe<Array<UserOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<UserWhereInput>;
};

export type QueryRootvideoStreamArgs = {
  id: Scalars["String"]["input"];
};

export type QueryRootvideoStreamsArgs = {
  orderBy?: InputMaybe<Array<VideoStreamOrderByInput>>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<VideoStreamWhereInput>;
};

export type RefreshToken = {
  CreatedAt: Scalars["String"]["output"];
  ExpiresAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  IpAddress?: Maybe<Scalars["String"]["output"]>;
  LastUsedAt?: Maybe<Scalars["String"]["output"]>;
  ReplacedByTokenId?: Maybe<Scalars["String"]["output"]>;
  RevocationReason?: Maybe<Scalars["String"]["output"]>;
  RevokedAt?: Maybe<Scalars["String"]["output"]>;
  Scopes: Array<Scalars["String"]["output"]>;
  Session: Scalars["String"]["output"];
  SessionFamilyId: Scalars["String"]["output"];
  SessionId: Scalars["String"]["output"];
  TokenHash: Scalars["String"]["output"];
  UserAgent?: Maybe<Scalars["String"]["output"]>;
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type RefreshTokenChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  refreshToken?: Maybe<RefreshToken>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type RefreshTokenConnection = {
  /** The edges in this connection */
  edges: Array<RefreshTokenEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type RefreshTokenEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: RefreshToken;
};

/** GraphQL input for refresh token mutation. */
export type RefreshTokenInput = {
  RefreshToken: Scalars["String"]["input"];
};

export type RefreshTokenOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  ExpiresAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type RefreshTokenResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  refreshToken?: Maybe<RefreshToken>;
  success: Scalars["Boolean"]["output"];
};

export type RefreshTokenWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  ExpiresAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastUsedAt?: InputMaybe<DateFilter>;
  RevokedAt?: InputMaybe<DateFilter>;
  SessionFamilyId?: InputMaybe<StringFilter>;
  SessionId?: InputMaybe<StringFilter>;
  TokenHash?: InputMaybe<StringFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<RefreshTokenWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<RefreshTokenWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<RefreshTokenWhereInput>>;
};

/** GraphQL input for user registration (PascalCase field names). */
export type RegisterUserInput = {
  Email: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Password: Scalars["String"]["input"];
};

export type RelativeDateInput = {
  days: Scalars["Int"]["input"];
};

/** Result of re-matching files for a source */
export type RematchSourceResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  MatchCount: Scalars["Int"]["output"];
  Success: Scalars["Boolean"]["output"];
};

export type RenameFileInput = {
  NewName: Scalars["String"]["input"];
  Path: Scalars["String"]["input"];
};

export type RssFeed = {
  ConsecutiveFailures?: Maybe<Scalars["Int"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Enabled: Scalars["Boolean"]["output"];
  Id: Scalars["String"]["output"];
  LastError?: Maybe<Scalars["String"]["output"]>;
  LastPolledAt?: Maybe<Scalars["String"]["output"]>;
  LastSuccessfulAt?: Maybe<Scalars["String"]["output"]>;
  LibraryId?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  PollIntervalMinutes: Scalars["Int"]["output"];
  PostDownloadAction?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  Url: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type RssFeedChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  rssFeed?: Maybe<RssFeed>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type RssFeedConnection = {
  /** The edges in this connection */
  edges: Array<RssFeedEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type RssFeedEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: RssFeed;
};

export type RssFeedItem = {
  Description?: Maybe<Scalars["String"]["output"]>;
  FeedId: Scalars["String"]["output"];
  Guid?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  Link: Scalars["String"]["output"];
  LinkHash: Scalars["String"]["output"];
  ParsedAudio?: Maybe<Scalars["String"]["output"]>;
  ParsedCodec?: Maybe<Scalars["String"]["output"]>;
  ParsedEpisode?: Maybe<Scalars["Int"]["output"]>;
  ParsedHdr?: Maybe<Scalars["String"]["output"]>;
  ParsedResolution?: Maybe<Scalars["String"]["output"]>;
  ParsedSeason?: Maybe<Scalars["Int"]["output"]>;
  ParsedShowName?: Maybe<Scalars["String"]["output"]>;
  ParsedSource?: Maybe<Scalars["String"]["output"]>;
  Processed: Scalars["Boolean"]["output"];
  PubDate?: Maybe<Scalars["String"]["output"]>;
  SeenAt: Scalars["String"]["output"];
  SkippedReason?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TitleHash: Scalars["String"]["output"];
  TorrentId?: Maybe<Scalars["String"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type RssFeedItemChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  rssFeedItem?: Maybe<RssFeedItem>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type RssFeedItemConnection = {
  /** The edges in this connection */
  edges: Array<RssFeedItemEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type RssFeedItemEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: RssFeedItem;
};

export type RssFeedItemOrderByInput = {
  PubDate?: InputMaybe<OrderDirection>;
  SeenAt?: InputMaybe<OrderDirection>;
  Title?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type RssFeedItemResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  rssFeedItem?: Maybe<RssFeedItem>;
  success: Scalars["Boolean"]["output"];
};

export type RssFeedItemWhereInput = {
  FeedId?: InputMaybe<StringFilter>;
  Guid?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  LinkHash?: InputMaybe<StringFilter>;
  ParsedAudio?: InputMaybe<StringFilter>;
  ParsedCodec?: InputMaybe<StringFilter>;
  ParsedEpisode?: InputMaybe<IntFilter>;
  ParsedHdr?: InputMaybe<StringFilter>;
  ParsedResolution?: InputMaybe<StringFilter>;
  ParsedSeason?: InputMaybe<IntFilter>;
  ParsedShowName?: InputMaybe<StringFilter>;
  ParsedSource?: InputMaybe<StringFilter>;
  Processed?: InputMaybe<BoolFilter>;
  PubDate?: InputMaybe<DateFilter>;
  SeenAt?: InputMaybe<DateFilter>;
  Title?: InputMaybe<StringFilter>;
  TitleHash?: InputMaybe<StringFilter>;
  TorrentId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<RssFeedItemWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<RssFeedItemWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<RssFeedItemWhereInput>>;
};

export type RssFeedOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  LastPolledAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type RssFeedResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  rssFeed?: Maybe<RssFeed>;
  success: Scalars["Boolean"]["output"];
};

export type RssFeedWhereInput = {
  ConsecutiveFailures?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  LastPolledAt?: InputMaybe<DateFilter>;
  LastSuccessfulAt?: InputMaybe<DateFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  PollIntervalMinutes?: InputMaybe<IntFilter>;
  PostDownloadAction?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Url?: InputMaybe<StringFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<RssFeedWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<RssFeedWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<RssFeedWhereInput>>;
};

export type ScanLibraryResult = {
  Message?: Maybe<Scalars["String"]["output"]>;
  Status: Scalars["String"]["output"];
  Success: Scalars["Boolean"]["output"];
};

export type ScheduleCache = {
  AirDate: Scalars["String"]["output"];
  AirStamp?: Maybe<Scalars["String"]["output"]>;
  AirTime?: Maybe<Scalars["String"]["output"]>;
  CountryCode: Scalars["String"]["output"];
  CreatedAt: Scalars["String"]["output"];
  EpisodeImageUrl?: Maybe<Scalars["String"]["output"]>;
  EpisodeName: Scalars["String"]["output"];
  EpisodeNumber: Scalars["Int"]["output"];
  EpisodeType?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  Season: Scalars["Int"]["output"];
  ShowGenres: Array<Scalars["String"]["output"]>;
  ShowName: Scalars["String"]["output"];
  ShowNetwork?: Maybe<Scalars["String"]["output"]>;
  ShowPosterUrl?: Maybe<Scalars["String"]["output"]>;
  Summary?: Maybe<Scalars["String"]["output"]>;
  TvmazeEpisodeId: Scalars["Int"]["output"];
  TvmazeShowId: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type ScheduleCacheChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  scheduleCache?: Maybe<ScheduleCache>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type ScheduleCacheConnection = {
  /** The edges in this connection */
  edges: Array<ScheduleCacheEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ScheduleCacheEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: ScheduleCache;
};

export type ScheduleCacheOrderByInput = {
  AirDate?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  EpisodeName?: InputMaybe<OrderDirection>;
  EpisodeNumber?: InputMaybe<OrderDirection>;
  Season?: InputMaybe<OrderDirection>;
  ShowName?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type ScheduleCacheResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  scheduleCache?: Maybe<ScheduleCache>;
  success: Scalars["Boolean"]["output"];
};

export type ScheduleCacheWhereInput = {
  AirDate?: InputMaybe<DateFilter>;
  AirStamp?: InputMaybe<DateFilter>;
  CountryCode?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EpisodeName?: InputMaybe<StringFilter>;
  EpisodeNumber?: InputMaybe<IntFilter>;
  EpisodeType?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  Runtime?: InputMaybe<IntFilter>;
  Season?: InputMaybe<IntFilter>;
  ShowName?: InputMaybe<StringFilter>;
  ShowNetwork?: InputMaybe<StringFilter>;
  TvmazeEpisodeId?: InputMaybe<IntFilter>;
  TvmazeShowId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<ScheduleCacheWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<ScheduleCacheWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<ScheduleCacheWhereInput>>;
};

export type ScheduleSyncState = {
  CountryCode: Scalars["String"]["output"];
  CreatedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  LastSyncDays: Scalars["Int"]["output"];
  LastSyncedAt: Scalars["String"]["output"];
  SyncError?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type ScheduleSyncStateChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  scheduleSyncState?: Maybe<ScheduleSyncState>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type ScheduleSyncStateConnection = {
  /** The edges in this connection */
  edges: Array<ScheduleSyncStateEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ScheduleSyncStateEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: ScheduleSyncState;
};

export type ScheduleSyncStateOrderByInput = {
  CountryCode?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  LastSyncedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type ScheduleSyncStateResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  scheduleSyncState?: Maybe<ScheduleSyncState>;
  success: Scalars["Boolean"]["output"];
};

export type ScheduleSyncStateWhereInput = {
  CountryCode?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastSyncDays?: InputMaybe<IntFilter>;
  LastSyncedAt?: InputMaybe<DateFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<ScheduleSyncStateWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<ScheduleSyncStateWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<ScheduleSyncStateWhereInput>>;
};

/** Input for searching sources */
export type SearchSourcesInput = {
  Categories?: InputMaybe<Array<Scalars["Int"]["input"]>>;
  Episode?: InputMaybe<Scalars["String"]["input"]>;
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  Limit?: InputMaybe<Scalars["Int"]["input"]>;
  Query: Scalars["String"]["input"];
  Season?: InputMaybe<Scalars["Int"]["input"]>;
  SourceIds?: InputMaybe<Array<Scalars["String"]["input"]>>;
};

/**
 * Show entity representing a TV show.
 *
 * The Episodes relation is automatically generated by the GraphQLRelations macro
 * and uses DataLoader for N+1 prevention. Query with:
 * ```graphql
 * Show {
 * Episodes(Page: { Limit: 10 }) {
 * Edges { Node { ... } }
 * PageInfo { TotalCount }
 * }
 * }
 * ```
 */
export type Show = {
  AutoDownload: Scalars["Boolean"]["output"];
  AutoDownloadMode: AutoDownloadMode;
  BackdropUrl?: Maybe<Scalars["String"]["output"]>;
  ContentRating?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Genres: Array<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  ImdbId?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  Network?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  Path?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  SortName?: Maybe<Scalars["String"]["output"]>;
  TmdbId?: Maybe<Scalars["Int"]["output"]>;
  TvdbId?: Maybe<Scalars["Int"]["output"]>;
  TvmazeId?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  Year?: Maybe<Scalars["Int"]["output"]>;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  episodes: EpisodeConnection;
  /** Get related #graphql_name */
  library?: Maybe<Library>;
};

/**
 * Show entity representing a TV show.
 *
 * The Episodes relation is automatically generated by the GraphQLRelations macro
 * and uses DataLoader for N+1 prevention. Query with:
 * ```graphql
 * Show {
 * Episodes(Page: { Limit: 10 }) {
 * Edges { Node { ... } }
 * PageInfo { TotalCount }
 * }
 * }
 * ```
 */
export type ShowepisodesArgs = {
  orderBy?: InputMaybe<EpisodeOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<EpisodeWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type ShowChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  show?: Maybe<Show>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type ShowConnection = {
  /** The edges in this connection */
  edges: Array<ShowEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ShowEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Show;
};

export type ShowOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  SortName?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
  Year?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type ShowResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  show?: Maybe<Show>;
  success: Scalars["Boolean"]["output"];
};

export type ShowWhereInput = {
  AutoDownload?: InputMaybe<BoolFilter>;
  ContentRating?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  ImdbId?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  Network?: InputMaybe<StringFilter>;
  Runtime?: InputMaybe<IntFilter>;
  TmdbId?: InputMaybe<IntFilter>;
  TvdbId?: InputMaybe<IntFilter>;
  TvmazeId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Year?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<ShowWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<ShowWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<ShowWhereInput>>;
};

export type SimilarityInput = {
  value: Scalars["String"]["input"];
};

export type Source = {
  CreatedAt: Scalars["String"]["output"];
  DefinitionId: Scalars["String"]["output"];
  Enabled: Scalars["Boolean"]["output"];
  ErrorCount: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  LastError?: Maybe<Scalars["String"]["output"]>;
  LastErrorAt?: Maybe<Scalars["String"]["output"]>;
  LastSuccessAt?: Maybe<Scalars["String"]["output"]>;
  MediaTypes: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  Priority: Scalars["Int"]["output"];
  Settings?: Maybe<Scalars["String"]["output"]>;
  SiteUrl?: Maybe<Scalars["String"]["output"]>;
  SourceType: Scalars["String"]["output"];
  SupportsBookSearch: Scalars["Boolean"]["output"];
  SupportsMovieSearch: Scalars["Boolean"]["output"];
  SupportsMusicSearch: Scalars["Boolean"]["output"];
  SupportsSearch: Scalars["Boolean"]["output"];
  SupportsTvSearch: Scalars["Boolean"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type SourceChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  source?: Maybe<Source>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
};

/** Connection containing edges and page info */
export type SourceConnection = {
  /** The edges in this connection */
  edges: Array<SourceEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Information about an available source definition (e.g., IPTorrents) */
export type SourceDefinitionInfo = {
  Description: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  Language: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  RequiredCredentials: Array<Scalars["String"]["output"]>;
  SiteLink: Scalars["String"]["output"];
  SourceType: Scalars["String"]["output"];
  TrackerType: Scalars["String"]["output"];
};

/** Edge containing a node and cursor */
export type SourceEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Source;
};

/** Generic success/error result for source mutations */
export type SourceMutationResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type SourceOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  Priority?: InputMaybe<OrderDirection>;
  SourceType?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

export type SourcePriorityRule = {
  CreatedAt: Scalars["String"]["output"];
  Enabled: Scalars["Boolean"]["output"];
  Id: Scalars["String"]["output"];
  LibraryId?: Maybe<Scalars["String"]["output"]>;
  LibraryType?: Maybe<Scalars["String"]["output"]>;
  PriorityOrder: Array<Scalars["String"]["output"]>;
  SearchAllSources: Scalars["Boolean"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type SourcePriorityRuleChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  sourcePriorityRule?: Maybe<SourcePriorityRule>;
};

/** Connection containing edges and page info */
export type SourcePriorityRuleConnection = {
  /** The edges in this connection */
  edges: Array<SourcePriorityRuleEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type SourcePriorityRuleEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: SourcePriorityRule;
};

export type SourcePriorityRuleOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type SourcePriorityRuleResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  sourcePriorityRule?: Maybe<SourcePriorityRule>;
  success: Scalars["Boolean"]["output"];
};

export type SourcePriorityRuleWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  LibraryType?: InputMaybe<StringFilter>;
  SearchAllSources?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<SourcePriorityRuleWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<SourcePriorityRuleWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<SourcePriorityRuleWhereInput>>;
};

/** A single release from a source search */
export type SourceReleaseInfo = {
  Categories: Array<Scalars["Int"]["output"]>;
  Description?: Maybe<Scalars["String"]["output"]>;
  Details?: Maybe<Scalars["String"]["output"]>;
  Grabs?: Maybe<Scalars["Int"]["output"]>;
  Guid: Scalars["String"]["output"];
  ImdbId?: Maybe<Scalars["String"]["output"]>;
  InfoHash?: Maybe<Scalars["String"]["output"]>;
  IsFreeleech: Scalars["Boolean"]["output"];
  Leechers?: Maybe<Scalars["Int"]["output"]>;
  Link?: Maybe<Scalars["String"]["output"]>;
  MagnetUri?: Maybe<Scalars["String"]["output"]>;
  Peers?: Maybe<Scalars["Int"]["output"]>;
  Poster?: Maybe<Scalars["String"]["output"]>;
  PublishDate: Scalars["String"]["output"];
  Seeders?: Maybe<Scalars["Int"]["output"]>;
  Size?: Maybe<Scalars["Int"]["output"]>;
  SizeFormatted?: Maybe<Scalars["String"]["output"]>;
  SourceId?: Maybe<Scalars["String"]["output"]>;
  SourceName?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
};

/** Result type for #struct_name mutations */
export type SourceResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  source?: Maybe<Source>;
  success: Scalars["Boolean"]["output"];
};

/** Results from a single source */
export type SourceSearchResultItem = {
  ElapsedMs: Scalars["Int"]["output"];
  Error?: Maybe<Scalars["String"]["output"]>;
  FromCache: Scalars["Boolean"]["output"];
  Releases: Array<SourceReleaseInfo>;
  SourceId: Scalars["String"]["output"];
  SourceName: Scalars["String"]["output"];
};

/** Aggregated search results from all sources */
export type SourceSearchResultSet = {
  Sources: Array<SourceSearchResultItem>;
  SourcesSearched: Scalars["Int"]["output"];
  TotalElapsedMs: Scalars["Int"]["output"];
  TotalReleases: Scalars["Int"]["output"];
};

/** Definition of a configurable setting for a source */
export type SourceSettingDefinition = {
  DefaultValue?: Maybe<Scalars["String"]["output"]>;
  Key: Scalars["String"]["output"];
  Label: Scalars["String"]["output"];
  Options?: Maybe<Array<SourceSettingOption>>;
  SettingType: Scalars["String"]["output"];
};

/** Option for a select-type setting */
export type SourceSettingOption = {
  Label: Scalars["String"]["output"];
  Value: Scalars["String"]["output"];
};

/** Result of testing a source connection */
export type SourceTestConnectionResult = {
  ElapsedMs?: Maybe<Scalars["Int"]["output"]>;
  Error?: Maybe<Scalars["String"]["output"]>;
  ReleasesFound?: Maybe<Scalars["Int"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type SourceWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  DefinitionId?: InputMaybe<StringFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  ErrorCount?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  LastErrorAt?: InputMaybe<DateFilter>;
  LastSuccessAt?: InputMaybe<DateFilter>;
  MediaTypes?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  Priority?: InputMaybe<IntFilter>;
  SourceType?: InputMaybe<StringFilter>;
  SupportsBookSearch?: InputMaybe<BoolFilter>;
  SupportsMovieSearch?: InputMaybe<BoolFilter>;
  SupportsMusicSearch?: InputMaybe<BoolFilter>;
  SupportsSearch?: InputMaybe<BoolFilter>;
  SupportsTvSearch?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<SourceWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<SourceWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<SourceWhereInput>>;
};

export type StringFilter = {
  contains?: InputMaybe<Scalars["String"]["input"]>;
  endsWith?: InputMaybe<Scalars["String"]["input"]>;
  eq?: InputMaybe<Scalars["String"]["input"]>;
  inList?: InputMaybe<Array<Scalars["String"]["input"]>>;
  isNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  ne?: InputMaybe<Scalars["String"]["input"]>;
  notIn?: InputMaybe<Array<Scalars["String"]["input"]>>;
  similar?: InputMaybe<SimilarityInput>;
  startsWith?: InputMaybe<Scalars["String"]["input"]>;
};

export type SubscriptionFilterInput = {
  actions?: InputMaybe<Array<ChangeAction>>;
  dummy?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type SubscriptionRoot = {
  /**
   * Subscribe to filesystem change events (create/delete/copy/move/rename).
   * Fires when any filesystem mutation completes. Optional path filter.
   */
  FilesystemChanged: FilesystemChangeEvent;
  /** Subscribe to #struct_name_str changes */
  albumChanged: AlbumChangedEvent;
  /** Subscribe to #struct_name_str changes */
  appLogChanged: AppLogChangedEvent;
  /** Subscribe to #struct_name_str changes */
  appSettingChanged: AppSettingChangedEvent;
  /** Subscribe to #struct_name_str changes */
  artistChanged: ArtistChangedEvent;
  /** Subscribe to #struct_name_str changes */
  artworkCacheChanged: ArtworkCacheChangedEvent;
  /** Subscribe to #struct_name_str changes */
  audioStreamChanged: AudioStreamChangedEvent;
  /** Subscribe to #struct_name_str changes */
  audiobookChanged: AudiobookChangedEvent;
  /** Subscribe to #struct_name_str changes */
  castDeviceChanged: CastDeviceChangedEvent;
  /** Subscribe to #struct_name_str changes */
  castSessionChanged: CastSessionChangedEvent;
  /** Subscribe to #struct_name_str changes */
  castSettingChanged: CastSettingChangedEvent;
  /** Subscribe to #struct_name_str changes */
  chapterChanged: ChapterChangedEvent;
  /** Subscribe to #struct_name_str changes */
  collectionChanged: CollectionChangedEvent;
  /** Subscribe to #struct_name_str changes */
  episodeChanged: EpisodeChangedEvent;
  /** Subscribe to #struct_name_str changes */
  inviteTokenChanged: InviteTokenChangedEvent;
  /** Subscribe to #struct_name_str changes */
  libraryChanged: LibraryChangedEvent;
  /** Subscribe to #struct_name_str changes */
  mediaChapterChanged: MediaChapterChangedEvent;
  /** Subscribe to #struct_name_str changes */
  mediaFileChanged: MediaFileChangedEvent;
  /** Subscribe to #struct_name_str changes */
  metadataCacheChanged: MetadataCacheChangedEvent;
  /** Subscribe to #struct_name_str changes */
  movieCastCreditChanged: MovieCastCreditChangedEvent;
  /** Subscribe to #struct_name_str changes */
  movieChanged: MovieChangedEvent;
  /** Subscribe to #struct_name_str changes */
  namingPatternChanged: NamingPatternChangedEvent;
  /** Subscribe to #struct_name_str changes */
  notificationChanged: NotificationChangedEvent;
  /** Subscribe to #struct_name_str changes */
  pendingFileMatchChanged: PendingFileMatchChangedEvent;
  /** Subscribe to #struct_name_str changes */
  personChanged: PersonChangedEvent;
  /** Subscribe to #struct_name_str changes */
  playbackProgressChanged: PlaybackProgressChangedEvent;
  /** Subscribe to #struct_name_str changes */
  playbackSessionChanged: PlaybackSessionChangedEvent;
  /** Subscribe to #struct_name_str changes */
  refreshTokenChanged: RefreshTokenChangedEvent;
  /** Subscribe to #struct_name_str changes */
  rssFeedChanged: RssFeedChangedEvent;
  /** Subscribe to #struct_name_str changes */
  rssFeedItemChanged: RssFeedItemChangedEvent;
  /** Subscribe to #struct_name_str changes */
  scheduleCacheChanged: ScheduleCacheChangedEvent;
  /** Subscribe to #struct_name_str changes */
  scheduleSyncStateChanged: ScheduleSyncStateChangedEvent;
  /** Subscribe to #struct_name_str changes */
  showChanged: ShowChangedEvent;
  /** Subscribe to #struct_name_str changes */
  sourceChanged: SourceChangedEvent;
  /** Subscribe to #struct_name_str changes */
  sourcePriorityRuleChanged: SourcePriorityRuleChangedEvent;
  /** Subscribe to #struct_name_str changes */
  subtitleChanged: SubtitleChangedEvent;
  torrentAdded: TorrentAddedEvent;
  torrentCompleted: TorrentCompletedEvent;
  /** Subscribe to #struct_name_str changes */
  torrentFileChanged: TorrentFileChangedEvent;
  torrentProgress: TorrentProgress;
  torrentRemoved: TorrentRemovedEvent;
  /** Subscribe to #struct_name_str changes */
  torznabCategoryChanged: TorznabCategoryChangedEvent;
  /** Subscribe to #struct_name_str changes */
  trackChanged: TrackChangedEvent;
  /** Subscribe to #struct_name_str changes */
  usenetDownloadChanged: UsenetDownloadChangedEvent;
  /** Subscribe to #struct_name_str changes */
  usenetServerChanged: UsenetServerChangedEvent;
  /** Subscribe to #struct_name_str changes */
  userChanged: UserChangedEvent;
  /** Subscribe to #struct_name_str changes */
  videoStreamChanged: VideoStreamChangedEvent;
};

export type SubscriptionRootFilesystemChangedArgs = {
  Path?: InputMaybe<Scalars["String"]["input"]>;
};

export type SubscriptionRootalbumChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootappLogChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootappSettingChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootartistChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootartworkCacheChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootaudioStreamChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootaudiobookChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootcastDeviceChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootcastSessionChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootcastSettingChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootchapterChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootcollectionChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootepisodeChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootinviteTokenChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootlibraryChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootmediaChapterChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootmediaFileChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootmetadataCacheChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootmovieCastCreditChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootmovieChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootnamingPatternChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootnotificationChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootpendingFileMatchChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootpersonChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootplaybackProgressChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootplaybackSessionChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootrefreshTokenChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootrssFeedChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootrssFeedItemChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootscheduleCacheChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootscheduleSyncStateChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootshowChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootsourceChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootsourcePriorityRuleChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootsubtitleChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRoottorrentFileChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRoottorznabCategoryChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRoottrackChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootusenetDownloadChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootusenetServerChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootuserChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootvideoStreamChangedArgs = {
  filter?: InputMaybe<SubscriptionFilterInput>;
};

export type Subtitle = {
  Codec?: Maybe<Scalars["String"]["output"]>;
  CodecLongName?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  DownloadedAt?: Maybe<Scalars["String"]["output"]>;
  FilePath?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  IsDefault: Scalars["Boolean"]["output"];
  IsForced: Scalars["Boolean"]["output"];
  IsHearingImpaired: Scalars["Boolean"]["output"];
  Language?: Maybe<Scalars["String"]["output"]>;
  MediaFileId: Scalars["String"]["output"];
  Metadata?: Maybe<Scalars["String"]["output"]>;
  OpensubtitlesId?: Maybe<Scalars["String"]["output"]>;
  SourceType: Scalars["String"]["output"];
  StreamIndex?: Maybe<Scalars["Int"]["output"]>;
  Title?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type SubtitleChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  subtitle?: Maybe<Subtitle>;
};

/** Connection containing edges and page info */
export type SubtitleConnection = {
  /** The edges in this connection */
  edges: Array<SubtitleEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type SubtitleEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Subtitle;
};

export type SubtitleOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type SubtitleResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  subtitle?: Maybe<Subtitle>;
  success: Scalars["Boolean"]["output"];
};

export type SubtitleWhereInput = {
  Codec?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DownloadedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IsDefault?: InputMaybe<BoolFilter>;
  IsForced?: InputMaybe<BoolFilter>;
  IsHearingImpaired?: InputMaybe<BoolFilter>;
  Language?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  OpensubtitlesId?: InputMaybe<StringFilter>;
  SourceType?: InputMaybe<StringFilter>;
  StreamIndex?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<SubtitleWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<SubtitleWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<SubtitleWhereInput>>;
};

export type Torrent = {
  AddedAt: Scalars["String"]["output"];
  CompletedAt?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  DownloadPath?: Maybe<Scalars["String"]["output"]>;
  DownloadedBytes: Scalars["Int"]["output"];
  ExcludedFiles: Array<Scalars["Int"]["output"]>;
  Id: Scalars["String"]["output"];
  InfoHash: Scalars["String"]["output"];
  LibraryId?: Maybe<Scalars["String"]["output"]>;
  MagnetUri?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  PostProcessError?: Maybe<Scalars["String"]["output"]>;
  PostProcessStatus?: Maybe<Scalars["String"]["output"]>;
  ProcessedAt?: Maybe<Scalars["String"]["output"]>;
  Progress: Scalars["Float"]["output"];
  SavePath: Scalars["String"]["output"];
  SourceFeedId?: Maybe<Scalars["String"]["output"]>;
  SourceIndexerId?: Maybe<Scalars["String"]["output"]>;
  SourceUrl?: Maybe<Scalars["String"]["output"]>;
  State: Scalars["String"]["output"];
  TotalBytes: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UploadedBytes: Scalars["Int"]["output"];
  UserId: Scalars["String"]["output"];
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  files: TorrentFileConnection;
};

export type TorrentfilesArgs = {
  orderBy?: InputMaybe<TorrentFileOrderByInput>;
  page?: InputMaybe<PageInput>;
  where?: InputMaybe<TorrentFileWhereInput>;
};

/** Result of pause/resume/remove */
export type TorrentActionResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type TorrentAddedEvent = {
  id: Scalars["Int"]["output"];
  infoHash: Scalars["String"]["output"];
  name: Scalars["String"]["output"];
};

export type TorrentCompletedEvent = {
  id: Scalars["Int"]["output"];
  infoHash: Scalars["String"]["output"];
  name: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type TorrentConnection = {
  /** The edges in this connection */
  edges: Array<TorrentEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TorrentEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Torrent;
};

export type TorrentFile = {
  CreatedAt: Scalars["String"]["output"];
  DownloadedBytes: Scalars["Int"]["output"];
  FileIndex: Scalars["Int"]["output"];
  FilePath: Scalars["String"]["output"];
  FileSize: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  IsExcluded: Scalars["Boolean"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  Progress: Scalars["Float"]["output"];
  RelativePath: Scalars["String"]["output"];
  TorrentId: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type TorrentFileChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  torrentFile?: Maybe<TorrentFile>;
};

/** Connection containing edges and page info */
export type TorrentFileConnection = {
  /** The edges in this connection */
  edges: Array<TorrentFileEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TorrentFileEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: TorrentFile;
};

export type TorrentFileOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  FileIndex?: InputMaybe<OrderDirection>;
  FileSize?: InputMaybe<OrderDirection>;
  Progress?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type TorrentFileResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  torrentFile?: Maybe<TorrentFile>;
};

export type TorrentFileWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  DownloadedBytes?: InputMaybe<IntFilter>;
  FileIndex?: InputMaybe<IntFilter>;
  FilePath?: InputMaybe<StringFilter>;
  FileSize?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IsExcluded?: InputMaybe<BoolFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  Progress?: InputMaybe<IntFilter>;
  RelativePath?: InputMaybe<StringFilter>;
  TorrentId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<TorrentFileWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<TorrentFileWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<TorrentFileWhereInput>>;
};

export type TorrentOrderByInput = {
  AddedAt?: InputMaybe<OrderDirection>;
  CreatedAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  Progress?: InputMaybe<OrderDirection>;
  State?: InputMaybe<OrderDirection>;
  TotalBytes?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

export type TorrentProgress = {
  downloadSpeed: Scalars["Int"]["output"];
  id: Scalars["Int"]["output"];
  infoHash: Scalars["String"]["output"];
  peers: Scalars["Int"]["output"];
  progress: Scalars["Float"]["output"];
  state: Scalars["String"]["output"];
  uploadSpeed: Scalars["Int"]["output"];
};

export type TorrentRemovedEvent = {
  id: Scalars["Int"]["output"];
  infoHash: Scalars["String"]["output"];
};

/** Result type for #struct_name mutations */
export type TorrentResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  torrent?: Maybe<Torrent>;
};

export type TorrentWhereInput = {
  AddedAt?: InputMaybe<DateFilter>;
  CompletedAt?: InputMaybe<DateFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DownloadedBytes?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  InfoHash?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  PostProcessStatus?: InputMaybe<StringFilter>;
  ProcessedAt?: InputMaybe<DateFilter>;
  Progress?: InputMaybe<IntFilter>;
  SavePath?: InputMaybe<StringFilter>;
  SourceFeedId?: InputMaybe<StringFilter>;
  SourceIndexerId?: InputMaybe<StringFilter>;
  State?: InputMaybe<StringFilter>;
  TotalBytes?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UploadedBytes?: InputMaybe<IntFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<TorrentWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<TorrentWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<TorrentWhereInput>>;
};

export type TorznabCategory = {
  Description?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  ParentId?: Maybe<Scalars["String"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type TorznabCategoryChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  torznabCategory?: Maybe<TorznabCategory>;
};

/** Connection containing edges and page info */
export type TorznabCategoryConnection = {
  /** The edges in this connection */
  edges: Array<TorznabCategoryEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TorznabCategoryEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: TorznabCategory;
};

export type TorznabCategoryOrderByInput = {
  Id?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type TorznabCategoryResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  torznabCategory?: Maybe<TorznabCategory>;
};

export type TorznabCategoryWhereInput = {
  Id?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  ParentId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<TorznabCategoryWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<TorznabCategoryWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<TorznabCategoryWhereInput>>;
};

export type Track = {
  AlbumId: Scalars["String"]["output"];
  ArtistId?: Maybe<Scalars["String"]["output"]>;
  ArtistName?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  DiscNumber?: Maybe<Scalars["Int"]["output"]>;
  DurationSecs?: Maybe<Scalars["Int"]["output"]>;
  Explicit: Scalars["Boolean"]["output"];
  Id: Scalars["String"]["output"];
  Isrc?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  MusicbrainzId?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TrackNumber: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  Wanted: Scalars["Boolean"]["output"];
  /** Get related #graphql_name */
  mediaFile?: Maybe<MediaFile>;
};

/** Event for #struct_name changes (subscriptions) */
export type TrackChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  track?: Maybe<Track>;
};

/** Connection containing edges and page info */
export type TrackConnection = {
  /** The edges in this connection */
  edges: Array<TrackEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TrackEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: Track;
};

export type TrackOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  DiscNumber?: InputMaybe<OrderDirection>;
  DurationSecs?: InputMaybe<OrderDirection>;
  Title?: InputMaybe<OrderDirection>;
  TrackNumber?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type TrackResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  track?: Maybe<Track>;
};

export type TrackWhereInput = {
  AlbumId?: InputMaybe<StringFilter>;
  ArtistId?: InputMaybe<StringFilter>;
  ArtistName?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DiscNumber?: InputMaybe<IntFilter>;
  DurationSecs?: InputMaybe<IntFilter>;
  Explicit?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  Isrc?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  MusicbrainzId?: InputMaybe<StringFilter>;
  Title?: InputMaybe<StringFilter>;
  TrackNumber?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Wanted?: InputMaybe<BoolFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<TrackWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<TrackWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<TrackWhereInput>>;
};

/** Result of TV show operations */
export type TvShowOperationResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Show?: Maybe<Show>;
  Success: Scalars["Boolean"]["output"];
};

/** TV show search result from TVMaze */
export type TvShowSearchResult = {
  ImdbId?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  Network?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  Provider: Scalars["String"]["output"];
  ProviderId: Scalars["Int"]["output"];
  Score?: Maybe<Scalars["Float"]["output"]>;
  Status?: Maybe<Scalars["String"]["output"]>;
  TvdbId?: Maybe<Scalars["Int"]["output"]>;
  Year?: Maybe<Scalars["Int"]["output"]>;
};

export type UnmatchMediaFileResult = {
  Reason?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type UpdateAlbumInput = {
  AlbumType?: InputMaybe<Scalars["String"]["input"]>;
  ArtistId?: InputMaybe<Scalars["String"]["input"]>;
  AutoDownload?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoDownloadMode?: InputMaybe<AutoDownloadMode>;
  Country?: InputMaybe<Scalars["String"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  DiscCount?: InputMaybe<Scalars["Int"]["input"]>;
  Genres?: InputMaybe<Scalars["JSON"]["input"]>;
  HasFiles?: InputMaybe<Scalars["Boolean"]["input"]>;
  Label?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  TrackCount?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateAlbumsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateAppLogInput = {
  Fields?: InputMaybe<Scalars["String"]["input"]>;
  Level?: InputMaybe<Scalars["String"]["input"]>;
  Message?: InputMaybe<Scalars["String"]["input"]>;
  SpanId?: InputMaybe<Scalars["String"]["input"]>;
  SpanName?: InputMaybe<Scalars["String"]["input"]>;
  Target?: InputMaybe<Scalars["String"]["input"]>;
  Timestamp?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateAppLogsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateAppSettingInput = {
  Category?: InputMaybe<Scalars["String"]["input"]>;
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Key?: InputMaybe<Scalars["String"]["input"]>;
  Value?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateAppSettingsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateArtistInput = {
  AlbumCount?: InputMaybe<Scalars["Int"]["input"]>;
  Bio?: InputMaybe<Scalars["String"]["input"]>;
  Disambiguation?: InputMaybe<Scalars["String"]["input"]>;
  ImageUrl?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  TrackCount?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateArtistsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateArtworkCacheInput = {
  ArtworkType?: InputMaybe<Scalars["String"]["input"]>;
  ContentHash?: InputMaybe<Scalars["String"]["input"]>;
  EntityId?: InputMaybe<Scalars["String"]["input"]>;
  EntityType?: InputMaybe<Scalars["String"]["input"]>;
  Height?: InputMaybe<Scalars["Int"]["input"]>;
  MimeType?: InputMaybe<Scalars["String"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SourceUrl?: InputMaybe<Scalars["String"]["input"]>;
  Width?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateArtworkCachesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateAudioStreamInput = {
  BitDepth?: InputMaybe<Scalars["Int"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
  ChannelLayout?: InputMaybe<Scalars["String"]["input"]>;
  Channels?: InputMaybe<Scalars["Int"]["input"]>;
  Codec?: InputMaybe<Scalars["String"]["input"]>;
  CodecLongName?: InputMaybe<Scalars["String"]["input"]>;
  IsCommentary?: InputMaybe<Scalars["Boolean"]["input"]>;
  IsDefault?: InputMaybe<Scalars["Boolean"]["input"]>;
  Language?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  SampleRate?: InputMaybe<Scalars["Int"]["input"]>;
  StreamIndex?: InputMaybe<Scalars["Int"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateAudioStreamsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateAudiobookInput = {
  Asin?: InputMaybe<Scalars["String"]["input"]>;
  AudibleId?: InputMaybe<Scalars["String"]["input"]>;
  AuthorName?: InputMaybe<Scalars["String"]["input"]>;
  AutoDownload?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoDownloadMode?: InputMaybe<AutoDownloadMode>;
  ChapterCount?: InputMaybe<Scalars["Int"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  Description?: InputMaybe<Scalars["String"]["input"]>;
  GoodreadsId?: InputMaybe<Scalars["String"]["input"]>;
  HasFiles?: InputMaybe<Scalars["Boolean"]["input"]>;
  Isbn?: InputMaybe<Scalars["String"]["input"]>;
  Language?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  NarratorName?: InputMaybe<Scalars["String"]["input"]>;
  Narrators?: InputMaybe<Scalars["JSON"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PublishedDate?: InputMaybe<Scalars["String"]["input"]>;
  Publisher?: InputMaybe<Scalars["String"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateAudiobooksResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateCastDeviceInput = {
  Address?: InputMaybe<Scalars["String"]["input"]>;
  DeviceType?: InputMaybe<Scalars["String"]["input"]>;
  IsFavorite?: InputMaybe<Scalars["Boolean"]["input"]>;
  IsManual?: InputMaybe<Scalars["Boolean"]["input"]>;
  LastSeenAt?: InputMaybe<Scalars["String"]["input"]>;
  Model?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Port?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateCastDevicesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateCastSessionInput = {
  CurrentPosition?: InputMaybe<Scalars["Float"]["input"]>;
  DeviceId?: InputMaybe<Scalars["String"]["input"]>;
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  EndedAt?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  IsMuted?: InputMaybe<Scalars["Boolean"]["input"]>;
  LastPosition?: InputMaybe<Scalars["Float"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  PlayerState?: InputMaybe<Scalars["String"]["input"]>;
  StartedAt?: InputMaybe<Scalars["String"]["input"]>;
  StreamUrl?: InputMaybe<Scalars["String"]["input"]>;
  Volume?: InputMaybe<Scalars["Float"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateCastSessionsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateCastSettingInput = {
  AutoDiscoveryEnabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  DefaultVolume?: InputMaybe<Scalars["Float"]["input"]>;
  DiscoveryIntervalSeconds?: InputMaybe<Scalars["Int"]["input"]>;
  PreferredQuality?: InputMaybe<Scalars["String"]["input"]>;
  TranscodeIncompatible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateCastSettingsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateChapterInput = {
  AudiobookId?: InputMaybe<Scalars["String"]["input"]>;
  ChapterNumber?: InputMaybe<Scalars["Int"]["input"]>;
  DurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  EndTimeSecs?: InputMaybe<Scalars["Float"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  StartTimeSecs?: InputMaybe<Scalars["Float"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  Wanted?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateChaptersResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateCollectionInput = {
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  LastSyncedAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MovieCount?: InputMaybe<Scalars["Int"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  TmdbCollectionId?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateCollectionsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateEpisodeInput = {
  AbsoluteNumber?: InputMaybe<Scalars["Int"]["input"]>;
  AirDate?: InputMaybe<Scalars["String"]["input"]>;
  Episode?: InputMaybe<Scalars["Int"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  Season?: InputMaybe<Scalars["Int"]["input"]>;
  ShowId?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeId?: InputMaybe<Scalars["Int"]["input"]>;
  Wanted?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateEpisodesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateInviteTokenInput = {
  AccessLevel?: InputMaybe<Scalars["String"]["input"]>;
  ApplyRestrictions?: InputMaybe<Scalars["Boolean"]["input"]>;
  CreatedBy?: InputMaybe<Scalars["String"]["input"]>;
  ExpiresAt?: InputMaybe<Scalars["String"]["input"]>;
  IsActive?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryIds?: InputMaybe<Scalars["JSON"]["input"]>;
  MaxUses?: InputMaybe<Scalars["Int"]["input"]>;
  RestrictionsTemplate?: InputMaybe<Scalars["String"]["input"]>;
  Role?: InputMaybe<Scalars["String"]["input"]>;
  Token?: InputMaybe<Scalars["String"]["input"]>;
  UseCount?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateInviteTokensResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk update by Where filter */
export type UpdateLibrariesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateLibraryInput = {
  AutoOrganize?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoScan?: InputMaybe<Scalars["Boolean"]["input"]>;
  Color?: InputMaybe<Scalars["String"]["input"]>;
  Icon?: InputMaybe<Scalars["String"]["input"]>;
  LastScannedAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  NamingPattern?: InputMaybe<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  ScanIntervalMinutes?: InputMaybe<Scalars["Int"]["input"]>;
  Scanning?: InputMaybe<Scalars["Boolean"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  WatchForChanges?: InputMaybe<Scalars["Boolean"]["input"]>;
};

export type UpdateMediaChapterInput = {
  ChapterIndex?: InputMaybe<Scalars["Int"]["input"]>;
  EndSecs?: InputMaybe<Scalars["Float"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  StartSecs?: InputMaybe<Scalars["Float"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateMediaChaptersResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateMediaFileInput = {
  AddedAt?: InputMaybe<Scalars["String"]["input"]>;
  AnalyzedAt?: InputMaybe<Scalars["String"]["input"]>;
  AudioChannels?: InputMaybe<Scalars["String"]["input"]>;
  AudioCodec?: InputMaybe<Scalars["String"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
  ChapterId?: InputMaybe<Scalars["String"]["input"]>;
  Container?: InputMaybe<Scalars["String"]["input"]>;
  ContentType?: InputMaybe<Scalars["String"]["input"]>;
  Duration?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  HdrType?: InputMaybe<Scalars["String"]["input"]>;
  Height?: InputMaybe<Scalars["Int"]["input"]>;
  IsHdr?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  OriginalName?: InputMaybe<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  RelativePath?: InputMaybe<Scalars["String"]["input"]>;
  Resolution?: InputMaybe<Scalars["String"]["input"]>;
  Size?: InputMaybe<Scalars["Int"]["input"]>;
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
  VideoCodec?: InputMaybe<Scalars["String"]["input"]>;
  Width?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateMediaFilesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateMetadataCacheInput = {
  CacheKey?: InputMaybe<Scalars["String"]["input"]>;
  FetchedAt?: InputMaybe<Scalars["String"]["input"]>;
  Operation?: InputMaybe<Scalars["String"]["input"]>;
  Payload?: InputMaybe<Scalars["String"]["input"]>;
  PayloadVersion?: InputMaybe<Scalars["Int"]["input"]>;
  Provider?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateMetadataCachesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateMovieCastCreditInput = {
  CastOrder?: InputMaybe<Scalars["Int"]["input"]>;
  CharacterName?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  PersonId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateMovieCastCreditsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateMovieInput = {
  CastNames?: InputMaybe<Scalars["JSON"]["input"]>;
  Certification?: InputMaybe<Scalars["String"]["input"]>;
  CollectionId?: InputMaybe<Scalars["Int"]["input"]>;
  CollectionName?: InputMaybe<Scalars["String"]["input"]>;
  CollectionPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Director?: InputMaybe<Scalars["String"]["input"]>;
  DownloadStatus?: InputMaybe<Scalars["String"]["input"]>;
  Genres?: InputMaybe<Scalars["JSON"]["input"]>;
  HasFile?: InputMaybe<Scalars["Boolean"]["input"]>;
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Monitored?: InputMaybe<Scalars["Boolean"]["input"]>;
  OriginalTitle?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  ProductionCountries?: InputMaybe<Scalars["JSON"]["input"]>;
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  SpokenLanguages?: InputMaybe<Scalars["JSON"]["input"]>;
  Tagline?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TmdbRating?: InputMaybe<Scalars["String"]["input"]>;
  TmdbStatus?: InputMaybe<Scalars["String"]["input"]>;
  TmdbVoteCount?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  Wanted?: InputMaybe<Scalars["Boolean"]["input"]>;
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateMoviesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateNamingPatternInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  IsDefault?: InputMaybe<Scalars["Boolean"]["input"]>;
  IsSystem?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Pattern?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateNamingPatternsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateNotificationInput = {
  ActionData?: InputMaybe<Scalars["String"]["input"]>;
  ActionType?: InputMaybe<Scalars["String"]["input"]>;
  Category?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Message?: InputMaybe<Scalars["String"]["input"]>;
  NotificationType?: InputMaybe<Scalars["String"]["input"]>;
  PendingMatchId?: InputMaybe<Scalars["String"]["input"]>;
  ReadAt?: InputMaybe<Scalars["String"]["input"]>;
  Resolution?: InputMaybe<Scalars["String"]["input"]>;
  ResolvedAt?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TorrentId?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateNotificationsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdatePendingFileMatchInput = {
  ChapterId?: InputMaybe<Scalars["String"]["input"]>;
  CopiedAt?: InputMaybe<Scalars["String"]["input"]>;
  CopyAttempts?: InputMaybe<Scalars["Int"]["input"]>;
  CopyError?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  FileSize?: InputMaybe<Scalars["Int"]["input"]>;
  MatchAttempts?: InputMaybe<Scalars["Int"]["input"]>;
  MatchConfidence?: InputMaybe<Scalars["Float"]["input"]>;
  MatchType?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  ParsedAudio?: InputMaybe<Scalars["String"]["input"]>;
  ParsedCodec?: InputMaybe<Scalars["String"]["input"]>;
  ParsedResolution?: InputMaybe<Scalars["String"]["input"]>;
  ParsedSource?: InputMaybe<Scalars["String"]["input"]>;
  SourceFileIndex?: InputMaybe<Scalars["Int"]["input"]>;
  SourceId?: InputMaybe<Scalars["String"]["input"]>;
  SourcePath?: InputMaybe<Scalars["String"]["input"]>;
  SourceType?: InputMaybe<Scalars["String"]["input"]>;
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
  UnmatchedReason?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  VerificationReason?: InputMaybe<Scalars["String"]["input"]>;
  VerificationStatus?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdatePendingFileMatchesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk update by Where filter */
export type UpdatePeopleResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdatePersonInput = {
  Name?: InputMaybe<Scalars["String"]["input"]>;
  ProfileUrl?: InputMaybe<Scalars["String"]["input"]>;
  TmdbPersonId?: InputMaybe<Scalars["Int"]["input"]>;
};

export type UpdatePlaybackProgressInput = {
  CurrentPosition?: InputMaybe<Scalars["Float"]["input"]>;
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  IsWatched?: InputMaybe<Scalars["Boolean"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  ProgressPercent?: InputMaybe<Scalars["Float"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  WatchedAt?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdatePlaybackProgressesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdatePlaybackSessionInput = {
  AlbumId?: InputMaybe<Scalars["String"]["input"]>;
  AudiobookId?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  ContentType?: InputMaybe<Scalars["String"]["input"]>;
  CurrentPosition?: InputMaybe<Scalars["Float"]["input"]>;
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  IsMuted?: InputMaybe<Scalars["Boolean"]["input"]>;
  IsPlaying?: InputMaybe<Scalars["Boolean"]["input"]>;
  LastUpdatedAt?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  StartedAt?: InputMaybe<Scalars["String"]["input"]>;
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
  TvShowId?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  Volume?: InputMaybe<Scalars["Float"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdatePlaybackSessionsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateRefreshTokenInput = {
  ExpiresAt?: InputMaybe<Scalars["String"]["input"]>;
  IpAddress?: InputMaybe<Scalars["String"]["input"]>;
  LastUsedAt?: InputMaybe<Scalars["String"]["input"]>;
  ReplacedByTokenId?: InputMaybe<Scalars["String"]["input"]>;
  RevocationReason?: InputMaybe<Scalars["String"]["input"]>;
  RevokedAt?: InputMaybe<Scalars["String"]["input"]>;
  Scopes?: InputMaybe<Scalars["JSON"]["input"]>;
  Session?: InputMaybe<Scalars["String"]["input"]>;
  SessionFamilyId?: InputMaybe<Scalars["String"]["input"]>;
  SessionId?: InputMaybe<Scalars["String"]["input"]>;
  TokenHash?: InputMaybe<Scalars["String"]["input"]>;
  UserAgent?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateRefreshTokensResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateRssFeedInput = {
  ConsecutiveFailures?: InputMaybe<Scalars["Int"]["input"]>;
  Enabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastPolledAt?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessfulAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  PollIntervalMinutes?: InputMaybe<Scalars["Int"]["input"]>;
  PostDownloadAction?: InputMaybe<Scalars["String"]["input"]>;
  Url?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

export type UpdateRssFeedItemInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  FeedId?: InputMaybe<Scalars["String"]["input"]>;
  Guid?: InputMaybe<Scalars["String"]["input"]>;
  Link?: InputMaybe<Scalars["String"]["input"]>;
  LinkHash?: InputMaybe<Scalars["String"]["input"]>;
  ParsedAudio?: InputMaybe<Scalars["String"]["input"]>;
  ParsedCodec?: InputMaybe<Scalars["String"]["input"]>;
  ParsedEpisode?: InputMaybe<Scalars["Int"]["input"]>;
  ParsedHdr?: InputMaybe<Scalars["String"]["input"]>;
  ParsedResolution?: InputMaybe<Scalars["String"]["input"]>;
  ParsedSeason?: InputMaybe<Scalars["Int"]["input"]>;
  ParsedShowName?: InputMaybe<Scalars["String"]["input"]>;
  ParsedSource?: InputMaybe<Scalars["String"]["input"]>;
  Processed?: InputMaybe<Scalars["Boolean"]["input"]>;
  PubDate?: InputMaybe<Scalars["String"]["input"]>;
  SeenAt?: InputMaybe<Scalars["String"]["input"]>;
  SkippedReason?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TitleHash?: InputMaybe<Scalars["String"]["input"]>;
  TorrentId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateRssFeedItemsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk update by Where filter */
export type UpdateRssFeedsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateScheduleCacheInput = {
  AirDate?: InputMaybe<Scalars["String"]["input"]>;
  AirStamp?: InputMaybe<Scalars["String"]["input"]>;
  AirTime?: InputMaybe<Scalars["String"]["input"]>;
  CountryCode?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeImageUrl?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeName?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeNumber?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeType?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  Season?: InputMaybe<Scalars["Int"]["input"]>;
  ShowGenres?: InputMaybe<Scalars["JSON"]["input"]>;
  ShowName?: InputMaybe<Scalars["String"]["input"]>;
  ShowNetwork?: InputMaybe<Scalars["String"]["input"]>;
  ShowPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Summary?: InputMaybe<Scalars["String"]["input"]>;
  TvmazeEpisodeId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeShowId?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateScheduleCachesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateScheduleSyncStateInput = {
  CountryCode?: InputMaybe<Scalars["String"]["input"]>;
  LastSyncDays?: InputMaybe<Scalars["Int"]["input"]>;
  LastSyncedAt?: InputMaybe<Scalars["String"]["input"]>;
  SyncError?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateScheduleSyncStatesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateShowInput = {
  AutoDownload?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoDownloadMode?: InputMaybe<AutoDownloadMode>;
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  ContentRating?: InputMaybe<Scalars["String"]["input"]>;
  Genres?: InputMaybe<Scalars["JSON"]["input"]>;
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Network?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeId?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateShowsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateSourceInput = {
  DefinitionId?: InputMaybe<Scalars["String"]["input"]>;
  Enabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  ErrorCount?: InputMaybe<Scalars["Int"]["input"]>;
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastErrorAt?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessAt?: InputMaybe<Scalars["String"]["input"]>;
  MediaTypes?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Priority?: InputMaybe<Scalars["Int"]["input"]>;
  Settings?: InputMaybe<Scalars["String"]["input"]>;
  SiteUrl?: InputMaybe<Scalars["String"]["input"]>;
  SourceType?: InputMaybe<Scalars["String"]["input"]>;
  SupportsBookSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsMovieSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsMusicSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsTvSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  credentials?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating source priorities */
export type UpdateSourcePrioritiesInput = {
  /** Source IDs in the desired priority order (first = highest priority) */
  SourceIds: Array<Scalars["String"]["input"]>;
};

export type UpdateSourcePriorityRuleInput = {
  Enabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  PriorityOrder?: InputMaybe<Scalars["JSON"]["input"]>;
  SearchAllSources?: InputMaybe<Scalars["Boolean"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateSourcePriorityRulesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk update by Where filter */
export type UpdateSourcesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateSubtitleInput = {
  Codec?: InputMaybe<Scalars["String"]["input"]>;
  CodecLongName?: InputMaybe<Scalars["String"]["input"]>;
  DownloadedAt?: InputMaybe<Scalars["String"]["input"]>;
  FilePath?: InputMaybe<Scalars["String"]["input"]>;
  IsDefault?: InputMaybe<Scalars["Boolean"]["input"]>;
  IsForced?: InputMaybe<Scalars["Boolean"]["input"]>;
  IsHearingImpaired?: InputMaybe<Scalars["Boolean"]["input"]>;
  Language?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  OpensubtitlesId?: InputMaybe<Scalars["String"]["input"]>;
  SourceType?: InputMaybe<Scalars["String"]["input"]>;
  StreamIndex?: InputMaybe<Scalars["Int"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateSubtitlesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateTorrentFileInput = {
  DownloadedBytes?: InputMaybe<Scalars["Int"]["input"]>;
  FileIndex?: InputMaybe<Scalars["Int"]["input"]>;
  FilePath?: InputMaybe<Scalars["String"]["input"]>;
  FileSize?: InputMaybe<Scalars["Int"]["input"]>;
  IsExcluded?: InputMaybe<Scalars["Boolean"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Progress?: InputMaybe<Scalars["Float"]["input"]>;
  RelativePath?: InputMaybe<Scalars["String"]["input"]>;
  TorrentId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateTorrentFilesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateTorrentInput = {
  AddedAt?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  DownloadPath?: InputMaybe<Scalars["String"]["input"]>;
  DownloadedBytes?: InputMaybe<Scalars["Int"]["input"]>;
  ExcludedFiles?: InputMaybe<Scalars["JSON"]["input"]>;
  InfoHash?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MagnetUri?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  PostProcessError?: InputMaybe<Scalars["String"]["input"]>;
  PostProcessStatus?: InputMaybe<Scalars["String"]["input"]>;
  ProcessedAt?: InputMaybe<Scalars["String"]["input"]>;
  Progress?: InputMaybe<Scalars["Float"]["input"]>;
  SavePath?: InputMaybe<Scalars["String"]["input"]>;
  SourceFeedId?: InputMaybe<Scalars["String"]["input"]>;
  SourceIndexerId?: InputMaybe<Scalars["String"]["input"]>;
  SourceUrl?: InputMaybe<Scalars["String"]["input"]>;
  State?: InputMaybe<Scalars["String"]["input"]>;
  TotalBytes?: InputMaybe<Scalars["Int"]["input"]>;
  UploadedBytes?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateTorrentsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk update by Where filter */
export type UpdateTorznabCategoriesResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateTorznabCategoryInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  ParentId?: InputMaybe<Scalars["String"]["input"]>;
};

export type UpdateTrackInput = {
  AlbumId?: InputMaybe<Scalars["String"]["input"]>;
  ArtistId?: InputMaybe<Scalars["String"]["input"]>;
  ArtistName?: InputMaybe<Scalars["String"]["input"]>;
  DiscNumber?: InputMaybe<Scalars["Int"]["input"]>;
  DurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  Explicit?: InputMaybe<Scalars["Boolean"]["input"]>;
  Isrc?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TrackNumber?: InputMaybe<Scalars["Int"]["input"]>;
  Wanted?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateTracksResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateUsenetDownloadInput = {
  AlbumId?: InputMaybe<Scalars["String"]["input"]>;
  AudiobookId?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  DownloadPath?: InputMaybe<Scalars["String"]["input"]>;
  DownloadSpeed?: InputMaybe<Scalars["Int"]["input"]>;
  DownloadedBytes?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  ErrorMessage?: InputMaybe<Scalars["String"]["input"]>;
  EtaSeconds?: InputMaybe<Scalars["Int"]["input"]>;
  IndexerId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  NzbData?: InputMaybe<Scalars["String"]["input"]>;
  NzbHash?: InputMaybe<Scalars["String"]["input"]>;
  NzbName?: InputMaybe<Scalars["String"]["input"]>;
  NzbUrl?: InputMaybe<Scalars["String"]["input"]>;
  PostProcessStatus?: InputMaybe<Scalars["String"]["input"]>;
  Progress?: InputMaybe<Scalars["String"]["input"]>;
  RetryCount?: InputMaybe<Scalars["Int"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  State?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateUsenetDownloadsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateUsenetServerInput = {
  Connections?: InputMaybe<Scalars["Int"]["input"]>;
  Enabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  EncryptedPassword?: InputMaybe<Scalars["String"]["input"]>;
  ErrorCount?: InputMaybe<Scalars["Int"]["input"]>;
  Host?: InputMaybe<Scalars["String"]["input"]>;
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessAt?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  PasswordNonce?: InputMaybe<Scalars["String"]["input"]>;
  Port?: InputMaybe<Scalars["Int"]["input"]>;
  Priority?: InputMaybe<Scalars["Int"]["input"]>;
  RetentionDays?: InputMaybe<Scalars["Int"]["input"]>;
  UseSsl?: InputMaybe<Scalars["Boolean"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  Username?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateUsenetServersResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateUserInput = {
  AvatarUrl?: InputMaybe<Scalars["String"]["input"]>;
  DisplayName?: InputMaybe<Scalars["String"]["input"]>;
  Email?: InputMaybe<Scalars["String"]["input"]>;
  IsActive?: InputMaybe<Scalars["Boolean"]["input"]>;
  LastLoginAt?: InputMaybe<Scalars["String"]["input"]>;
  Role?: InputMaybe<Scalars["String"]["input"]>;
  Username?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateUsersResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UpdateVideoStreamInput = {
  AspectRatio?: InputMaybe<Scalars["String"]["input"]>;
  AvgFrameRate?: InputMaybe<Scalars["String"]["input"]>;
  BitDepth?: InputMaybe<Scalars["Int"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
  Codec?: InputMaybe<Scalars["String"]["input"]>;
  CodecLongName?: InputMaybe<Scalars["String"]["input"]>;
  ColorPrimaries?: InputMaybe<Scalars["String"]["input"]>;
  ColorSpace?: InputMaybe<Scalars["String"]["input"]>;
  ColorTransfer?: InputMaybe<Scalars["String"]["input"]>;
  FrameRate?: InputMaybe<Scalars["String"]["input"]>;
  HdrType?: InputMaybe<Scalars["String"]["input"]>;
  Height?: InputMaybe<Scalars["Int"]["input"]>;
  IsDefault?: InputMaybe<Scalars["Boolean"]["input"]>;
  Language?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Metadata?: InputMaybe<Scalars["String"]["input"]>;
  PixelFormat?: InputMaybe<Scalars["String"]["input"]>;
  StreamIndex?: InputMaybe<Scalars["Int"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  Width?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Result of bulk update by Where filter */
export type UpdateVideoStreamsResult = {
  affectedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type UsenetDownload = {
  AlbumId?: Maybe<Scalars["String"]["output"]>;
  AudiobookId?: Maybe<Scalars["String"]["output"]>;
  CompletedAt?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  DownloadPath?: Maybe<Scalars["String"]["output"]>;
  DownloadSpeed?: Maybe<Scalars["Int"]["output"]>;
  DownloadedBytes?: Maybe<Scalars["Int"]["output"]>;
  EpisodeId?: Maybe<Scalars["String"]["output"]>;
  ErrorMessage?: Maybe<Scalars["String"]["output"]>;
  EtaSeconds?: Maybe<Scalars["Int"]["output"]>;
  Id: Scalars["String"]["output"];
  IndexerId?: Maybe<Scalars["String"]["output"]>;
  LibraryId?: Maybe<Scalars["String"]["output"]>;
  MovieId?: Maybe<Scalars["String"]["output"]>;
  NzbData?: Maybe<Scalars["String"]["output"]>;
  NzbHash?: Maybe<Scalars["String"]["output"]>;
  NzbName: Scalars["String"]["output"];
  NzbUrl?: Maybe<Scalars["String"]["output"]>;
  PostProcessStatus?: Maybe<Scalars["String"]["output"]>;
  Progress?: Maybe<Scalars["String"]["output"]>;
  RetryCount: Scalars["Int"]["output"];
  SizeBytes?: Maybe<Scalars["Int"]["output"]>;
  State: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type UsenetDownloadChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  usenetDownload?: Maybe<UsenetDownload>;
};

/** Connection containing edges and page info */
export type UsenetDownloadConnection = {
  /** The edges in this connection */
  edges: Array<UsenetDownloadEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type UsenetDownloadEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: UsenetDownload;
};

export type UsenetDownloadOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  NzbName?: InputMaybe<OrderDirection>;
  SizeBytes?: InputMaybe<OrderDirection>;
  State?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type UsenetDownloadResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  usenetDownload?: Maybe<UsenetDownload>;
};

export type UsenetDownloadWhereInput = {
  AlbumId?: InputMaybe<StringFilter>;
  AudiobookId?: InputMaybe<StringFilter>;
  CompletedAt?: InputMaybe<DateFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DownloadSpeed?: InputMaybe<IntFilter>;
  DownloadedBytes?: InputMaybe<IntFilter>;
  EpisodeId?: InputMaybe<StringFilter>;
  EtaSeconds?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IndexerId?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MovieId?: InputMaybe<StringFilter>;
  NzbHash?: InputMaybe<StringFilter>;
  NzbName?: InputMaybe<StringFilter>;
  PostProcessStatus?: InputMaybe<StringFilter>;
  RetryCount?: InputMaybe<IntFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  State?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<UsenetDownloadWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<UsenetDownloadWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<UsenetDownloadWhereInput>>;
};

export type UsenetServer = {
  Connections: Scalars["Int"]["output"];
  CreatedAt: Scalars["String"]["output"];
  Enabled: Scalars["Boolean"]["output"];
  EncryptedPassword?: Maybe<Scalars["String"]["output"]>;
  ErrorCount: Scalars["Int"]["output"];
  Host: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  LastError?: Maybe<Scalars["String"]["output"]>;
  LastSuccessAt?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  PasswordNonce?: Maybe<Scalars["String"]["output"]>;
  Port: Scalars["Int"]["output"];
  Priority: Scalars["Int"]["output"];
  RetentionDays?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UseSsl: Scalars["Boolean"]["output"];
  UserId: Scalars["String"]["output"];
  Username?: Maybe<Scalars["String"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type UsenetServerChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  usenetServer?: Maybe<UsenetServer>;
};

/** Connection containing edges and page info */
export type UsenetServerConnection = {
  /** The edges in this connection */
  edges: Array<UsenetServerEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type UsenetServerEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: UsenetServer;
};

export type UsenetServerOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  Name?: InputMaybe<OrderDirection>;
  Priority?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type UsenetServerResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  usenetServer?: Maybe<UsenetServer>;
};

export type UsenetServerWhereInput = {
  Connections?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  ErrorCount?: InputMaybe<IntFilter>;
  Host?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  LastSuccessAt?: InputMaybe<DateFilter>;
  Name?: InputMaybe<StringFilter>;
  Port?: InputMaybe<IntFilter>;
  Priority?: InputMaybe<IntFilter>;
  RetentionDays?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UseSsl?: InputMaybe<BoolFilter>;
  UserId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<UsenetServerWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<UsenetServerWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<UsenetServerWhereInput>>;
};

export type User = {
  AvatarUrl?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  DisplayName?: Maybe<Scalars["String"]["output"]>;
  Email?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  IsActive: Scalars["Boolean"]["output"];
  LastLoginAt?: Maybe<Scalars["String"]["output"]>;
  Role: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  Username: Scalars["String"]["output"];
  passwordHash: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type UserChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  user?: Maybe<User>;
};

/** Connection containing edges and page info */
export type UserConnection = {
  /** The edges in this connection */
  edges: Array<UserEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type UserEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: User;
};

export type UserOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  LastLoginAt?: InputMaybe<OrderDirection>;
  Role?: InputMaybe<OrderDirection>;
  UpdatedAt?: InputMaybe<OrderDirection>;
  Username?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type UserResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  user?: Maybe<User>;
};

export type UserWhereInput = {
  CreatedAt?: InputMaybe<DateFilter>;
  DisplayName?: InputMaybe<StringFilter>;
  Email?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  IsActive?: InputMaybe<BoolFilter>;
  LastLoginAt?: InputMaybe<DateFilter>;
  Role?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Username?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<UserWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<UserWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<UserWhereInput>>;
};

export type VideoStream = {
  AspectRatio?: Maybe<Scalars["String"]["output"]>;
  AvgFrameRate?: Maybe<Scalars["String"]["output"]>;
  BitDepth?: Maybe<Scalars["Int"]["output"]>;
  Bitrate?: Maybe<Scalars["Int"]["output"]>;
  Codec: Scalars["String"]["output"];
  CodecLongName?: Maybe<Scalars["String"]["output"]>;
  ColorPrimaries?: Maybe<Scalars["String"]["output"]>;
  ColorSpace?: Maybe<Scalars["String"]["output"]>;
  ColorTransfer?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  FrameRate?: Maybe<Scalars["String"]["output"]>;
  HdrType?: Maybe<Scalars["String"]["output"]>;
  Height: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  IsDefault: Scalars["Boolean"]["output"];
  Language?: Maybe<Scalars["String"]["output"]>;
  MediaFileId: Scalars["String"]["output"];
  Metadata?: Maybe<Scalars["String"]["output"]>;
  PixelFormat?: Maybe<Scalars["String"]["output"]>;
  StreamIndex: Scalars["Int"]["output"];
  Title?: Maybe<Scalars["String"]["output"]>;
  Width: Scalars["Int"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type VideoStreamChangedEvent = {
  action: ChangeAction;
  changeKind: ChangeKind;
  id: Scalars["String"]["output"];
  path: Array<Scalars["String"]["output"]>;
  sourceEntity?: Maybe<Scalars["String"]["output"]>;
  sourceId?: Maybe<Scalars["String"]["output"]>;
  videoStream?: Maybe<VideoStream>;
};

/** Connection containing edges and page info */
export type VideoStreamConnection = {
  /** The edges in this connection */
  edges: Array<VideoStreamEdge>;
  /** Pagination information */
  pageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type VideoStreamEdge = {
  /** A cursor for pagination */
  cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  node: VideoStream;
};

export type VideoStreamOrderByInput = {
  CreatedAt?: InputMaybe<OrderDirection>;
  StreamIndex?: InputMaybe<OrderDirection>;
};

/** Result type for #struct_name mutations */
export type VideoStreamResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
  videoStream?: Maybe<VideoStream>;
};

export type VideoStreamWhereInput = {
  BitDepth?: InputMaybe<IntFilter>;
  Bitrate?: InputMaybe<IntFilter>;
  Codec?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  HdrType?: InputMaybe<StringFilter>;
  Height?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IsDefault?: InputMaybe<BoolFilter>;
  Language?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  StreamIndex?: InputMaybe<IntFilter>;
  Width?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  and?: InputMaybe<Array<VideoStreamWhereInput>>;
  /** Logical NOT of condition */
  not?: InputMaybe<VideoStreamWhereInput>;
  /** Logical OR of conditions */
  or?: InputMaybe<Array<VideoStreamWhereInput>>;
};

export type PlaybackSyncIntervalQueryVariables = Exact<{
  Key: string;
}>;

export type PlaybackSyncIntervalQuery = {
  AppSettings: {
    Edges: Array<{ Node: { Id: string; Key: string; Value: string } }>;
  };
};

export type TorrentAppSettingsQueryVariables = Exact<{ [key: string]: never }>;

export type TorrentAppSettingsQuery = {
  AppSettings: {
    Edges: Array<{ Node: { Id: string; Key: string; Value: string } }>;
  };
};

export type MetadataAppSettingsQueryVariables = Exact<{ [key: string]: never }>;

export type MetadataAppSettingsQuery = {
  AppSettings: {
    Edges: Array<{ Node: { Id: string; Key: string; Value: string } }>;
  };
};

export type LlmAppSettingsQueryVariables = Exact<{ [key: string]: never }>;

export type LlmAppSettingsQuery = {
  AppSettings: {
    Edges: Array<{ Node: { Id: string; Key: string; Value: string } }>;
  };
};

export type CreateAppSettingMutationVariables = Exact<{
  Input: CreateAppSettingInput;
}>;

export type CreateAppSettingMutation = {
  CreateAppSetting: {
    Success: boolean;
    Error: string | null;
    AppSetting: { Id: string; Key: string; Value: string } | null;
  };
};

export type UpdateAppSettingMutationVariables = Exact<{
  Id: string;
  Input: UpdateAppSettingInput;
}>;

export type UpdateAppSettingMutation = {
  UpdateAppSetting: {
    Success: boolean;
    Error: string | null;
    AppSetting: { Id: string; Key: string; Value: string } | null;
  };
};

export type NeedsSetupQueryVariables = Exact<{ [key: string]: never }>;

export type NeedsSetupQuery = { NeedsSetup: boolean };

export type MeQueryVariables = Exact<{ [key: string]: never }>;

export type MeQuery = {
  Me: {
    Id: string;
    Email: string | null;
    Username: string;
    Role: string;
    DisplayName: string | null;
  } | null;
};

export type LoginMutationVariables = Exact<{
  input: LoginInput;
}>;

export type LoginMutation = {
  Login: {
    Success: boolean;
    Error: string | null;
    User: {
      Id: string;
      Email: string | null;
      Username: string;
      Role: string;
      DisplayName: string | null;
    } | null;
    Tokens: {
      AccessToken: string;
      RefreshToken: string;
      ExpiresIn: number;
      TokenType: string;
    } | null;
  };
};

export type RegisterMutationVariables = Exact<{
  input: RegisterUserInput;
}>;

export type RegisterMutation = {
  Register: {
    Success: boolean;
    Error: string | null;
    User: {
      Id: string;
      Email: string | null;
      Username: string;
      Role: string;
      DisplayName: string | null;
    } | null;
    Tokens: {
      AccessToken: string;
      RefreshToken: string;
      ExpiresIn: number;
      TokenType: string;
    } | null;
  };
};

export type RefreshTokenMutationVariables = Exact<{
  input: RefreshTokenInput;
}>;

export type RefreshTokenMutation = {
  RefreshToken: {
    Success: boolean;
    Error: string | null;
    Tokens: {
      AccessToken: string;
      RefreshToken: string;
      ExpiresIn: number;
      TokenType: string;
    } | null;
  };
};

export type LogoutMutationVariables = Exact<{
  input: LogoutInput;
}>;

export type LogoutMutation = {
  Logout: { Success: boolean; Error: string | null };
};

export type CastDevicesQueryVariables = Exact<{
  Where?: CastDeviceWhereInput | null | undefined;
  OrderBy?:
    | Array<CastDeviceOrderByInput>
    | CastDeviceOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type CastDevicesQuery = {
  CastDevices: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        Name: string;
        Address: string;
        Port: number;
        Model: string | null;
        DeviceType: string;
        IsFavorite: boolean;
        IsManual: boolean;
        LastSeenAt: string | null;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount: number | null };
  };
};

export type CastSessionsQueryVariables = Exact<{
  Where?: CastSessionWhereInput | null | undefined;
  OrderBy?:
    | Array<CastSessionOrderByInput>
    | CastSessionOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type CastSessionsQuery = {
  CastSessions: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        DeviceId: string | null;
        MediaFileId: string | null;
        EpisodeId: string | null;
        StreamUrl: string;
        PlayerState: string;
        CurrentPosition: number;
        Duration: number | null;
        Volume: number;
        IsMuted: boolean;
        StartedAt: string;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount: number | null };
  };
};

export type CastSettingsQueryVariables = Exact<{
  Where?: CastSettingWhereInput | null | undefined;
  OrderBy?:
    | Array<CastSettingOrderByInput>
    | CastSettingOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type CastSettingsQuery = {
  CastSettings: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        AutoDiscoveryEnabled: boolean;
        DiscoveryIntervalSeconds: number;
        DefaultVolume: number;
        TranscodeIncompatible: boolean;
        PreferredQuality: string | null;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount: number | null };
  };
};

export type CreateCastDeviceMutationVariables = Exact<{
  Input: CreateCastDeviceInput;
}>;

export type CreateCastDeviceMutation = {
  CreateCastDevice: {
    Success: boolean;
    Error: string | null;
    CastDevice: {
      Id: string;
      Name: string;
      Address: string;
      Port: number;
      Model: string | null;
      DeviceType: string;
      IsFavorite: boolean;
      IsManual: boolean;
      LastSeenAt: string | null;
    } | null;
  };
};

export type UpdateCastDeviceMutationVariables = Exact<{
  Id: string;
  Input: UpdateCastDeviceInput;
}>;

export type UpdateCastDeviceMutation = {
  UpdateCastDevice: {
    Success: boolean;
    Error: string | null;
    CastDevice: {
      Id: string;
      Name: string;
      Address: string;
      Port: number;
      Model: string | null;
      DeviceType: string;
      IsFavorite: boolean;
      IsManual: boolean;
      LastSeenAt: string | null;
    } | null;
  };
};

export type DeleteCastDeviceMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteCastDeviceMutation = {
  DeleteCastDevice: { Success: boolean; Error: string | null };
};

export type CreateCastSettingMutationVariables = Exact<{
  Input: CreateCastSettingInput;
}>;

export type CreateCastSettingMutation = {
  CreateCastSetting: {
    Success: boolean;
    Error: string | null;
    CastSetting: {
      Id: string;
      AutoDiscoveryEnabled: boolean;
      DiscoveryIntervalSeconds: number;
      DefaultVolume: number;
      TranscodeIncompatible: boolean;
      PreferredQuality: string | null;
    } | null;
  };
};

export type UpdateCastSettingMutationVariables = Exact<{
  Id: string;
  Input: UpdateCastSettingInput;
}>;

export type UpdateCastSettingMutation = {
  UpdateCastSetting: {
    Success: boolean;
    Error: string | null;
    CastSetting: {
      Id: string;
      AutoDiscoveryEnabled: boolean;
      DiscoveryIntervalSeconds: number;
      DefaultVolume: number;
      TranscodeIncompatible: boolean;
      PreferredQuality: string | null;
    } | null;
  };
};

export type DiscoverCastDevicesOpMutationVariables = Exact<{
  [key: string]: never;
}>;

export type DiscoverCastDevicesOpMutation = {
  DiscoverCastDevices: Array<{
    id: string;
    name: string;
    address: string;
    port: number;
    model: string | null;
    deviceType: string;
    isFavorite: boolean;
    isManual: boolean;
    isConnected: boolean;
    lastSeenAt: string | null;
  }>;
};

export type CastMediaOpMutationVariables = Exact<{
  input: CastMediaInput;
}>;

export type CastMediaOpMutation = {
  CastMedia: {
    success: boolean;
    error: string | null;
    session: {
      id: string;
      deviceId: string | null;
      deviceName: string | null;
      mediaFileId: string | null;
      episodeId: string | null;
      streamUrl: string;
      playerState: string;
      currentTime: number;
      duration: number | null;
      volume: number;
      isMuted: boolean;
      startedAt: string;
    } | null;
  };
};

export type CastPlayOpMutationVariables = Exact<{
  sessionId: string;
}>;

export type CastPlayOpMutation = {
  CastPlay: {
    success: boolean;
    error: string | null;
    session: { id: string; playerState: string; currentTime: number } | null;
  };
};

export type CastPauseOpMutationVariables = Exact<{
  sessionId: string;
}>;

export type CastPauseOpMutation = {
  CastPause: {
    success: boolean;
    error: string | null;
    session: { id: string; playerState: string; currentTime: number } | null;
  };
};

export type CastStopOpMutationVariables = Exact<{
  sessionId: string;
}>;

export type CastStopOpMutation = {
  CastStop: { success: boolean; error: string | null };
};

export type CastSeekOpMutationVariables = Exact<{
  sessionId: string;
  position: number;
}>;

export type CastSeekOpMutation = {
  CastSeek: {
    success: boolean;
    error: string | null;
    session: { id: string; playerState: string; currentTime: number } | null;
  };
};

export type CastSetVolumeOpMutationVariables = Exact<{
  sessionId: string;
  volume: number;
}>;

export type CastSetVolumeOpMutation = {
  CastSetVolume: {
    success: boolean;
    error: string | null;
    session: { id: string; volume: number; isMuted: boolean } | null;
  };
};

export type CastSetMutedOpMutationVariables = Exact<{
  sessionId: string;
  muted: boolean;
}>;

export type CastSetMutedOpMutation = {
  CastSetMuted: {
    success: boolean;
    error: string | null;
    session: { id: string; volume: number; isMuted: boolean } | null;
  };
};

export type DashboardShowsQueryVariables = Exact<{
  Where?: ShowWhereInput | null | undefined;
  Page?: PageInput | null | undefined;
  OrderBy?: Array<ShowOrderByInput> | ShowOrderByInput | null | undefined;
}>;

export type DashboardShowsQuery = {
  Shows: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        LibraryId: string;
        Name: string;
        SortName: string | null;
        Year: number | null;
        TvmazeId: number | null;
        TmdbId: number | null;
        TvdbId: number | null;
        ImdbId: string | null;
        Overview: string | null;
        Network: string | null;
        Runtime: number | null;
        PosterUrl: string | null;
        BackdropUrl: string | null;
        Path: string | null;
        Genres: Array<string>;
        CreatedAt: string;
      };
    }>;
    PageInfo: { TotalCount: number | null };
  };
};

export type DashboardScheduleCachesQueryVariables = Exact<{
  Where?: ScheduleCacheWhereInput | null | undefined;
  OrderBy?:
    | Array<ScheduleCacheOrderByInput>
    | ScheduleCacheOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type DashboardScheduleCachesQuery = {
  ScheduleCaches: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        TvmazeEpisodeId: number;
        EpisodeName: string;
        Season: number;
        EpisodeNumber: number;
        EpisodeType: string | null;
        AirDate: string;
        AirTime: string | null;
        AirStamp: string | null;
        Runtime: number | null;
        EpisodeImageUrl: string | null;
        Summary: string | null;
        TvmazeShowId: number;
        ShowName: string;
        ShowNetwork: string | null;
        ShowPosterUrl: string | null;
        ShowGenres: Array<string>;
        CountryCode: string;
      };
    }>;
    PageInfo: { TotalCount: number | null };
  };
};

export type MediaFilePropertiesQueryVariables = Exact<{
  Id: string;
}>;

export type MediaFilePropertiesQuery = {
  MediaFile: {
    Id: string;
    LibraryId: string | null;
    Path: string;
    RelativePath: string | null;
    OriginalName: string | null;
    Size: number;
    Container: string | null;
    VideoCodec: string | null;
    AudioCodec: string | null;
    Resolution: string | null;
    IsHdr: boolean;
    HdrType: string | null;
    Width: number | null;
    Height: number | null;
    Duration: number | null;
    Bitrate: number | null;
    AudioChannels: string | null;
    EpisodeId: string | null;
    MovieId: string | null;
    TrackId: string | null;
    ContentType: string | null;
    AddedAt: string;
  } | null;
  VideoStreams: {
    Edges: Array<{
      Node: {
        Id: string;
        StreamIndex: number;
        Codec: string;
        CodecLongName: string | null;
        Width: number;
        Height: number;
        AspectRatio: string | null;
        FrameRate: string | null;
        Bitrate: number | null;
        PixelFormat: string | null;
        HdrType: string | null;
        BitDepth: number | null;
        Language: string | null;
        Title: string | null;
        IsDefault: boolean;
      };
    }>;
  };
  AudioStreams: {
    Edges: Array<{
      Node: {
        Id: string;
        StreamIndex: number;
        Codec: string;
        CodecLongName: string | null;
        Channels: number;
        ChannelLayout: string | null;
        SampleRate: number | null;
        Bitrate: number | null;
        BitDepth: number | null;
        Language: string | null;
        Title: string | null;
        IsDefault: boolean;
        IsCommentary: boolean;
      };
    }>;
  };
  Subtitles: {
    Edges: Array<{
      Node: {
        Id: string;
        StreamIndex: number | null;
        SourceType: string;
        Codec: string | null;
        CodecLongName: string | null;
        Language: string | null;
        Title: string | null;
        IsDefault: boolean;
        IsForced: boolean;
        IsHearingImpaired: boolean;
        FilePath: string | null;
      };
    }>;
  };
  MediaChapters: {
    Edges: Array<{
      Node: {
        Id: string;
        ChapterIndex: number;
        StartSecs: number;
        EndSecs: number;
        Title: string | null;
      };
    }>;
  };
};

export type MediaFileByPathLookupQueryVariables = Exact<{
  Path: string;
}>;

export type MediaFileByPathLookupQuery = {
  MediaFiles: { Edges: Array<{ Node: { Id: string; Path: string } }> };
};

export type MediaFileMetadataQueryVariables = Exact<{
  Id: string;
}>;

export type MediaFileMetadataQuery = {
  MediaFile: { Id: string; Metadata: string | null } | null;
};

export type BrowseDirectoryQueryVariables = Exact<{
  Input?: BrowseDirectoryInput | null | undefined;
}>;

export type BrowseDirectoryQuery = {
  BrowseDirectory: {
    CurrentPath: string;
    ParentPath: string | null;
    IsLibraryPath: boolean;
    LibraryId: string | null;
    Entries: Array<{
      Name: string;
      Path: string;
      IsDir: boolean;
      Size: number;
      SizeFormatted: string;
      Readable: boolean;
      Writable: boolean;
      MimeType: string | null;
      ModifiedAt: string | null;
    }>;
    QuickPaths: Array<{ Name: string; Path: string }>;
  };
};

export type FilesystemRuntimeInfoQueryVariables = Exact<{
  [key: string]: never;
}>;

export type FilesystemRuntimeInfoQuery = {
  FilesystemRuntimeInfo: {
    Platform: string;
    SupportsUncCredentials: boolean;
    SupportsSambaMount: boolean;
    DefaultLinuxMountBase: string | null;
  };
};

export type LibraryPathAvailabilityQueryVariables = Exact<{
  Input: LibraryPathAvailabilityInput;
}>;

export type LibraryPathAvailabilityQuery = {
  LibraryPathAvailability: Array<{
    Path: string;
    Reachable: boolean;
    Exists: boolean;
    IsDirectory: boolean;
    NeedsReconnect: boolean;
    ReconnectAttempted: boolean;
    ReconnectSucceeded: boolean;
    Message: string | null;
  }>;
};

export type ConfigureNetworkPathMutationVariables = Exact<{
  Input: ConfigureNetworkPathInput;
}>;

export type ConfigureNetworkPathMutation = {
  ConfigureNetworkPath: {
    Success: boolean;
    Error: string | null;
    ResolvedPath: string;
    Connected: boolean;
    Stored: boolean;
    Message: string | null;
  };
};

export type ReconnectLibraryPathMutationVariables = Exact<{
  Path: string;
}>;

export type ReconnectLibraryPathMutation = {
  ReconnectLibraryPath: {
    Success: boolean;
    Error: string | null;
    ResolvedPath: string;
    Connected: boolean;
    Stored: boolean;
    Message: string | null;
  };
};

export type LibrariesQueryVariables = Exact<{
  Where?: LibraryWhereInput | null | undefined;
  OrderBy?: Array<LibraryOrderByInput> | LibraryOrderByInput | null | undefined;
  Page?: PageInput | null | undefined;
}>;

export type LibrariesQuery = {
  Libraries: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        UserId: string;
        Name: string;
        Path: string;
        LibraryType: string;
        Icon: string | null;
        Color: string | null;
        AutoScan: boolean;
        ScanIntervalMinutes: number;
        WatchForChanges: boolean;
        AutoOrganize: boolean;
        NamingPattern: string;
        Scanning: boolean;
        LastScannedAt: string | null;
        CreatedAt: string;
        UpdatedAt: string;
        Shows: { PageInfo: { TotalCount: number | null } };
        ShowArtwork: {
          Edges: Array<{ Node: { Id: string; PosterUrl: string | null } }>;
        };
        Movies: { PageInfo: { TotalCount: number | null } };
        MovieArtwork: {
          Edges: Array<{
            Node: { Id: string; CollectionPosterUrl: string | null };
          }>;
        };
        Albums: { PageInfo: { TotalCount: number | null } };
        AlbumArtwork: {
          Edges: Array<{ Node: { Id: string; CoverUrl: string | null } }>;
        };
        Audiobooks: { PageInfo: { TotalCount: number | null } };
        AudiobookArtwork: {
          Edges: Array<{ Node: { Id: string; CoverUrl: string | null } }>;
        };
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount: number | null };
  };
};

export type LibraryChangedSubscriptionVariables = Exact<{
  Filter?: SubscriptionFilterInput | null | undefined;
}>;

export type LibraryChangedSubscription = {
  LibraryChanged: {
    Action: ChangeAction;
    Id: string;
    Library: {
      Id: string;
      Name: string;
      Path: string;
      LibraryType: string;
      Icon: string | null;
      Color: string | null;
      AutoScan: boolean;
      ScanIntervalMinutes: number;
      WatchForChanges: boolean;
      Scanning: boolean;
      LastScannedAt: string | null;
      CreatedAt: string;
      UpdatedAt: string;
    } | null;
  };
};

export type CreateLibraryMutationVariables = Exact<{
  Input: CreateLibraryInput;
}>;

export type CreateLibraryMutation = {
  CreateLibrary: {
    Success: boolean;
    Error: string | null;
    Library: {
      Id: string;
      Name: string;
      Path: string;
      LibraryType: string;
      Icon: string | null;
      Color: string | null;
    } | null;
  };
};

export type DeleteLibraryMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteLibraryMutation = {
  DeleteLibrary: { Success: boolean; Error: string | null };
};

export type ScanLibraryMutationVariables = Exact<{
  Id: string;
}>;

export type ScanLibraryMutation = {
  ScanLibrary: { Success: boolean; Message: string | null };
};

export type LibraryAlbumsTabQueryVariables = Exact<{
  LibraryId: string;
}>;

export type LibraryAlbumsTabQuery = {
  Albums: {
    Edges: Array<{
      Node: {
        Id: string;
        ArtistId: string;
        LibraryId: string;
        Name: string;
        SortName: string | null;
        Year: number | null;
        MusicbrainzId: string | null;
        AlbumType: string | null;
        Genres: Array<string>;
        Label: string | null;
        Country: string | null;
        ReleaseDate: string | null;
        CoverUrl: string | null;
        TrackCount: number | null;
        DiscCount: number | null;
        TotalDurationSecs: number | null;
        HasFiles: boolean;
        SizeBytes: number | null;
        Path: string | null;
      };
    }>;
  };
};

export type LibraryArtistsTabQueryVariables = Exact<{
  LibraryId: string;
}>;

export type LibraryArtistsTabQuery = {
  Artists: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string;
        Name: string;
        SortName: string | null;
        MusicbrainzId: string | null;
      };
    }>;
  };
};

export type LibraryAudiobooksTabQueryVariables = Exact<{
  LibraryId: string;
}>;

export type LibraryAudiobooksTabQuery = {
  Audiobooks: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string;
        Title: string;
        SortTitle: string | null;
        Isbn: string | null;
        Description: string | null;
        Publisher: string | null;
        Language: string | null;
        Narrators: Array<string>;
        CoverUrl: string | null;
        HasFiles: boolean;
        SizeBytes: number | null;
        Path: string | null;
        ChapterCount: number | null;
        TotalDurationSecs: number | null;
        AuthorName: string | null;
      };
    }>;
  };
};

export type LibraryUnmatchedMediaFilesTabQueryVariables = Exact<{
  LibraryId: string;
}>;

export type LibraryUnmatchedMediaFilesTabQuery = {
  MediaFiles: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string | null;
        Path: string;
        RelativePath: string | null;
        OriginalName: string | null;
        Size: number;
        Container: string | null;
        VideoCodec: string | null;
        AudioCodec: string | null;
        Resolution: string | null;
        IsHdr: boolean;
        HdrType: string | null;
        Width: number | null;
        Height: number | null;
        Duration: number | null;
        EpisodeId: string | null;
        ChapterId: string | null;
        AddedAt: string;
      };
    }>;
  };
};

export type LibraryDetailRouteQueryVariables = Exact<{
  Id: string;
}>;

export type LibraryDetailRouteQuery = {
  Library: {
    Id: string;
    Name: string;
    Path: string;
    LibraryType: string;
    AutoScan: boolean;
    ScanIntervalMinutes: number;
    WatchForChanges: boolean;
    AutoOrganize: boolean;
    NamingPattern: string;
    Scanning: boolean;
  } | null;
};

export type UpdateLibraryRouteMutationVariables = Exact<{
  Id: string;
  Input: UpdateLibraryInput;
}>;

export type UpdateLibraryRouteMutation = {
  UpdateLibrary: {
    Success: boolean;
    Error: string | null;
    Library: { Id: string } | null;
  };
};

export type DeleteShowRouteMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteShowRouteMutation = {
  DeleteShow: { Success: boolean; Error: string | null };
};

export type AppLogsQueryVariables = Exact<{
  Where?: AppLogWhereInput | null | undefined;
  OrderBy?: Array<AppLogOrderByInput> | AppLogOrderByInput | null | undefined;
  Page?: PageInput | null | undefined;
}>;

export type AppLogsQuery = {
  AppLogs: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        Timestamp: string;
        Level: string;
        Target: string;
        Message: string;
        Fields: string | null;
        SpanName: string | null;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount: number | null };
  };
};

export type AppLogChangedSubscriptionVariables = Exact<{
  Filter?: SubscriptionFilterInput | null | undefined;
}>;

export type AppLogChangedSubscription = {
  AppLogChanged: {
    Action: ChangeAction;
    Id: string;
    AppLog: {
      Id: string;
      Timestamp: string;
      Level: string;
      Target: string;
      Message: string;
      Fields: string | null;
      SpanName: string | null;
    } | null;
  };
};

export type DeleteAppLogsMutationVariables = Exact<{
  Where: AppLogWhereInput;
}>;

export type DeleteAppLogsMutation = {
  DeleteAppLogs: {
    success: boolean;
    error: string | null;
    DeletedCount: number;
  };
};

export type ManualMatchShowsByLibraryQueryVariables = Exact<{
  LibraryId: string;
}>;

export type ManualMatchShowsByLibraryQuery = {
  Shows: {
    Edges: Array<{
      Node: {
        Id: string;
        Name: string;
        Year: number | null;
        Episodes: {
          Edges: Array<{
            Node: {
              Id: string;
              Season: number;
              Episode: number;
              Title: string | null;
            };
          }>;
        };
      };
    }>;
  };
};

export type ManualMatchMoviesByLibraryQueryVariables = Exact<{
  LibraryId: string;
}>;

export type ManualMatchMoviesByLibraryQuery = {
  Movies: {
    Edges: Array<{ Node: { Id: string; Title: string; Year: number | null } }>;
  };
};

export type ManualMatchAlbumsByLibraryQueryVariables = Exact<{
  LibraryId: string;
}>;

export type ManualMatchAlbumsByLibraryQuery = {
  Albums: {
    Edges: Array<{ Node: { Id: string; Name: string; Year: number | null } }>;
  };
  Tracks: {
    Edges: Array<{
      Node: {
        Id: string;
        AlbumId: string;
        ArtistName: string | null;
        TrackNumber: number;
        Title: string;
      };
    }>;
  };
};

export type ManualMatchAudiobooksByLibraryQueryVariables = Exact<{
  LibraryId: string;
}>;

export type ManualMatchAudiobooksByLibraryQuery = {
  Audiobooks: {
    Edges: Array<{
      Node: {
        Id: string;
        Title: string;
        AuthorName: string | null;
        Chapters: {
          Edges: Array<{
            Node: { Id: string; ChapterNumber: number; Title: string | null };
          }>;
        };
      };
    }>;
  };
};

export type ManualMatchFileMutationVariables = Exact<{
  Input: MatchMediaFileInput;
}>;

export type ManualMatchFileMutation = {
  MatchMediaFile: {
    Success: boolean;
    Confidence: number;
    MatchedId: string | null;
    MatchedType: string | null;
    Reason: string | null;
  };
};

export type SearchAlbumsQueryVariables = Exact<{
  Query: string;
  IncludeEps?: boolean | null | undefined;
  IncludeSingles?: boolean | null | undefined;
  IncludeCompilations?: boolean | null | undefined;
  IncludeLive?: boolean | null | undefined;
  IncludeSoundtracks?: boolean | null | undefined;
}>;

export type SearchAlbumsQuery = {
  SearchAlbums: Array<{
    Provider: string;
    ProviderId: string;
    Title: string;
    ArtistName: string | null;
    Year: number | null;
    AlbumType: string | null;
    CoverUrl: string | null;
    Score: number | null;
  }>;
};

export type SearchAudiobooksQueryVariables = Exact<{
  Query: string;
}>;

export type SearchAudiobooksQuery = {
  SearchAudiobooks: Array<{
    Provider: string;
    ProviderId: string;
    Title: string;
    AuthorName: string | null;
    Year: number | null;
    CoverUrl: string | null;
    Isbn: string | null;
    Description: string | null;
  }>;
};

export type AddAlbumMutationVariables = Exact<{
  Input: AddAlbumInput;
}>;

export type AddAlbumMutation = {
  AddAlbum: { Success: boolean; Error: string | null };
};

export type AddAudiobookMutationVariables = Exact<{
  Input: AddAudiobookInput;
}>;

export type AddAudiobookMutation = {
  AddAudiobook: { Success: boolean; Error: string | null };
};

export type AddTorrentMutationVariables = Exact<{
  Input: AddTorrentInput;
}>;

export type AddTorrentMutation = {
  AddTorrent: {
    Success: boolean;
    Error: string | null;
    Torrent: { Id: number; Name: string } | null;
  };
};

export type AlbumDetailRouteQueryVariables = Exact<{
  Id: string;
}>;

export type AlbumDetailRouteQuery = {
  Album: {
    Id: string;
    ArtistId: string;
    LibraryId: string;
    Name: string;
    SortName: string | null;
    Year: number | null;
    MusicbrainzId: string | null;
    AlbumType: string | null;
    Genres: Array<string>;
    Label: string | null;
    Country: string | null;
    ReleaseDate: string | null;
    CoverUrl: string | null;
    TrackCount: number | null;
    DiscCount: number | null;
    TotalDurationSecs: number | null;
    HasFiles: boolean;
    SizeBytes: number | null;
    Path: string | null;
  } | null;
  Tracks: {
    Edges: Array<{
      Node: {
        Id: string;
        AlbumId: string;
        LibraryId: string;
        Title: string;
        TrackNumber: number;
        DiscNumber: number | null;
        MusicbrainzId: string | null;
        Isrc: string | null;
        DurationSecs: number | null;
        Explicit: boolean;
        ArtistName: string | null;
        ArtistId: string | null;
        MediaFileId: string | null;
        Wanted: boolean;
      };
    }>;
  };
};

export type DeleteAlbumRouteMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteAlbumRouteMutation = {
  DeleteAlbum: { Success: boolean; Error: string | null };
};

export type AlbumDetailSetTrackWantedMutationVariables = Exact<{
  AlbumId: string;
  Wanted: boolean;
}>;

export type AlbumDetailSetTrackWantedMutation = {
  UpdateTracks: {
    success: boolean;
    error: string | null;
    affectedCount: number;
  };
};

export type AudiobookDetailRouteQueryVariables = Exact<{
  Id: string;
}>;

export type AudiobookDetailRouteQuery = {
  Audiobook: {
    Id: string;
    LibraryId: string;
    Title: string;
    SortTitle: string | null;
    Isbn: string | null;
    Description: string | null;
    Publisher: string | null;
    Language: string | null;
    Narrators: Array<string>;
    TotalDurationSecs: number | null;
    CoverUrl: string | null;
    HasFiles: boolean;
    SizeBytes: number | null;
    Path: string | null;
    Chapters: {
      Edges: Array<{
        Node: {
          Id: string;
          AudiobookId: string;
          ChapterNumber: number;
          Title: string | null;
          StartTimeSecs: number;
          EndTimeSecs: number | null;
          DurationSecs: number | null;
          MediaFileId: string | null;
          Wanted: boolean;
        };
      }>;
    };
  } | null;
};

export type DeleteAudiobookRouteMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteAudiobookRouteMutation = {
  DeleteAudiobook: { Success: boolean; Error: string | null };
};

export type AudiobookDetailSetChapterWantedMutationVariables = Exact<{
  AudiobookId: string;
  Wanted: boolean;
}>;

export type AudiobookDetailSetChapterWantedMutation = {
  UpdateChapters: {
    success: boolean;
    error: string | null;
    affectedCount: number;
  };
};

export type SearchMoviesQueryVariables = Exact<{
  Query: string;
  Year?: number | null | undefined;
}>;

export type SearchMoviesQuery = {
  SearchMovies: Array<{
    Provider: string;
    ProviderId: number;
    Title: string;
    OriginalTitle: string | null;
    Year: number | null;
    Overview: string | null;
    PosterUrl: string | null;
    BackdropUrl: string | null;
    ImdbId: string | null;
    VoteAverage: number | null;
    Popularity: number | null;
  }>;
};

export type SearchMovieCollectionsQueryVariables = Exact<{
  Query: string;
}>;

export type SearchMovieCollectionsQuery = {
  SearchMovieCollections: Array<{
    Provider: string;
    CollectionId: number;
    Name: string;
    Overview: string | null;
    PosterUrl: string | null;
    BackdropUrl: string | null;
  }>;
};

export type AddMovieMutationVariables = Exact<{
  LibraryId: string;
  Input: AddMovieInput;
}>;

export type AddMovieMutation = {
  AddMovie: {
    Success: boolean;
    Error: string | null;
    Movie: {
      Id: string;
      LibraryId: string;
      Title: string;
      Year: number | null;
      TmdbId: number | null;
      ImdbId: string | null;
      Overview: string | null;
      Monitored: boolean;
      MediaFileId: string | null;
    } | null;
  };
};

export type AddMovieCollectionMutationVariables = Exact<{
  LibraryId: string;
  Input: AddMovieCollectionInput;
}>;

export type AddMovieCollectionMutation = {
  AddMovieCollection: {
    Success: boolean;
    CollectionId: number | null;
    CollectionName: string | null;
    ImportedCount: number;
    ExistingCount: number;
    WantedUpdatedCount: number;
    Error: string | null;
  };
};

export type MovieChangedSubscriptionVariables = Exact<{
  Filter?: SubscriptionFilterInput | null | undefined;
}>;

export type MovieChangedSubscription = {
  MovieChanged: {
    Action: ChangeAction;
    Id: string;
    Movie: { LibraryId: string } | null;
  };
};

export type MovieDetailRouteQueryVariables = Exact<{
  Id: string;
}>;

export type MovieDetailRouteQuery = {
  Movie: {
    Id: string;
    LibraryId: string;
    Title: string;
    SortTitle: string | null;
    OriginalTitle: string | null;
    Year: number | null;
    TmdbId: number | null;
    ImdbId: string | null;
    Overview: string | null;
    Tagline: string | null;
    Runtime: number | null;
    Genres: Array<string>;
    Director: string | null;
    CastNames: Array<string>;
    Monitored: boolean;
    MediaFileId: string | null;
    CollectionId: number | null;
    CollectionName: string | null;
    CollectionPosterUrl: string | null;
    TmdbRating: string | null;
    TmdbVoteCount: number | null;
    Certification: string | null;
    ReleaseDate: string | null;
    ProductionCountries: Array<string>;
    SpokenLanguages: Array<string>;
    Wanted: boolean;
    PosterUrl: string | null;
    MediaFile: { Id: string; Size: number; Duration: number | null } | null;
  } | null;
};

export type MovieDetailSetWantedMutationVariables = Exact<{
  Id: string;
  Wanted: boolean;
}>;

export type MovieDetailSetWantedMutation = {
  UpdateMovie: {
    Success: boolean;
    Error: string | null;
    Movie: { Id: string; Wanted: boolean } | null;
  };
};

export type RefreshMovieRouteMutationVariables = Exact<{
  Id: string;
}>;

export type RefreshMovieRouteMutation = {
  RefreshMovie: {
    Success: boolean;
    Error: string | null;
    Movie: {
      Id: string;
      Title: string;
      Overview: string | null;
      Tagline: string | null;
      TmdbRating: string | null;
      TmdbVoteCount: number | null;
    } | null;
  };
};

export type DeleteMovieModalMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteMovieModalMutation = {
  DeleteMovie: { Success: boolean; Error: string | null };
};

export type OrganizationNamingPatternsQueryVariables = Exact<{
  OrderBy?:
    | Array<NamingPatternOrderByInput>
    | NamingPatternOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type OrganizationNamingPatternsQuery = {
  NamingPatterns: {
    Edges: Array<{
      Node: {
        Id: string;
        Name: string;
        Pattern: string;
        Description: string | null;
        LibraryType: string;
        IsDefault: boolean;
        IsSystem: boolean;
      };
    }>;
  };
};

export type OrganizationCreateNamingPatternMutationVariables = Exact<{
  Input: CreateNamingPatternInput;
}>;

export type OrganizationCreateNamingPatternMutation = {
  CreateNamingPattern: {
    Success: boolean;
    Error: string | null;
    NamingPattern: {
      Id: string;
      Name: string;
      Pattern: string;
      Description: string | null;
      LibraryType: string;
      IsDefault: boolean;
      IsSystem: boolean;
    } | null;
  };
};

export type OrganizationUpdateNamingPatternMutationVariables = Exact<{
  Id: string;
  Input: UpdateNamingPatternInput;
}>;

export type OrganizationUpdateNamingPatternMutation = {
  UpdateNamingPattern: {
    Success: boolean;
    Error: string | null;
    NamingPattern: {
      Id: string;
      Name: string;
      Pattern: string;
      Description: string | null;
      LibraryType: string;
      IsDefault: boolean;
      IsSystem: boolean;
    } | null;
  };
};

export type OrganizationDeleteNamingPatternMutationVariables = Exact<{
  Id: string;
}>;

export type OrganizationDeleteNamingPatternMutation = {
  DeleteNamingPattern: { Success: boolean; Error: string | null };
};

export type NotificationsQueryVariables = Exact<{
  Where?: NotificationWhereInput | null | undefined;
  OrderBy?:
    | Array<NotificationOrderByInput>
    | NotificationOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type NotificationsQuery = {
  Notifications: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        UserId: string;
        NotificationType: string;
        Category: string;
        Title: string;
        Message: string;
        LibraryId: string | null;
        TorrentId: string | null;
        MediaFileId: string | null;
        PendingMatchId: string | null;
        ActionType: string | null;
        ActionData: string | null;
        ReadAt: string | null;
        ResolvedAt: string | null;
        Resolution: string | null;
        CreatedAt: string;
        UpdatedAt: string;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount: number | null };
  };
};

export type NotificationChangedSubscriptionVariables = Exact<{
  Filter?: SubscriptionFilterInput | null | undefined;
}>;

export type NotificationChangedSubscription = {
  NotificationChanged: {
    Action: ChangeAction;
    Id: string;
    Notification: {
      Id: string;
      ReadAt: string | null;
      ResolvedAt: string | null;
      Resolution: string | null;
    } | null;
  };
};

export type UpdateNotificationMutationVariables = Exact<{
  Id: string;
  Input: UpdateNotificationInput;
}>;

export type UpdateNotificationMutation = {
  UpdateNotification: {
    Success: boolean;
    Error: string | null;
    Notification: {
      Id: string;
      ReadAt: string | null;
      ResolvedAt: string | null;
      Resolution: string | null;
    } | null;
  };
};

export type DeleteNotificationMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteNotificationMutation = {
  DeleteNotification: { Success: boolean; Error: string | null };
};

export type PlaybackSessionsQueryVariables = Exact<{
  Where?: PlaybackSessionWhereInput | null | undefined;
  OrderBy?:
    | Array<PlaybackSessionOrderByInput>
    | PlaybackSessionOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type PlaybackSessionsQuery = {
  PlaybackSessions: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        UserId: string;
        MediaFileId: string | null;
        CurrentPosition: number;
        Duration: number | null;
        Volume: number;
        IsMuted: boolean;
        IsPlaying: boolean;
        StartedAt: string;
        LastUpdatedAt: string;
        CompletedAt: string | null;
        CreatedAt: string;
        UpdatedAt: string;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount: number | null };
  };
};

export type ShowPlaybackProgressByMediaQueryVariables = Exact<{
  Where?: PlaybackProgressWhereInput | null | undefined;
  Page?: PageInput | null | undefined;
  OrderBy?:
    | Array<PlaybackProgressOrderByInput>
    | PlaybackProgressOrderByInput
    | null
    | undefined;
}>;

export type ShowPlaybackProgressByMediaQuery = {
  PlaybackProgresses: {
    Edges: Array<{
      Node: {
        Id: string;
        MediaFileId: string | null;
        CurrentPosition: number;
        Duration: number | null;
        ProgressPercent: number;
        IsWatched: boolean;
        UpdatedAt: string;
      };
    }>;
  };
};

export type PlaybackProgressByMediaFileContextQueryVariables = Exact<{
  Where?: PlaybackProgressWhereInput | null | undefined;
  Page?: PageInput | null | undefined;
  OrderBy?:
    | Array<PlaybackProgressOrderByInput>
    | PlaybackProgressOrderByInput
    | null
    | undefined;
}>;

export type PlaybackProgressByMediaFileContextQuery = {
  PlaybackProgresses: {
    Edges: Array<{
      Node: {
        Id: string;
        UserId: string;
        MediaFileId: string | null;
        CurrentPosition: number;
        Duration: number | null;
        ProgressPercent: number;
        IsWatched: boolean;
        WatchedAt: string | null;
        CreatedAt: string;
        UpdatedAt: string;
      };
    }>;
  };
};

export type CreatePlaybackSessionContextMutationVariables = Exact<{
  Input: CreatePlaybackSessionInput;
}>;

export type CreatePlaybackSessionContextMutation = {
  CreatePlaybackSession: {
    Success: boolean;
    Error: string | null;
    PlaybackSession: {
      Id: string;
      UserId: string;
      ContentType: string | null;
      MediaFileId: string | null;
      EpisodeId: string | null;
      MovieId: string | null;
      TrackId: string | null;
      AudiobookId: string | null;
      TvShowId: string | null;
      AlbumId: string | null;
      CurrentPosition: number;
      Duration: number | null;
      Volume: number;
      IsMuted: boolean;
      IsPlaying: boolean;
      StartedAt: string;
      LastUpdatedAt: string;
      CompletedAt: string | null;
      CreatedAt: string;
      UpdatedAt: string;
    } | null;
  };
};

export type UpdatePlaybackSessionContextMutationVariables = Exact<{
  Id: string;
  Input: UpdatePlaybackSessionInput;
}>;

export type UpdatePlaybackSessionContextMutation = {
  UpdatePlaybackSession: {
    Success: boolean;
    Error: string | null;
    PlaybackSession: {
      Id: string;
      UserId: string;
      ContentType: string | null;
      MediaFileId: string | null;
      EpisodeId: string | null;
      MovieId: string | null;
      TrackId: string | null;
      AudiobookId: string | null;
      TvShowId: string | null;
      AlbumId: string | null;
      CurrentPosition: number;
      Duration: number | null;
      Volume: number;
      IsMuted: boolean;
      IsPlaying: boolean;
      StartedAt: string;
      LastUpdatedAt: string;
      CompletedAt: string | null;
      CreatedAt: string;
      UpdatedAt: string;
    } | null;
  };
};

export type CreatePlaybackProgressContextMutationVariables = Exact<{
  Input: CreatePlaybackProgressInput;
}>;

export type CreatePlaybackProgressContextMutation = {
  CreatePlaybackProgress: {
    Success: boolean;
    Error: string | null;
    PlaybackProgress: {
      Id: string;
      UserId: string;
      MediaFileId: string | null;
      CurrentPosition: number;
      Duration: number | null;
      ProgressPercent: number;
      IsWatched: boolean;
      WatchedAt: string | null;
      CreatedAt: string;
      UpdatedAt: string;
    } | null;
  };
};

export type UpdatePlaybackProgressContextMutationVariables = Exact<{
  Id: string;
  Input: UpdatePlaybackProgressInput;
}>;

export type UpdatePlaybackProgressContextMutation = {
  UpdatePlaybackProgress: {
    Success: boolean;
    Error: string | null;
    PlaybackProgress: {
      Id: string;
      UserId: string;
      MediaFileId: string | null;
      CurrentPosition: number;
      Duration: number | null;
      ProgressPercent: number;
      IsWatched: boolean;
      WatchedAt: string | null;
      CreatedAt: string;
      UpdatedAt: string;
    } | null;
  };
};

export type LibrarySearchShowsQueryVariables = Exact<{ [key: string]: never }>;

export type LibrarySearchShowsQuery = {
  Shows: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string;
        Name: string;
        Year: number | null;
        PosterUrl: string | null;
      };
    }>;
  };
};

export type LibrarySearchMoviesQueryVariables = Exact<{ [key: string]: never }>;

export type LibrarySearchMoviesQuery = {
  Movies: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string;
        Title: string;
        Year: number | null;
        MediaFileId: string | null;
        Wanted: boolean;
      };
    }>;
  };
};

export type SearchTvShowsQueryVariables = Exact<{
  Query: string;
}>;

export type SearchTvShowsQuery = {
  SearchTvShows: Array<{
    Provider: string;
    ProviderId: number;
    Name: string;
    Year: number | null;
    Network: string | null;
    Overview: string | null;
    Status: string | null;
    PosterUrl: string | null;
    TvdbId: number | null;
    ImdbId: string | null;
    Score: number | null;
  }>;
};

export type AddTvShowMutationVariables = Exact<{
  LibraryId: string;
  Input: AddTvShowInput;
}>;

export type AddTvShowMutation = {
  AddTvShow: {
    Success: boolean;
    Error: string | null;
    Show: { Id: string; Name: string } | null;
  };
};

export type ShowChangedSubscriptionVariables = Exact<{
  Filter?: SubscriptionFilterInput | null | undefined;
}>;

export type ShowChangedSubscription = {
  ShowChanged: {
    Action: ChangeAction;
    Id: string;
    Show: { LibraryId: string } | null;
  };
};

export type ShowDetailRouteQueryVariables = Exact<{
  Id: string;
}>;

export type ShowDetailRouteQuery = {
  Show: {
    Id: string;
    LibraryId: string;
    Name: string;
    SortName: string | null;
    Year: number | null;
    TvmazeId: number | null;
    TmdbId: number | null;
    TvdbId: number | null;
    ImdbId: string | null;
    Overview: string | null;
    Network: string | null;
    PosterUrl: string | null;
    BackdropUrl: string | null;
    Runtime: number | null;
    Genres: Array<string>;
    AutoDownload: boolean;
    AutoDownloadMode: AutoDownloadMode;
    Path: string | null;
    CreatedAt: string;
    UpdatedAt: string;
    UserId: string;
    Episodes: {
      Edges: Array<{
        Node: {
          Id: string;
          ShowId: string;
          Season: number;
          Episode: number;
          AbsoluteNumber: number | null;
          Title: string | null;
          Overview: string | null;
          AirDate: string | null;
          Runtime: number | null;
          TvmazeId: number | null;
          TmdbId: number | null;
          TvdbId: number | null;
          MediaFileId: string | null;
          Wanted: boolean;
          CreatedAt: string;
          UpdatedAt: string;
          MediaFile: {
            Id: string;
            Size: number;
            Duration: number | null;
            Resolution: string | null;
            VideoCodec: string | null;
            AudioCodec: string | null;
            AudioChannels: string | null;
            IsHdr: boolean;
            HdrType: string | null;
          } | null;
        };
      }>;
    };
  } | null;
};

export type RefreshShowRouteMutationVariables = Exact<{
  Id: string;
}>;

export type RefreshShowRouteMutation = {
  RefreshShow: {
    Success: boolean;
    Error: string | null;
    Show: { Id: string; Name: string; Overview: string | null } | null;
  };
};

export type ShowDetailSetEpisodeWantedMutationVariables = Exact<{
  ShowId: string;
  Wanted: boolean;
}>;

export type ShowDetailSetEpisodeWantedMutation = {
  UpdateEpisodes: {
    success: boolean;
    error: string | null;
    affectedCount: number;
  };
};

export type SourcesQueryVariables = Exact<{
  Where?: SourceWhereInput | null | undefined;
  OrderBy?: Array<SourceOrderByInput> | SourceOrderByInput | null | undefined;
  Page?: PageInput | null | undefined;
}>;

export type SourcesQuery = {
  Sources: {
    Edges: Array<{
      Node: {
        Id: string;
        Name: string;
        SourceType: string;
        DefinitionId: string;
        Enabled: boolean;
        Priority: number;
        MediaTypes: string;
        SiteUrl: string | null;
        SupportsSearch: boolean;
        SupportsTvSearch: boolean;
        SupportsMovieSearch: boolean;
        SupportsMusicSearch: boolean;
        SupportsBookSearch: boolean;
        Settings: string | null;
        LastError: string | null;
        ErrorCount: number;
        LastSuccessAt: string | null;
        LastErrorAt: string | null;
        CreatedAt: string;
        UpdatedAt: string;
      };
    }>;
    PageInfo: {
      TotalCount: number | null;
      HasNextPage: boolean;
      HasPreviousPage: boolean;
    };
  };
};

export type AvailableSourceDefinitionsQueryVariables = Exact<{
  [key: string]: never;
}>;

export type AvailableSourceDefinitionsQuery = {
  AvailableSourceDefinitions: Array<{
    Id: string;
    Name: string;
    Description: string;
    SourceType: string;
    TrackerType: string;
    Language: string;
    SiteLink: string;
    RequiredCredentials: Array<string>;
  }>;
};

export type SourceSettingDefinitionsQueryVariables = Exact<{
  DefinitionId: string;
}>;

export type SourceSettingDefinitionsQuery = {
  SourceSettingDefinitions: Array<{
    Key: string;
    Label: string;
    SettingType: string;
    DefaultValue: string | null;
    Options: Array<{ Value: string; Label: string }> | null;
  }>;
};

export type SearchSourcesQueryVariables = Exact<{
  Input: SearchSourcesInput;
}>;

export type SearchSourcesQuery = {
  SearchSources: {
    TotalReleases: number;
    TotalElapsedMs: number;
    SourcesSearched: number;
    Sources: Array<{
      SourceId: string;
      SourceName: string;
      ElapsedMs: number;
      FromCache: boolean;
      Error: string | null;
      Releases: Array<{
        Title: string;
        Guid: string;
        Link: string | null;
        MagnetUri: string | null;
        InfoHash: string | null;
        Details: string | null;
        PublishDate: string;
        Categories: Array<number>;
        Size: number | null;
        SizeFormatted: string | null;
        Seeders: number | null;
        Leechers: number | null;
        Peers: number | null;
        Grabs: number | null;
        IsFreeleech: boolean;
        ImdbId: string | null;
        Poster: string | null;
        Description: string | null;
        SourceId: string | null;
        SourceName: string | null;
      }>;
    }>;
  };
};

export type CreateSourceMutationVariables = Exact<{
  Input: CreateSourceInput;
}>;

export type CreateSourceMutation = {
  CreateSource: { Success: boolean; Error: string | null };
};

export type UpdateSourceMutationVariables = Exact<{
  Id: string;
  Input: UpdateSourceInput;
}>;

export type UpdateSourceMutation = {
  UpdateSource: { Success: boolean; Error: string | null };
};

export type DeleteSourceMutationVariables = Exact<{
  Id: string;
}>;

export type DeleteSourceMutation = {
  DeleteSource: { Success: boolean; Error: string | null };
};

export type TestSourceMutationVariables = Exact<{
  Id: string;
}>;

export type TestSourceMutation = {
  TestSource: {
    Success: boolean;
    Error: string | null;
    ReleasesFound: number | null;
    ElapsedMs: number | null;
  };
};

export type UpdateSourcePrioritiesMutationVariables = Exact<{
  Input: UpdateSourcePrioritiesInput;
}>;

export type UpdateSourcePrioritiesMutation = {
  UpdateSourcePriorities: { Success: boolean; Error: string | null };
};

export type ActiveDownloadCountQueryVariables = Exact<{ [key: string]: never }>;

export type ActiveDownloadCountQuery = { ActiveDownloadCount: number };

export type TorrentModalMediaFilesByPathsQueryVariables = Exact<{
  Paths: Array<string> | string;
}>;

export type TorrentModalMediaFilesByPathsQuery = {
  MediaFiles: {
    Edges: Array<{
      Node: { Id: string; Path: string; Metadata: string | null };
    }>;
  };
};

export type DownloadsTorrentsQueryVariables = Exact<{
  Where?: TorrentWhereInput | null | undefined;
  Page?: PageInput | null | undefined;
}>;

export type DownloadsTorrentsQuery = {
  Torrents: {
    Edges: Array<{
      Node: {
        Id: string;
        InfoHash: string;
        Name: string;
        State: string;
        Progress: number;
        TotalBytes: number;
        DownloadedBytes: number;
        UploadedBytes: number;
        SavePath: string;
        AddedAt: string;
      };
    }>;
    PageInfo: { TotalCount: number | null; HasNextPage: boolean };
  };
};

export type TorrentByInfoHashWithFilesQueryVariables = Exact<{
  Where?: TorrentWhereInput | null | undefined;
  Page?: PageInput | null | undefined;
}>;

export type TorrentByInfoHashWithFilesQuery = {
  Torrents: {
    Edges: Array<{
      Node: {
        Id: string;
        InfoHash: string;
        Name: string;
        State: string;
        Progress: number;
        TotalBytes: number;
        DownloadedBytes: number;
        UploadedBytes: number;
        SavePath: string;
        AddedAt: string;
        Files: {
          Edges: Array<{
            Node: {
              FileIndex: number;
              FilePath: string;
              FileSize: number;
              DownloadedBytes: number;
              Progress: number;
            };
          }>;
        };
      };
    }>;
  };
};

export type PendingFileMatchesBySourceQueryVariables = Exact<{
  Where?: PendingFileMatchWhereInput | null | undefined;
  Page?: PageInput | null | undefined;
}>;

export type PendingFileMatchesBySourceQuery = {
  PendingFileMatches: {
    Edges: Array<{
      Node: {
        Id: string;
        SourceType: string;
        SourceId: string | null;
        SourceFileIndex: number | null;
        SourcePath: string;
        FileSize: number;
        EpisodeId: string | null;
        MovieId: string | null;
        TrackId: string | null;
        ChapterId: string | null;
        MatchType: string | null;
        MatchConfidence: number | null;
        ParsedResolution: string | null;
        ParsedCodec: string | null;
        ParsedSource: string | null;
        ParsedAudio: string | null;
        CopiedAt: string | null;
        CopyError: string | null;
      };
    }>;
  };
};

export type PauseTorrentByInfoHashMutationVariables = Exact<{
  InfoHash: string;
}>;

export type PauseTorrentByInfoHashMutation = {
  PauseTorrentByInfoHash: { Success: boolean; Error: string | null };
};

export type ResumeTorrentByInfoHashMutationVariables = Exact<{
  InfoHash: string;
}>;

export type ResumeTorrentByInfoHashMutation = {
  ResumeTorrentByInfoHash: { Success: boolean; Error: string | null };
};

export type RemoveTorrentByInfoHashMutationVariables = Exact<{
  InfoHash: string;
  DeleteFiles?: boolean | null | undefined;
}>;

export type RemoveTorrentByInfoHashMutation = {
  RemoveTorrentByInfoHash: { Success: boolean; Error: string | null };
};

export type ProcessSourceMutationVariables = Exact<{
  SourceType: string;
  SourceId: string;
}>;

export type ProcessSourceMutation = {
  ProcessSource: {
    Success: boolean;
    FilesProcessed: number;
    FilesFailed: number;
    Messages: Array<string>;
    Error: string | null;
  };
};

export type RematchSourceMutationVariables = Exact<{
  SourceType: string;
  SourceId: string;
  LibraryId?: string | null | undefined;
}>;

export type RematchSourceMutation = {
  RematchSource: { Success: boolean; MatchCount: number; Error: string | null };
};

export type LinkTorrentToLibraryMutationVariables = Exact<{
  Id: string;
  Input: UpdateTorrentInput;
}>;

export type LinkTorrentToLibraryMutation = {
  UpdateTorrent: {
    Success: boolean;
    Error: string | null;
    Torrent: { Id: string; LibraryId: string | null } | null;
  };
};

export type TorrentChangedSubscriptionVariables = Exact<{
  [key: string]: never;
}>;

export type TorrentChangedSubscription = { TorrentChanged: { Id: number } };

export type CreateUnmatchedMediaFileFromTorrentMutationVariables = Exact<{
  Input: CreateMediaFileInput;
}>;

export type CreateUnmatchedMediaFileFromTorrentMutation = {
  CreateMediaFile: {
    Success: boolean;
    Error: string | null;
    MediaFile: { Id: string; Path: string; Metadata: string | null } | null;
  };
};

export type AnalyzeMediaFileForTorrentMutationVariables = Exact<{
  MediaFileId: string;
  Path: string;
}>;

export type AnalyzeMediaFileForTorrentMutation = {
  AnalyzeMediaFile: {
    Success: boolean;
    Queued: boolean;
    Message: string | null;
  };
};

export type SettingsUsenetServersQueryVariables = Exact<{
  OrderBy?:
    | Array<UsenetServerOrderByInput>
    | UsenetServerOrderByInput
    | null
    | undefined;
  Page?: PageInput | null | undefined;
}>;

export type SettingsUsenetServersQuery = {
  UsenetServers: {
    Edges: Array<{
      Node: {
        Id: string;
        Name: string;
        Host: string;
        Port: number;
        UseSsl: boolean;
        Username: string | null;
        Connections: number;
        Priority: number;
        Enabled: boolean;
        RetentionDays: number | null;
        LastSuccessAt: string | null;
        LastError: string | null;
        ErrorCount: number;
      };
    }>;
  };
};

export type SettingsUpdateUsenetServerMutationVariables = Exact<{
  Id: string;
  Input: UpdateUsenetServerInput;
}>;

export type SettingsUpdateUsenetServerMutation = {
  UpdateUsenetServer: {
    Success: boolean;
    Error: string | null;
    UsenetServer: { Id: string; Enabled: boolean; Priority: number } | null;
  };
};

export type SettingsCreateUsenetServerMutationVariables = Exact<{
  Input: CreateUsenetServerInput;
}>;

export type SettingsCreateUsenetServerMutation = {
  CreateUsenetServer: {
    Success: boolean;
    Error: string | null;
    UsenetServer: {
      Id: string;
      Name: string;
      Host: string;
      Port: number;
      UseSsl: boolean;
      Username: string | null;
      Connections: number;
      Priority: number;
      Enabled: boolean;
      RetentionDays: number | null;
      LastSuccessAt: string | null;
      LastError: string | null;
      ErrorCount: number;
    } | null;
  };
};

export type SettingsDeleteUsenetServerMutationVariables = Exact<{
  Id: string;
}>;

export type SettingsDeleteUsenetServerMutation = {
  DeleteUsenetServer: { Success: boolean; Error: string | null };
};

export const PlaybackSyncIntervalDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "PlaybackSyncInterval" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Key" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "AppSettings" },
            name: { kind: "Name", value: "appSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Key" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Key" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "1" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Key" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Value" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  PlaybackSyncIntervalQuery,
  PlaybackSyncIntervalQueryVariables
>;
export const TorrentAppSettingsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "TorrentAppSettings" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "AppSettings" },
            name: { kind: "Name", value: "appSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Category" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "StringValue",
                              value: "torrent",
                              block: false,
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "20" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Key" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Value" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  TorrentAppSettingsQuery,
  TorrentAppSettingsQueryVariables
>;
export const MetadataAppSettingsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "MetadataAppSettings" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "AppSettings" },
            name: { kind: "Name", value: "appSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Category" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "StringValue",
                              value: "metadata",
                              block: false,
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "50" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Key" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Value" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  MetadataAppSettingsQuery,
  MetadataAppSettingsQueryVariables
>;
export const LlmAppSettingsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LlmAppSettings" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "AppSettings" },
            name: { kind: "Name", value: "appSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Category" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "StringValue",
                              value: "llm",
                              block: false,
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "100" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Key" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Value" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<LlmAppSettingsQuery, LlmAppSettingsQueryVariables>;
export const CreateAppSettingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreateAppSetting" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateAppSettingInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateAppSetting" },
            name: { kind: "Name", value: "createAppSetting" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "AppSetting" },
                  name: { kind: "Name", value: "appSetting" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Key" } },
                      { kind: "Field", name: { kind: "Name", value: "Value" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreateAppSettingMutation,
  CreateAppSettingMutationVariables
>;
export const UpdateAppSettingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdateAppSetting" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateAppSettingInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateAppSetting" },
            name: { kind: "Name", value: "updateAppSetting" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "AppSetting" },
                  name: { kind: "Name", value: "appSetting" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Key" } },
                      { kind: "Field", name: { kind: "Name", value: "Value" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdateAppSettingMutation,
  UpdateAppSettingMutationVariables
>;
export const NeedsSetupDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "NeedsSetup" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          { kind: "Field", name: { kind: "Name", value: "NeedsSetup" } },
        ],
      },
    },
  ],
} as unknown as DocumentNode<NeedsSetupQuery, NeedsSetupQueryVariables>;
export const MeDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "Me" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "Me" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "Email" } },
                { kind: "Field", name: { kind: "Name", value: "Username" } },
                { kind: "Field", name: { kind: "Name", value: "Role" } },
                { kind: "Field", name: { kind: "Name", value: "DisplayName" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<MeQuery, MeQueryVariables>;
export const LoginDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "Login" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "LoginInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "Login" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "User" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Id" },
                        name: { kind: "Name", value: "id" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Email" },
                        name: { kind: "Name", value: "email" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Username" },
                        name: { kind: "Name", value: "username" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Role" },
                        name: { kind: "Name", value: "role" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "DisplayName" },
                        name: { kind: "Name", value: "displayName" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Tokens" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "AccessToken" },
                        name: { kind: "Name", value: "accessToken" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "RefreshToken" },
                        name: { kind: "Name", value: "refreshToken" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "ExpiresIn" },
                        name: { kind: "Name", value: "expiresIn" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TokenType" },
                        name: { kind: "Name", value: "tokenType" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<LoginMutation, LoginMutationVariables>;
export const RegisterDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "Register" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "RegisterUserInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "Register" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "User" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Id" },
                        name: { kind: "Name", value: "id" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Email" },
                        name: { kind: "Name", value: "email" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Username" },
                        name: { kind: "Name", value: "username" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Role" },
                        name: { kind: "Name", value: "role" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "DisplayName" },
                        name: { kind: "Name", value: "displayName" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Tokens" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "AccessToken" },
                        name: { kind: "Name", value: "accessToken" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "RefreshToken" },
                        name: { kind: "Name", value: "refreshToken" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "ExpiresIn" },
                        name: { kind: "Name", value: "expiresIn" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TokenType" },
                        name: { kind: "Name", value: "tokenType" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<RegisterMutation, RegisterMutationVariables>;
export const RefreshTokenDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "RefreshToken" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "RefreshTokenInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "RefreshToken" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Tokens" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "AccessToken" },
                        name: { kind: "Name", value: "accessToken" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "RefreshToken" },
                        name: { kind: "Name", value: "refreshToken" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "ExpiresIn" },
                        name: { kind: "Name", value: "expiresIn" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TokenType" },
                        name: { kind: "Name", value: "tokenType" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  RefreshTokenMutation,
  RefreshTokenMutationVariables
>;
export const LogoutDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "Logout" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "LogoutInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "Logout" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<LogoutMutation, LogoutMutationVariables>;
export const CastDevicesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "CastDevices" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "CastDeviceWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "CastDeviceOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CastDevices" },
            name: { kind: "Name", value: "castDevices" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Address" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Port" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Model" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DeviceType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsFavorite" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsManual" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastSeenAt" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastDevicesQuery, CastDevicesQueryVariables>;
export const CastSessionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "CastSessions" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "CastSessionWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "CastSessionOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CastSessions" },
            name: { kind: "Name", value: "castSessions" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DeviceId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaFileId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EpisodeId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "StreamUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "PlayerState" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CurrentPosition" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Duration" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Volume" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsMuted" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "StartedAt" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastSessionsQuery, CastSessionsQueryVariables>;
export const CastSettingsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "CastSettings" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "CastSettingWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "CastSettingOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CastSettings" },
            name: { kind: "Name", value: "castSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "AutoDiscoveryEnabled",
                              },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "DiscoveryIntervalSeconds",
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DefaultVolume" },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "TranscodeIncompatible",
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "PreferredQuality" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastSettingsQuery, CastSettingsQueryVariables>;
export const CreateCastDeviceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreateCastDevice" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateCastDeviceInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateCastDevice" },
            name: { kind: "Name", value: "createCastDevice" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "CastDevice" },
                  name: { kind: "Name", value: "castDevice" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Address" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Port" } },
                      { kind: "Field", name: { kind: "Name", value: "Model" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "DeviceType" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsFavorite" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsManual" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LastSeenAt" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreateCastDeviceMutation,
  CreateCastDeviceMutationVariables
>;
export const UpdateCastDeviceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdateCastDevice" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateCastDeviceInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateCastDevice" },
            name: { kind: "Name", value: "updateCastDevice" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "CastDevice" },
                  name: { kind: "Name", value: "castDevice" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Address" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Port" } },
                      { kind: "Field", name: { kind: "Name", value: "Model" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "DeviceType" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsFavorite" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsManual" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LastSeenAt" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdateCastDeviceMutation,
  UpdateCastDeviceMutationVariables
>;
export const DeleteCastDeviceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteCastDevice" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteCastDevice" },
            name: { kind: "Name", value: "deleteCastDevice" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteCastDeviceMutation,
  DeleteCastDeviceMutationVariables
>;
export const CreateCastSettingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreateCastSetting" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateCastSettingInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateCastSetting" },
            name: { kind: "Name", value: "createCastSetting" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "CastSetting" },
                  name: { kind: "Name", value: "castSetting" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "AutoDiscoveryEnabled" },
                      },
                      {
                        kind: "Field",
                        name: {
                          kind: "Name",
                          value: "DiscoveryIntervalSeconds",
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "DefaultVolume" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TranscodeIncompatible" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "PreferredQuality" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreateCastSettingMutation,
  CreateCastSettingMutationVariables
>;
export const UpdateCastSettingDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdateCastSetting" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateCastSettingInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateCastSetting" },
            name: { kind: "Name", value: "updateCastSetting" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "CastSetting" },
                  name: { kind: "Name", value: "castSetting" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "AutoDiscoveryEnabled" },
                      },
                      {
                        kind: "Field",
                        name: {
                          kind: "Name",
                          value: "DiscoveryIntervalSeconds",
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "DefaultVolume" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TranscodeIncompatible" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "PreferredQuality" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdateCastSettingMutation,
  UpdateCastSettingMutationVariables
>;
export const DiscoverCastDevicesOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DiscoverCastDevicesOp" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "DiscoverCastDevices" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "id" } },
                { kind: "Field", name: { kind: "Name", value: "name" } },
                { kind: "Field", name: { kind: "Name", value: "address" } },
                { kind: "Field", name: { kind: "Name", value: "port" } },
                { kind: "Field", name: { kind: "Name", value: "model" } },
                { kind: "Field", name: { kind: "Name", value: "deviceType" } },
                { kind: "Field", name: { kind: "Name", value: "isFavorite" } },
                { kind: "Field", name: { kind: "Name", value: "isManual" } },
                { kind: "Field", name: { kind: "Name", value: "isConnected" } },
                { kind: "Field", name: { kind: "Name", value: "lastSeenAt" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DiscoverCastDevicesOpMutation,
  DiscoverCastDevicesOpMutationVariables
>;
export const CastMediaOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CastMediaOp" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CastMediaInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "CastMedia" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "session" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "deviceId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "deviceName" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "mediaFileId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "episodeId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "streamUrl" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "playerState" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "currentTime" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "duration" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "volume" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "isMuted" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "startedAt" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastMediaOpMutation, CastMediaOpMutationVariables>;
export const CastPlayOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CastPlayOp" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "sessionId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "CastPlay" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "sessionId" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "session" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "playerState" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "currentTime" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastPlayOpMutation, CastPlayOpMutationVariables>;
export const CastPauseOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CastPauseOp" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "sessionId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "CastPause" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "sessionId" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "session" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "playerState" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "currentTime" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastPauseOpMutation, CastPauseOpMutationVariables>;
export const CastStopOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CastStopOp" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "sessionId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "CastStop" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "sessionId" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                { kind: "Field", name: { kind: "Name", value: "error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastStopOpMutation, CastStopOpMutationVariables>;
export const CastSeekOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CastSeekOp" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "sessionId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "position" },
          },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "Float" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "CastSeek" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "sessionId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "position" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "position" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "session" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "playerState" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "currentTime" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<CastSeekOpMutation, CastSeekOpMutationVariables>;
export const CastSetVolumeOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CastSetVolumeOp" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "sessionId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "volume" },
          },
          type: {
            kind: "NonNullType",
            type: { kind: "NamedType", name: { kind: "Name", value: "Float" } },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "CastSetVolume" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "sessionId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "volume" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "volume" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "session" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "volume" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "isMuted" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CastSetVolumeOpMutation,
  CastSetVolumeOpMutationVariables
>;
export const CastSetMutedOpDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CastSetMutedOp" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "sessionId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "muted" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "Boolean" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "CastSetMuted" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "sessionId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "sessionId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "muted" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "muted" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "session" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "volume" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "isMuted" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CastSetMutedOpMutation,
  CastSetMutedOpMutationVariables
>;
export const DashboardShowsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "DashboardShows" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "ShowWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "ShowOrderByInput" },
              },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Shows" },
            name: { kind: "Name", value: "shows" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SortName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Year" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TvmazeId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TmdbId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TvdbId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ImdbId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Overview" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Network" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Runtime" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "PosterUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "BackdropUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Path" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Genres" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CreatedAt" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<DashboardShowsQuery, DashboardShowsQueryVariables>;
export const DashboardScheduleCachesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "DashboardScheduleCaches" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "ScheduleCacheWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "ScheduleCacheOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "ScheduleCaches" },
            name: { kind: "Name", value: "scheduleCaches" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TvmazeEpisodeId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EpisodeName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Season" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EpisodeNumber" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EpisodeType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AirDate" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AirTime" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AirStamp" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Runtime" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EpisodeImageUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Summary" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TvmazeShowId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ShowName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ShowNetwork" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ShowPosterUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ShowGenres" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CountryCode" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DashboardScheduleCachesQuery,
  DashboardScheduleCachesQueryVariables
>;
export const MediaFilePropertiesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "MediaFileProperties" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "MediaFile" },
            name: { kind: "Name", value: "mediaFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "LibraryId" } },
                { kind: "Field", name: { kind: "Name", value: "Path" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "RelativePath" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "OriginalName" },
                },
                { kind: "Field", name: { kind: "Name", value: "Size" } },
                { kind: "Field", name: { kind: "Name", value: "Container" } },
                { kind: "Field", name: { kind: "Name", value: "VideoCodec" } },
                { kind: "Field", name: { kind: "Name", value: "AudioCodec" } },
                { kind: "Field", name: { kind: "Name", value: "Resolution" } },
                { kind: "Field", name: { kind: "Name", value: "IsHdr" } },
                { kind: "Field", name: { kind: "Name", value: "HdrType" } },
                { kind: "Field", name: { kind: "Name", value: "Width" } },
                { kind: "Field", name: { kind: "Name", value: "Height" } },
                { kind: "Field", name: { kind: "Name", value: "Duration" } },
                { kind: "Field", name: { kind: "Name", value: "Bitrate" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "AudioChannels" },
                },
                { kind: "Field", name: { kind: "Name", value: "EpisodeId" } },
                { kind: "Field", name: { kind: "Name", value: "MovieId" } },
                { kind: "Field", name: { kind: "Name", value: "TrackId" } },
                { kind: "Field", name: { kind: "Name", value: "ContentType" } },
                { kind: "Field", name: { kind: "Name", value: "AddedAt" } },
              ],
            },
          },
          {
            kind: "Field",
            alias: { kind: "Name", value: "VideoStreams" },
            name: { kind: "Name", value: "videoStreams" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "MediaFileId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Id" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "StreamIndex" },
                          value: { kind: "EnumValue", value: "ASC" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "200" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "StreamIndex" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Codec" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CodecLongName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Width" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Height" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AspectRatio" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "FrameRate" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Bitrate" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "PixelFormat" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "HdrType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "BitDepth" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Language" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsDefault" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            alias: { kind: "Name", value: "AudioStreams" },
            name: { kind: "Name", value: "audioStreams" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "MediaFileId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Id" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "StreamIndex" },
                          value: { kind: "EnumValue", value: "ASC" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "200" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "StreamIndex" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Codec" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CodecLongName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Channels" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ChannelLayout" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SampleRate" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Bitrate" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "BitDepth" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Language" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsDefault" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsCommentary" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            alias: { kind: "Name", value: "Subtitles" },
            name: { kind: "Name", value: "subtitles" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "MediaFileId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Id" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "CreatedAt" },
                          value: { kind: "EnumValue", value: "ASC" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "200" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "StreamIndex" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourceType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Codec" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CodecLongName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Language" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsDefault" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsForced" },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "IsHearingImpaired",
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "FilePath" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            alias: { kind: "Name", value: "MediaChapters" },
            name: { kind: "Name", value: "mediaChapters" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "MediaFileId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Id" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "ChapterIndex" },
                          value: { kind: "EnumValue", value: "ASC" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ChapterIndex" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "StartSecs" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EndSecs" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  MediaFilePropertiesQuery,
  MediaFilePropertiesQueryVariables
>;
export const MediaFileByPathLookupDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "MediaFileByPathLookup" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Path" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "MediaFiles" },
            name: { kind: "Name", value: "mediaFiles" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Path" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Path" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "1" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Path" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  MediaFileByPathLookupQuery,
  MediaFileByPathLookupQueryVariables
>;
export const MediaFileMetadataDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "MediaFileMetadata" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "MediaFile" },
            name: { kind: "Name", value: "mediaFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "Metadata" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  MediaFileMetadataQuery,
  MediaFileMetadataQueryVariables
>;
export const BrowseDirectoryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "BrowseDirectory" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "BrowseDirectoryInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "BrowseDirectory" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "CurrentPath" } },
                { kind: "Field", name: { kind: "Name", value: "ParentPath" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Entries" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      { kind: "Field", name: { kind: "Name", value: "Path" } },
                      { kind: "Field", name: { kind: "Name", value: "IsDir" } },
                      { kind: "Field", name: { kind: "Name", value: "Size" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "SizeFormatted" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Readable" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Writable" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MimeType" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ModifiedAt" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "QuickPaths" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      { kind: "Field", name: { kind: "Name", value: "Path" } },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "IsLibraryPath" },
                },
                { kind: "Field", name: { kind: "Name", value: "LibraryId" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  BrowseDirectoryQuery,
  BrowseDirectoryQueryVariables
>;
export const FilesystemRuntimeInfoDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "FilesystemRuntimeInfo" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "FilesystemRuntimeInfo" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Platform" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "SupportsUncCredentials" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "SupportsSambaMount" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "DefaultLinuxMountBase" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  FilesystemRuntimeInfoQuery,
  FilesystemRuntimeInfoQueryVariables
>;
export const LibraryPathAvailabilityDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibraryPathAvailability" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "LibraryPathAvailabilityInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "LibraryPathAvailability" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Path" } },
                { kind: "Field", name: { kind: "Name", value: "Reachable" } },
                { kind: "Field", name: { kind: "Name", value: "Exists" } },
                { kind: "Field", name: { kind: "Name", value: "IsDirectory" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "NeedsReconnect" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ReconnectAttempted" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ReconnectSucceeded" },
                },
                { kind: "Field", name: { kind: "Name", value: "Message" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibraryPathAvailabilityQuery,
  LibraryPathAvailabilityQueryVariables
>;
export const ConfigureNetworkPathDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "ConfigureNetworkPath" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "ConfigureNetworkPathInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "ConfigureNetworkPath" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ResolvedPath" },
                },
                { kind: "Field", name: { kind: "Name", value: "Connected" } },
                { kind: "Field", name: { kind: "Name", value: "Stored" } },
                { kind: "Field", name: { kind: "Name", value: "Message" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ConfigureNetworkPathMutation,
  ConfigureNetworkPathMutationVariables
>;
export const ReconnectLibraryPathDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "ReconnectLibraryPath" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Path" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "ReconnectLibraryPath" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Path" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Path" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ResolvedPath" },
                },
                { kind: "Field", name: { kind: "Name", value: "Connected" } },
                { kind: "Field", name: { kind: "Name", value: "Stored" } },
                { kind: "Field", name: { kind: "Name", value: "Message" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ReconnectLibraryPathMutation,
  ReconnectLibraryPathMutationVariables
>;
export const LibrariesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "Libraries" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "LibraryWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "LibraryOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Libraries" },
            name: { kind: "Name", value: "libraries" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UserId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Path" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Icon" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Color" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AutoScan" },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "ScanIntervalMinutes",
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "WatchForChanges" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AutoOrganize" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "NamingPattern" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Scanning" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastScannedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CreatedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UpdatedAt" },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Shows" },
                              name: { kind: "Name", value: "shows" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "PageInfo" },
                                    name: { kind: "Name", value: "pageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "TotalCount",
                                          },
                                          name: {
                                            kind: "Name",
                                            value: "totalCount",
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "ShowArtwork" },
                              name: { kind: "Name", value: "shows" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "orderBy" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: {
                                          kind: "Name",
                                          value: "UpdatedAt",
                                        },
                                        value: {
                                          kind: "EnumValue",
                                          value: "DESC",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "offset" },
                                        value: { kind: "IntValue", value: "0" },
                                      },
                                    ],
                                  },
                                },
                              ],
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "Edges" },
                                    name: { kind: "Name", value: "edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "Node",
                                          },
                                          name: { kind: "Name", value: "node" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Id",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "PosterUrl",
                                                },
                                              },
                                            ],
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Movies" },
                              name: { kind: "Name", value: "movies" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "PageInfo" },
                                    name: { kind: "Name", value: "pageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "TotalCount",
                                          },
                                          name: {
                                            kind: "Name",
                                            value: "totalCount",
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "MovieArtwork" },
                              name: { kind: "Name", value: "movies" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "orderBy" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: {
                                          kind: "Name",
                                          value: "UpdatedAt",
                                        },
                                        value: {
                                          kind: "EnumValue",
                                          value: "DESC",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "offset" },
                                        value: { kind: "IntValue", value: "0" },
                                      },
                                    ],
                                  },
                                },
                              ],
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "Edges" },
                                    name: { kind: "Name", value: "edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "Node",
                                          },
                                          name: { kind: "Name", value: "node" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Id",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "CollectionPosterUrl",
                                                },
                                              },
                                            ],
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Albums" },
                              name: { kind: "Name", value: "albums" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "PageInfo" },
                                    name: { kind: "Name", value: "pageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "TotalCount",
                                          },
                                          name: {
                                            kind: "Name",
                                            value: "totalCount",
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "AlbumArtwork" },
                              name: { kind: "Name", value: "albums" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "orderBy" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: {
                                          kind: "Name",
                                          value: "UpdatedAt",
                                        },
                                        value: {
                                          kind: "EnumValue",
                                          value: "DESC",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "offset" },
                                        value: { kind: "IntValue", value: "0" },
                                      },
                                    ],
                                  },
                                },
                              ],
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "Edges" },
                                    name: { kind: "Name", value: "edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "Node",
                                          },
                                          name: { kind: "Name", value: "node" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Id",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "CoverUrl",
                                                },
                                              },
                                            ],
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Audiobooks" },
                              name: { kind: "Name", value: "audiobooks" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "PageInfo" },
                                    name: { kind: "Name", value: "pageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "TotalCount",
                                          },
                                          name: {
                                            kind: "Name",
                                            value: "totalCount",
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                            {
                              kind: "Field",
                              alias: {
                                kind: "Name",
                                value: "AudiobookArtwork",
                              },
                              name: { kind: "Name", value: "audiobooks" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "orderBy" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: {
                                          kind: "Name",
                                          value: "UpdatedAt",
                                        },
                                        value: {
                                          kind: "EnumValue",
                                          value: "DESC",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "offset" },
                                        value: { kind: "IntValue", value: "0" },
                                      },
                                    ],
                                  },
                                },
                              ],
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "Edges" },
                                    name: { kind: "Name", value: "edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "Node",
                                          },
                                          name: { kind: "Name", value: "node" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Id",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "CoverUrl",
                                                },
                                              },
                                            ],
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<LibrariesQuery, LibrariesQueryVariables>;
export const LibraryChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "LibraryChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Filter" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "SubscriptionFilterInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "LibraryChanged" },
            name: { kind: "Name", value: "libraryChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Action" },
                  name: { kind: "Name", value: "action" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Id" },
                  name: { kind: "Name", value: "id" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Library" },
                  name: { kind: "Name", value: "library" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      { kind: "Field", name: { kind: "Name", value: "Path" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryType" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Icon" } },
                      { kind: "Field", name: { kind: "Name", value: "Color" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "AutoScan" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ScanIntervalMinutes" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "WatchForChanges" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Scanning" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LastScannedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CreatedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UpdatedAt" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibraryChangedSubscription,
  LibraryChangedSubscriptionVariables
>;
export const CreateLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreateLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateLibraryInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateLibrary" },
            name: { kind: "Name", value: "createLibrary" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Library" },
                  name: { kind: "Name", value: "library" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      { kind: "Field", name: { kind: "Name", value: "Path" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryType" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Icon" } },
                      { kind: "Field", name: { kind: "Name", value: "Color" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreateLibraryMutation,
  CreateLibraryMutationVariables
>;
export const DeleteLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteLibrary" },
            name: { kind: "Name", value: "deleteLibrary" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteLibraryMutation,
  DeleteLibraryMutationVariables
>;
export const ScanLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "ScanLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "ScanLibrary" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Message" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<ScanLibraryMutation, ScanLibraryMutationVariables>;
export const LibraryAlbumsTabDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibraryAlbumsTab" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Albums" },
            name: { kind: "Name", value: "albums" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "5000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ArtistId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SortName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Year" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MusicbrainzId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AlbumType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Genres" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Label" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Country" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ReleaseDate" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CoverUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TrackCount" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DiscCount" },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "TotalDurationSecs",
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "HasFiles" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SizeBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Path" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibraryAlbumsTabQuery,
  LibraryAlbumsTabQueryVariables
>;
export const LibraryArtistsTabDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibraryArtistsTab" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Artists" },
            name: { kind: "Name", value: "artists" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "5000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SortName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MusicbrainzId" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibraryArtistsTabQuery,
  LibraryArtistsTabQueryVariables
>;
export const LibraryAudiobooksTabDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibraryAudiobooksTab" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Audiobooks" },
            name: { kind: "Name", value: "audiobooks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "5000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SortTitle" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Isbn" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Description" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Publisher" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Language" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Narrators" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CoverUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "HasFiles" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SizeBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Path" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ChapterCount" },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "TotalDurationSecs",
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AuthorName" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibraryAudiobooksTabQuery,
  LibraryAudiobooksTabQueryVariables
>;
export const LibraryUnmatchedMediaFilesTabDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibraryUnmatchedMediaFilesTab" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "MediaFiles" },
            name: { kind: "Name", value: "mediaFiles" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "EpisodeId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isNull" },
                            value: { kind: "BooleanValue", value: true },
                          },
                        ],
                      },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "MovieId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isNull" },
                            value: { kind: "BooleanValue", value: true },
                          },
                        ],
                      },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "TrackId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isNull" },
                            value: { kind: "BooleanValue", value: true },
                          },
                        ],
                      },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ChapterId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "isNull" },
                            value: { kind: "BooleanValue", value: true },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "AddedAt" },
                          value: { kind: "EnumValue", value: "DESC" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "2000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Path" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "RelativePath" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "OriginalName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Size" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Container" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "VideoCodec" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AudioCodec" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Resolution" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsHdr" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "HdrType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Width" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Height" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Duration" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EpisodeId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ChapterId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AddedAt" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibraryUnmatchedMediaFilesTabQuery,
  LibraryUnmatchedMediaFilesTabQueryVariables
>;
export const LibraryDetailRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibraryDetailRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Library" },
            name: { kind: "Name", value: "library" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "Name" } },
                { kind: "Field", name: { kind: "Name", value: "Path" } },
                { kind: "Field", name: { kind: "Name", value: "LibraryType" } },
                { kind: "Field", name: { kind: "Name", value: "AutoScan" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ScanIntervalMinutes" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "WatchForChanges" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "AutoOrganize" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "NamingPattern" },
                },
                { kind: "Field", name: { kind: "Name", value: "Scanning" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibraryDetailRouteQuery,
  LibraryDetailRouteQueryVariables
>;
export const UpdateLibraryRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdateLibraryRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateLibraryInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateLibrary" },
            name: { kind: "Name", value: "updateLibrary" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Library" },
                  name: { kind: "Name", value: "library" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdateLibraryRouteMutation,
  UpdateLibraryRouteMutationVariables
>;
export const DeleteShowRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteShowRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteShow" },
            name: { kind: "Name", value: "deleteShow" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteShowRouteMutation,
  DeleteShowRouteMutationVariables
>;
export const AppLogsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "AppLogs" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "AppLogWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "AppLogOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "AppLogs" },
            name: { kind: "Name", value: "appLogs" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Timestamp" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Level" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Target" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Message" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Fields" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SpanName" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<AppLogsQuery, AppLogsQueryVariables>;
export const AppLogChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "AppLogChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Filter" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "SubscriptionFilterInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "AppLogChanged" },
            name: { kind: "Name", value: "appLogChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Action" },
                  name: { kind: "Name", value: "action" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Id" },
                  name: { kind: "Name", value: "id" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "AppLog" },
                  name: { kind: "Name", value: "appLog" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Timestamp" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Level" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Target" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Message" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Fields" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "SpanName" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AppLogChangedSubscription,
  AppLogChangedSubscriptionVariables
>;
export const DeleteAppLogsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteAppLogs" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "AppLogWhereInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteAppLogs" },
            name: { kind: "Name", value: "deleteAppLogs" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                { kind: "Field", name: { kind: "Name", value: "error" } },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "DeletedCount" },
                  name: { kind: "Name", value: "deletedCount" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteAppLogsMutation,
  DeleteAppLogsMutationVariables
>;
export const ManualMatchShowsByLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ManualMatchShowsByLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Shows" },
            name: { kind: "Name", value: "shows" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Year" },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Episodes" },
                              name: { kind: "Name", value: "episodes" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "Edges" },
                                    name: { kind: "Name", value: "edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "Node",
                                          },
                                          name: { kind: "Name", value: "node" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Id",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Season",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Episode",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Title",
                                                },
                                              },
                                            ],
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ManualMatchShowsByLibraryQuery,
  ManualMatchShowsByLibraryQueryVariables
>;
export const ManualMatchMoviesByLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ManualMatchMoviesByLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Movies" },
            name: { kind: "Name", value: "movies" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Year" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ManualMatchMoviesByLibraryQuery,
  ManualMatchMoviesByLibraryQueryVariables
>;
export const ManualMatchAlbumsByLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ManualMatchAlbumsByLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Albums" },
            name: { kind: "Name", value: "albums" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Year" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
          {
            kind: "Field",
            alias: { kind: "Name", value: "Tracks" },
            name: { kind: "Name", value: "tracks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "5000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AlbumId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ArtistName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TrackNumber" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ManualMatchAlbumsByLibraryQuery,
  ManualMatchAlbumsByLibraryQueryVariables
>;
export const ManualMatchAudiobooksByLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ManualMatchAudiobooksByLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Audiobooks" },
            name: { kind: "Name", value: "audiobooks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "LibraryId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AuthorName" },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Chapters" },
                              name: { kind: "Name", value: "chapters" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "Edges" },
                                    name: { kind: "Name", value: "edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "Node",
                                          },
                                          name: { kind: "Name", value: "node" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Id",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "ChapterNumber",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Title",
                                                },
                                              },
                                            ],
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ManualMatchAudiobooksByLibraryQuery,
  ManualMatchAudiobooksByLibraryQueryVariables
>;
export const ManualMatchFileDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "ManualMatchFile" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "MatchMediaFileInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "MatchMediaFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Confidence" } },
                { kind: "Field", name: { kind: "Name", value: "MatchedId" } },
                { kind: "Field", name: { kind: "Name", value: "MatchedType" } },
                { kind: "Field", name: { kind: "Name", value: "Reason" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ManualMatchFileMutation,
  ManualMatchFileMutationVariables
>;
export const SearchAlbumsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SearchAlbums" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Query" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "IncludeEps" },
          },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "IncludeSingles" },
          },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "IncludeCompilations" },
          },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "IncludeLive" },
          },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "IncludeSoundtracks" },
          },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "SearchAlbums" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Query" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Query" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "IncludeEps" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "IncludeEps" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "IncludeSingles" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "IncludeSingles" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "IncludeCompilations" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "IncludeCompilations" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "IncludeLive" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "IncludeLive" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "IncludeSoundtracks" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "IncludeSoundtracks" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Provider" } },
                { kind: "Field", name: { kind: "Name", value: "ProviderId" } },
                { kind: "Field", name: { kind: "Name", value: "Title" } },
                { kind: "Field", name: { kind: "Name", value: "ArtistName" } },
                { kind: "Field", name: { kind: "Name", value: "Year" } },
                { kind: "Field", name: { kind: "Name", value: "AlbumType" } },
                { kind: "Field", name: { kind: "Name", value: "CoverUrl" } },
                { kind: "Field", name: { kind: "Name", value: "Score" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SearchAlbumsQuery, SearchAlbumsQueryVariables>;
export const SearchAudiobooksDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SearchAudiobooks" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Query" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "SearchAudiobooks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Query" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Query" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Provider" } },
                { kind: "Field", name: { kind: "Name", value: "ProviderId" } },
                { kind: "Field", name: { kind: "Name", value: "Title" } },
                { kind: "Field", name: { kind: "Name", value: "AuthorName" } },
                { kind: "Field", name: { kind: "Name", value: "Year" } },
                { kind: "Field", name: { kind: "Name", value: "CoverUrl" } },
                { kind: "Field", name: { kind: "Name", value: "Isbn" } },
                { kind: "Field", name: { kind: "Name", value: "Description" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  SearchAudiobooksQuery,
  SearchAudiobooksQueryVariables
>;
export const AddAlbumDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AddAlbum" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "AddAlbumInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AddAlbum" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<AddAlbumMutation, AddAlbumMutationVariables>;
export const AddAudiobookDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AddAudiobook" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "AddAudiobookInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AddAudiobook" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AddAudiobookMutation,
  AddAudiobookMutationVariables
>;
export const AddTorrentDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AddTorrent" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "AddTorrentInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AddTorrent" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Torrent" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<AddTorrentMutation, AddTorrentMutationVariables>;
export const AlbumDetailRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "AlbumDetailRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Album" },
            name: { kind: "Name", value: "album" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "ArtistId" } },
                { kind: "Field", name: { kind: "Name", value: "LibraryId" } },
                { kind: "Field", name: { kind: "Name", value: "Name" } },
                { kind: "Field", name: { kind: "Name", value: "SortName" } },
                { kind: "Field", name: { kind: "Name", value: "Year" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "MusicbrainzId" },
                },
                { kind: "Field", name: { kind: "Name", value: "AlbumType" } },
                { kind: "Field", name: { kind: "Name", value: "Genres" } },
                { kind: "Field", name: { kind: "Name", value: "Label" } },
                { kind: "Field", name: { kind: "Name", value: "Country" } },
                { kind: "Field", name: { kind: "Name", value: "ReleaseDate" } },
                { kind: "Field", name: { kind: "Name", value: "CoverUrl" } },
                { kind: "Field", name: { kind: "Name", value: "TrackCount" } },
                { kind: "Field", name: { kind: "Name", value: "DiscCount" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "TotalDurationSecs" },
                },
                { kind: "Field", name: { kind: "Name", value: "HasFiles" } },
                { kind: "Field", name: { kind: "Name", value: "SizeBytes" } },
                { kind: "Field", name: { kind: "Name", value: "Path" } },
              ],
            },
          },
          {
            kind: "Field",
            alias: { kind: "Name", value: "Tracks" },
            name: { kind: "Name", value: "tracks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "AlbumId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Id" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "5000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "DiscNumber" },
                          value: { kind: "EnumValue", value: "ASC" },
                        },
                      ],
                    },
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "TrackNumber" },
                          value: { kind: "EnumValue", value: "ASC" },
                        },
                      ],
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AlbumId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TrackNumber" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DiscNumber" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MusicbrainzId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Isrc" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DurationSecs" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Explicit" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ArtistName" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ArtistId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaFileId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Wanted" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AlbumDetailRouteQuery,
  AlbumDetailRouteQueryVariables
>;
export const DeleteAlbumRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteAlbumRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteAlbum" },
            name: { kind: "Name", value: "deleteAlbum" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteAlbumRouteMutation,
  DeleteAlbumRouteMutationVariables
>;
export const AlbumDetailSetTrackWantedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AlbumDetailSetTrackWanted" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "AlbumId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Wanted" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "Boolean" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateTracks" },
            name: { kind: "Name", value: "updateTracks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "AlbumId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "AlbumId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Wanted" },
                      value: {
                        kind: "Variable",
                        name: { kind: "Name", value: "Wanted" },
                      },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                { kind: "Field", name: { kind: "Name", value: "error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "affectedCount" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AlbumDetailSetTrackWantedMutation,
  AlbumDetailSetTrackWantedMutationVariables
>;
export const AudiobookDetailRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "AudiobookDetailRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Audiobook" },
            name: { kind: "Name", value: "audiobook" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "LibraryId" } },
                { kind: "Field", name: { kind: "Name", value: "Title" } },
                { kind: "Field", name: { kind: "Name", value: "SortTitle" } },
                { kind: "Field", name: { kind: "Name", value: "Isbn" } },
                { kind: "Field", name: { kind: "Name", value: "Description" } },
                { kind: "Field", name: { kind: "Name", value: "Publisher" } },
                { kind: "Field", name: { kind: "Name", value: "Language" } },
                { kind: "Field", name: { kind: "Name", value: "Narrators" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "TotalDurationSecs" },
                },
                { kind: "Field", name: { kind: "Name", value: "CoverUrl" } },
                { kind: "Field", name: { kind: "Name", value: "HasFiles" } },
                { kind: "Field", name: { kind: "Name", value: "SizeBytes" } },
                { kind: "Field", name: { kind: "Name", value: "Path" } },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Chapters" },
                  name: { kind: "Name", value: "chapters" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "page" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "limit" },
                            value: { kind: "IntValue", value: "5000" },
                          },
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "offset" },
                            value: { kind: "IntValue", value: "0" },
                          },
                        ],
                      },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Edges" },
                        name: { kind: "Name", value: "edges" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Node" },
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Id" },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "AudiobookId",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "ChapterNumber",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Title" },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "StartTimeSecs",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "EndTimeSecs",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "DurationSecs",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "MediaFileId",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Wanted" },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AudiobookDetailRouteQuery,
  AudiobookDetailRouteQueryVariables
>;
export const DeleteAudiobookRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteAudiobookRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteAudiobook" },
            name: { kind: "Name", value: "deleteAudiobook" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteAudiobookRouteMutation,
  DeleteAudiobookRouteMutationVariables
>;
export const AudiobookDetailSetChapterWantedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AudiobookDetailSetChapterWanted" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "AudiobookId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Wanted" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "Boolean" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateChapters" },
            name: { kind: "Name", value: "updateChapters" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "AudiobookId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "AudiobookId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Wanted" },
                      value: {
                        kind: "Variable",
                        name: { kind: "Name", value: "Wanted" },
                      },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                { kind: "Field", name: { kind: "Name", value: "error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "affectedCount" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AudiobookDetailSetChapterWantedMutation,
  AudiobookDetailSetChapterWantedMutationVariables
>;
export const SearchMoviesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SearchMovies" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Query" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Year" } },
          type: { kind: "NamedType", name: { kind: "Name", value: "Int" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "SearchMovies" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Query" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Query" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Year" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Year" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Provider" } },
                { kind: "Field", name: { kind: "Name", value: "ProviderId" } },
                { kind: "Field", name: { kind: "Name", value: "Title" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "OriginalTitle" },
                },
                { kind: "Field", name: { kind: "Name", value: "Year" } },
                { kind: "Field", name: { kind: "Name", value: "Overview" } },
                { kind: "Field", name: { kind: "Name", value: "PosterUrl" } },
                { kind: "Field", name: { kind: "Name", value: "BackdropUrl" } },
                { kind: "Field", name: { kind: "Name", value: "ImdbId" } },
                { kind: "Field", name: { kind: "Name", value: "VoteAverage" } },
                { kind: "Field", name: { kind: "Name", value: "Popularity" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SearchMoviesQuery, SearchMoviesQueryVariables>;
export const SearchMovieCollectionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SearchMovieCollections" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Query" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "SearchMovieCollections" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Query" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Query" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Provider" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CollectionId" },
                },
                { kind: "Field", name: { kind: "Name", value: "Name" } },
                { kind: "Field", name: { kind: "Name", value: "Overview" } },
                { kind: "Field", name: { kind: "Name", value: "PosterUrl" } },
                { kind: "Field", name: { kind: "Name", value: "BackdropUrl" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  SearchMovieCollectionsQuery,
  SearchMovieCollectionsQueryVariables
>;
export const AddMovieDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AddMovie" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "AddMovieInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AddMovie" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "LibraryId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "LibraryId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Movie" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryId" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Title" } },
                      { kind: "Field", name: { kind: "Name", value: "Year" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TmdbId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ImdbId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Overview" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Monitored" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MediaFileId" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<AddMovieMutation, AddMovieMutationVariables>;
export const AddMovieCollectionDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AddMovieCollection" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "AddMovieCollectionInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AddMovieCollection" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "LibraryId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "LibraryId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CollectionId" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CollectionName" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ImportedCount" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ExistingCount" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "WantedUpdatedCount" },
                },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AddMovieCollectionMutation,
  AddMovieCollectionMutationVariables
>;
export const MovieChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "MovieChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Filter" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "SubscriptionFilterInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "MovieChanged" },
            name: { kind: "Name", value: "movieChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Action" },
                  name: { kind: "Name", value: "action" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Id" },
                  name: { kind: "Name", value: "id" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Movie" },
                  name: { kind: "Name", value: "movie" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryId" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  MovieChangedSubscription,
  MovieChangedSubscriptionVariables
>;
export const MovieDetailRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "MovieDetailRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Movie" },
            name: { kind: "Name", value: "movie" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "LibraryId" } },
                { kind: "Field", name: { kind: "Name", value: "Title" } },
                { kind: "Field", name: { kind: "Name", value: "SortTitle" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "OriginalTitle" },
                },
                { kind: "Field", name: { kind: "Name", value: "Year" } },
                { kind: "Field", name: { kind: "Name", value: "TmdbId" } },
                { kind: "Field", name: { kind: "Name", value: "ImdbId" } },
                { kind: "Field", name: { kind: "Name", value: "Overview" } },
                { kind: "Field", name: { kind: "Name", value: "Tagline" } },
                { kind: "Field", name: { kind: "Name", value: "Runtime" } },
                { kind: "Field", name: { kind: "Name", value: "Genres" } },
                { kind: "Field", name: { kind: "Name", value: "Director" } },
                { kind: "Field", name: { kind: "Name", value: "CastNames" } },
                { kind: "Field", name: { kind: "Name", value: "Monitored" } },
                { kind: "Field", name: { kind: "Name", value: "MediaFileId" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CollectionId" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CollectionName" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CollectionPosterUrl" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PosterUrl" },
                  name: { kind: "Name", value: "CollectionPosterUrl" },
                },
                { kind: "Field", name: { kind: "Name", value: "TmdbRating" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "TmdbVoteCount" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Certification" },
                },
                { kind: "Field", name: { kind: "Name", value: "ReleaseDate" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ProductionCountries" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "SpokenLanguages" },
                },
                { kind: "Field", name: { kind: "Name", value: "Wanted" } },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "MediaFile" },
                  name: { kind: "Name", value: "mediaFile" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Size" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Duration" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  MovieDetailRouteQuery,
  MovieDetailRouteQueryVariables
>;
export const MovieDetailSetWantedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "MovieDetailSetWanted" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Wanted" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "Boolean" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateMovie" },
            name: { kind: "Name", value: "updateMovie" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Wanted" },
                      value: {
                        kind: "Variable",
                        name: { kind: "Name", value: "Wanted" },
                      },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Movie" },
                  name: { kind: "Name", value: "movie" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Wanted" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  MovieDetailSetWantedMutation,
  MovieDetailSetWantedMutationVariables
>;
export const RefreshMovieRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "RefreshMovieRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "RefreshMovie" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Movie" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Title" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Overview" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Tagline" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TmdbRating" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TmdbVoteCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  RefreshMovieRouteMutation,
  RefreshMovieRouteMutationVariables
>;
export const DeleteMovieModalDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteMovieModal" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteMovie" },
            name: { kind: "Name", value: "deleteMovie" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteMovieModalMutation,
  DeleteMovieModalMutationVariables
>;
export const OrganizationNamingPatternsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "OrganizationNamingPatterns" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "NamingPatternOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "NamingPatterns" },
            name: { kind: "Name", value: "namingPatterns" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Pattern" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Description" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsDefault" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsSystem" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  OrganizationNamingPatternsQuery,
  OrganizationNamingPatternsQueryVariables
>;
export const OrganizationCreateNamingPatternDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "OrganizationCreateNamingPattern" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateNamingPatternInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateNamingPattern" },
            name: { kind: "Name", value: "createNamingPattern" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "NamingPattern" },
                  name: { kind: "Name", value: "namingPattern" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Pattern" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Description" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryType" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsDefault" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsSystem" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  OrganizationCreateNamingPatternMutation,
  OrganizationCreateNamingPatternMutationVariables
>;
export const OrganizationUpdateNamingPatternDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "OrganizationUpdateNamingPattern" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateNamingPatternInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateNamingPattern" },
            name: { kind: "Name", value: "updateNamingPattern" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "NamingPattern" },
                  name: { kind: "Name", value: "namingPattern" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Pattern" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Description" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryType" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsDefault" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsSystem" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  OrganizationUpdateNamingPatternMutation,
  OrganizationUpdateNamingPatternMutationVariables
>;
export const OrganizationDeleteNamingPatternDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "OrganizationDeleteNamingPattern" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteNamingPattern" },
            name: { kind: "Name", value: "deleteNamingPattern" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  OrganizationDeleteNamingPatternMutation,
  OrganizationDeleteNamingPatternMutationVariables
>;
export const NotificationsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "Notifications" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "NotificationWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "NotificationOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Notifications" },
            name: { kind: "Name", value: "notifications" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UserId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "NotificationType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Category" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Message" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TorrentId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaFileId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "PendingMatchId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ActionType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ActionData" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ReadAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ResolvedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Resolution" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CreatedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UpdatedAt" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<NotificationsQuery, NotificationsQueryVariables>;
export const NotificationChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "NotificationChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Filter" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "SubscriptionFilterInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "NotificationChanged" },
            name: { kind: "Name", value: "notificationChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Action" },
                  name: { kind: "Name", value: "action" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Id" },
                  name: { kind: "Name", value: "id" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Notification" },
                  name: { kind: "Name", value: "notification" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ReadAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ResolvedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Resolution" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  NotificationChangedSubscription,
  NotificationChangedSubscriptionVariables
>;
export const UpdateNotificationDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdateNotification" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateNotificationInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateNotification" },
            name: { kind: "Name", value: "updateNotification" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Notification" },
                  name: { kind: "Name", value: "notification" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ReadAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ResolvedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Resolution" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdateNotificationMutation,
  UpdateNotificationMutationVariables
>;
export const DeleteNotificationDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteNotification" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteNotification" },
            name: { kind: "Name", value: "deleteNotification" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteNotificationMutation,
  DeleteNotificationMutationVariables
>;
export const PlaybackSessionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "PlaybackSessions" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PlaybackSessionWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "PlaybackSessionOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "PlaybackSessions" },
            name: { kind: "Name", value: "playbackSessions" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UserId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaFileId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CurrentPosition" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Duration" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Volume" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsMuted" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsPlaying" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "StartedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastUpdatedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CompletedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CreatedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UpdatedAt" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Cursor" },
                        name: { kind: "Name", value: "cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  PlaybackSessionsQuery,
  PlaybackSessionsQueryVariables
>;
export const ShowPlaybackProgressByMediaDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ShowPlaybackProgressByMedia" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PlaybackProgressWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "PlaybackProgressOrderByInput" },
              },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "PlaybackProgresses" },
            name: { kind: "Name", value: "playbackProgresses" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaFileId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CurrentPosition" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Duration" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ProgressPercent" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsWatched" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UpdatedAt" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ShowPlaybackProgressByMediaQuery,
  ShowPlaybackProgressByMediaQueryVariables
>;
export const PlaybackProgressByMediaFileContextDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "PlaybackProgressByMediaFileContext" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PlaybackProgressWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "PlaybackProgressOrderByInput" },
              },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "PlaybackProgresses" },
            name: { kind: "Name", value: "playbackProgresses" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UserId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaFileId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CurrentPosition" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Duration" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ProgressPercent" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsWatched" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "WatchedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CreatedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UpdatedAt" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  PlaybackProgressByMediaFileContextQuery,
  PlaybackProgressByMediaFileContextQueryVariables
>;
export const CreatePlaybackSessionContextDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreatePlaybackSessionContext" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreatePlaybackSessionInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreatePlaybackSession" },
            name: { kind: "Name", value: "createPlaybackSession" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PlaybackSession" },
                  name: { kind: "Name", value: "playbackSession" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UserId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ContentType" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MediaFileId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "EpisodeId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MovieId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TrackId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "AudiobookId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TvShowId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "AlbumId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CurrentPosition" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Duration" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Volume" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsMuted" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsPlaying" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "StartedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LastUpdatedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CompletedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CreatedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UpdatedAt" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreatePlaybackSessionContextMutation,
  CreatePlaybackSessionContextMutationVariables
>;
export const UpdatePlaybackSessionContextDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdatePlaybackSessionContext" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdatePlaybackSessionInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdatePlaybackSession" },
            name: { kind: "Name", value: "updatePlaybackSession" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PlaybackSession" },
                  name: { kind: "Name", value: "playbackSession" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UserId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ContentType" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MediaFileId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "EpisodeId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MovieId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TrackId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "AudiobookId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TvShowId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "AlbumId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CurrentPosition" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Duration" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Volume" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsMuted" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsPlaying" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "StartedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LastUpdatedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CompletedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CreatedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UpdatedAt" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdatePlaybackSessionContextMutation,
  UpdatePlaybackSessionContextMutationVariables
>;
export const CreatePlaybackProgressContextDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreatePlaybackProgressContext" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreatePlaybackProgressInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreatePlaybackProgress" },
            name: { kind: "Name", value: "createPlaybackProgress" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PlaybackProgress" },
                  name: { kind: "Name", value: "playbackProgress" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UserId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MediaFileId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CurrentPosition" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Duration" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ProgressPercent" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsWatched" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "WatchedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CreatedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UpdatedAt" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreatePlaybackProgressContextMutation,
  CreatePlaybackProgressContextMutationVariables
>;
export const UpdatePlaybackProgressContextDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdatePlaybackProgressContext" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdatePlaybackProgressInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdatePlaybackProgress" },
            name: { kind: "Name", value: "updatePlaybackProgress" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PlaybackProgress" },
                  name: { kind: "Name", value: "playbackProgress" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UserId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "MediaFileId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CurrentPosition" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Duration" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ProgressPercent" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "IsWatched" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "WatchedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "CreatedAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UpdatedAt" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdatePlaybackProgressContextMutation,
  UpdatePlaybackProgressContextMutationVariables
>;
export const LibrarySearchShowsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibrarySearchShows" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Shows" },
            name: { kind: "Name", value: "shows" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Year" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "PosterUrl" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibrarySearchShowsQuery,
  LibrarySearchShowsQueryVariables
>;
export const LibrarySearchMoviesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "LibrarySearchMovies" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Movies" },
            name: { kind: "Name", value: "movies" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LibraryId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Year" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaFileId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Wanted" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LibrarySearchMoviesQuery,
  LibrarySearchMoviesQueryVariables
>;
export const SearchTvShowsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SearchTvShows" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Query" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "SearchTvShows" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Query" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Query" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Provider" } },
                { kind: "Field", name: { kind: "Name", value: "ProviderId" } },
                { kind: "Field", name: { kind: "Name", value: "Name" } },
                { kind: "Field", name: { kind: "Name", value: "Year" } },
                { kind: "Field", name: { kind: "Name", value: "Network" } },
                { kind: "Field", name: { kind: "Name", value: "Overview" } },
                { kind: "Field", name: { kind: "Name", value: "Status" } },
                { kind: "Field", name: { kind: "Name", value: "PosterUrl" } },
                { kind: "Field", name: { kind: "Name", value: "TvdbId" } },
                { kind: "Field", name: { kind: "Name", value: "ImdbId" } },
                { kind: "Field", name: { kind: "Name", value: "Score" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SearchTvShowsQuery, SearchTvShowsQueryVariables>;
export const AddTvShowDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AddTvShow" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "AddTvShowInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AddTvShow" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "LibraryId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "LibraryId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Show" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<AddTvShowMutation, AddTvShowMutationVariables>;
export const ShowChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "ShowChanged" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Filter" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "SubscriptionFilterInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "ShowChanged" },
            name: { kind: "Name", value: "showChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Action" },
                  name: { kind: "Name", value: "action" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Id" },
                  name: { kind: "Name", value: "id" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Show" },
                  name: { kind: "Name", value: "show" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryId" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ShowChangedSubscription,
  ShowChangedSubscriptionVariables
>;
export const ShowDetailRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ShowDetailRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Show" },
            name: { kind: "Name", value: "show" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "LibraryId" } },
                { kind: "Field", name: { kind: "Name", value: "Name" } },
                { kind: "Field", name: { kind: "Name", value: "SortName" } },
                { kind: "Field", name: { kind: "Name", value: "Year" } },
                { kind: "Field", name: { kind: "Name", value: "TvmazeId" } },
                { kind: "Field", name: { kind: "Name", value: "TmdbId" } },
                { kind: "Field", name: { kind: "Name", value: "TvdbId" } },
                { kind: "Field", name: { kind: "Name", value: "ImdbId" } },
                { kind: "Field", name: { kind: "Name", value: "Overview" } },
                { kind: "Field", name: { kind: "Name", value: "Network" } },
                { kind: "Field", name: { kind: "Name", value: "PosterUrl" } },
                { kind: "Field", name: { kind: "Name", value: "BackdropUrl" } },
                { kind: "Field", name: { kind: "Name", value: "Runtime" } },
                { kind: "Field", name: { kind: "Name", value: "Genres" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "AutoDownload" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "AutoDownloadMode" },
                },
                { kind: "Field", name: { kind: "Name", value: "Path" } },
                { kind: "Field", name: { kind: "Name", value: "CreatedAt" } },
                { kind: "Field", name: { kind: "Name", value: "UpdatedAt" } },
                { kind: "Field", name: { kind: "Name", value: "UserId" } },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Episodes" },
                  name: { kind: "Name", value: "episodes" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "page" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "limit" },
                            value: { kind: "IntValue", value: "2000" },
                          },
                        ],
                      },
                    },
                  ],
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Edges" },
                        name: { kind: "Name", value: "edges" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Node" },
                              name: { kind: "Name", value: "node" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Id" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "ShowId" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Season" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Episode" },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "AbsoluteNumber",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Title" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Overview" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "AirDate" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Runtime" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "TvmazeId" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "TmdbId" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "TvdbId" },
                                  },
                                  {
                                    kind: "Field",
                                    name: {
                                      kind: "Name",
                                      value: "MediaFileId",
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Wanted" },
                                  },
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "MediaFile" },
                                    name: { kind: "Name", value: "mediaFile" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Id" },
                                        },
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Size" },
                                        },
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "Duration",
                                          },
                                        },
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "Resolution",
                                          },
                                        },
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "VideoCodec",
                                          },
                                        },
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "AudioCodec",
                                          },
                                        },
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "AudioChannels",
                                          },
                                        },
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "IsHdr",
                                          },
                                        },
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "HdrType",
                                          },
                                        },
                                      ],
                                    },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "CreatedAt" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "UpdatedAt" },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ShowDetailRouteQuery,
  ShowDetailRouteQueryVariables
>;
export const RefreshShowRouteDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "RefreshShowRoute" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "RefreshShow" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Show" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Overview" },
                      },
                    ],
                  },
                },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  RefreshShowRouteMutation,
  RefreshShowRouteMutationVariables
>;
export const ShowDetailSetEpisodeWantedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "ShowDetailSetEpisodeWanted" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "ShowId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Wanted" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "Boolean" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateEpisodes" },
            name: { kind: "Name", value: "updateEpisodes" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "ShowId" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "eq" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "ShowId" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Wanted" },
                      value: {
                        kind: "Variable",
                        name: { kind: "Name", value: "Wanted" },
                      },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "success" } },
                { kind: "Field", name: { kind: "Name", value: "error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "affectedCount" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ShowDetailSetEpisodeWantedMutation,
  ShowDetailSetEpisodeWantedMutationVariables
>;
export const SourcesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "Sources" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "SourceWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "SourceOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Sources" },
            name: { kind: "Name", value: "sources" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourceType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DefinitionId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Enabled" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Priority" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MediaTypes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SiteUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SupportsSearch" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SupportsTvSearch" },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "SupportsMovieSearch",
                              },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "SupportsMusicSearch",
                              },
                            },
                            {
                              kind: "Field",
                              name: {
                                kind: "Name",
                                value: "SupportsBookSearch",
                              },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Settings" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastError" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ErrorCount" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastSuccessAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastErrorAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CreatedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UpdatedAt" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasPreviousPage" },
                        name: { kind: "Name", value: "hasPreviousPage" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SourcesQuery, SourcesQueryVariables>;
export const AvailableSourceDefinitionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "AvailableSourceDefinitions" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AvailableSourceDefinitions" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                { kind: "Field", name: { kind: "Name", value: "Name" } },
                { kind: "Field", name: { kind: "Name", value: "Description" } },
                { kind: "Field", name: { kind: "Name", value: "SourceType" } },
                { kind: "Field", name: { kind: "Name", value: "TrackerType" } },
                { kind: "Field", name: { kind: "Name", value: "Language" } },
                { kind: "Field", name: { kind: "Name", value: "SiteLink" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "RequiredCredentials" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AvailableSourceDefinitionsQuery,
  AvailableSourceDefinitionsQueryVariables
>;
export const SourceSettingDefinitionsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SourceSettingDefinitions" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "DefinitionId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "SourceSettingDefinitions" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "DefinitionId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "DefinitionId" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Key" } },
                { kind: "Field", name: { kind: "Name", value: "Label" } },
                { kind: "Field", name: { kind: "Name", value: "SettingType" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "DefaultValue" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Options" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Value" } },
                      { kind: "Field", name: { kind: "Name", value: "Label" } },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  SourceSettingDefinitionsQuery,
  SourceSettingDefinitionsQueryVariables
>;
export const SearchSourcesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SearchSources" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "SearchSourcesInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "SearchSources" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Sources" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "SourceId" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "SourceName" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Releases" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Title" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Guid" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Link" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MagnetUri" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "InfoHash" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Details" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "PublishDate" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Categories" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Size" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SizeFormatted" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Seeders" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Leechers" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Peers" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Grabs" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "IsFreeleech" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ImdbId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Poster" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Description" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourceId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourceName" },
                            },
                          ],
                        },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ElapsedMs" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "FromCache" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Error" } },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "TotalReleases" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "TotalElapsedMs" },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "SourcesSearched" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<SearchSourcesQuery, SearchSourcesQueryVariables>;
export const CreateSourceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreateSource" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateSourceInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateSource" },
            name: { kind: "Name", value: "createSource" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreateSourceMutation,
  CreateSourceMutationVariables
>;
export const UpdateSourceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdateSource" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateSourceInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateSource" },
            name: { kind: "Name", value: "updateSource" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdateSourceMutation,
  UpdateSourceMutationVariables
>;
export const DeleteSourceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "DeleteSource" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteSource" },
            name: { kind: "Name", value: "deleteSource" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DeleteSourceMutation,
  DeleteSourceMutationVariables
>;
export const TestSourceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "TestSource" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "TestSource" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "ReleasesFound" },
                },
                { kind: "Field", name: { kind: "Name", value: "ElapsedMs" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<TestSourceMutation, TestSourceMutationVariables>;
export const UpdateSourcePrioritiesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "UpdateSourcePriorities" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateSourcePrioritiesInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "UpdateSourcePriorities" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  UpdateSourcePrioritiesMutation,
  UpdateSourcePrioritiesMutationVariables
>;
export const ActiveDownloadCountDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "ActiveDownloadCount" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "ActiveDownloadCount" },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ActiveDownloadCountQuery,
  ActiveDownloadCountQueryVariables
>;
export const TorrentModalMediaFilesByPathsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "TorrentModalMediaFilesByPaths" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Paths" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "ListType",
              type: {
                kind: "NonNullType",
                type: {
                  kind: "NamedType",
                  name: { kind: "Name", value: "String" },
                },
              },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "MediaFiles" },
            name: { kind: "Name", value: "mediaFiles" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Path" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "inList" },
                            value: {
                              kind: "Variable",
                              name: { kind: "Name", value: "Paths" },
                            },
                          },
                        ],
                      },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "limit" },
                      value: { kind: "IntValue", value: "1000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Path" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Metadata" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  TorrentModalMediaFilesByPathsQuery,
  TorrentModalMediaFilesByPathsQueryVariables
>;
export const DownloadsTorrentsDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "DownloadsTorrents" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "TorrentWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Torrents" },
            name: { kind: "Name", value: "torrents" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "InfoHash" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "State" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Progress" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TotalBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DownloadedBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UploadedBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SavePath" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AddedAt" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "PageInfo" },
                  name: { kind: "Name", value: "pageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "TotalCount" },
                        name: { kind: "Name", value: "totalCount" },
                      },
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "HasNextPage" },
                        name: { kind: "Name", value: "hasNextPage" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  DownloadsTorrentsQuery,
  DownloadsTorrentsQueryVariables
>;
export const TorrentByInfoHashWithFilesDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "TorrentByInfoHashWithFiles" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "TorrentWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "Torrents" },
            name: { kind: "Name", value: "torrents" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "InfoHash" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "State" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Progress" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TotalBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "DownloadedBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UploadedBytes" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SavePath" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "AddedAt" },
                            },
                            {
                              kind: "Field",
                              alias: { kind: "Name", value: "Files" },
                              name: { kind: "Name", value: "files" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "limit" },
                                        value: {
                                          kind: "IntValue",
                                          value: "500",
                                        },
                                      },
                                    ],
                                  },
                                },
                              ],
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    alias: { kind: "Name", value: "Edges" },
                                    name: { kind: "Name", value: "edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          alias: {
                                            kind: "Name",
                                            value: "Node",
                                          },
                                          name: { kind: "Name", value: "node" },
                                          selectionSet: {
                                            kind: "SelectionSet",
                                            selections: [
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "FileIndex",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "FilePath",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "FileSize",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "DownloadedBytes",
                                                },
                                              },
                                              {
                                                kind: "Field",
                                                name: {
                                                  kind: "Name",
                                                  value: "Progress",
                                                },
                                              },
                                            ],
                                          },
                                        },
                                      ],
                                    },
                                  },
                                ],
                              },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  TorrentByInfoHashWithFilesQuery,
  TorrentByInfoHashWithFilesQueryVariables
>;
export const PendingFileMatchesBySourceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "PendingFileMatchesBySource" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Where" },
          },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PendingFileMatchWhereInput" },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "PendingFileMatches" },
            name: { kind: "Name", value: "pendingFileMatches" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourceType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourceId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourceFileIndex" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "SourcePath" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "FileSize" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "EpisodeId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MovieId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "TrackId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ChapterId" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MatchType" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "MatchConfidence" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ParsedResolution" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ParsedCodec" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ParsedSource" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ParsedAudio" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CopiedAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "CopyError" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  PendingFileMatchesBySourceQuery,
  PendingFileMatchesBySourceQueryVariables
>;
export const PauseTorrentByInfoHashDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "PauseTorrentByInfoHash" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "InfoHash" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "PauseTorrentByInfoHash" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "InfoHash" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "InfoHash" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  PauseTorrentByInfoHashMutation,
  PauseTorrentByInfoHashMutationVariables
>;
export const ResumeTorrentByInfoHashDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "ResumeTorrentByInfoHash" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "InfoHash" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "ResumeTorrentByInfoHash" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "InfoHash" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "InfoHash" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ResumeTorrentByInfoHashMutation,
  ResumeTorrentByInfoHashMutationVariables
>;
export const RemoveTorrentByInfoHashDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "RemoveTorrentByInfoHash" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "InfoHash" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "DeleteFiles" },
          },
          type: { kind: "NamedType", name: { kind: "Name", value: "Boolean" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "RemoveTorrentByInfoHash" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "InfoHash" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "InfoHash" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "DeleteFiles" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "DeleteFiles" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  RemoveTorrentByInfoHashMutation,
  RemoveTorrentByInfoHashMutationVariables
>;
export const ProcessSourceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "ProcessSource" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "SourceType" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "SourceId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "ProcessSource" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "SourceType" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "SourceType" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "SourceId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "SourceId" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "FilesProcessed" },
                },
                { kind: "Field", name: { kind: "Name", value: "FilesFailed" } },
                { kind: "Field", name: { kind: "Name", value: "Messages" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  ProcessSourceMutation,
  ProcessSourceMutationVariables
>;
export const RematchSourceDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "RematchSource" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "SourceType" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "SourceId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "LibraryId" },
          },
          type: { kind: "NamedType", name: { kind: "Name", value: "String" } },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "RematchSource" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "SourceType" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "SourceType" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "SourceId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "SourceId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "LibraryId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "LibraryId" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "MatchCount" } },
                { kind: "Field", name: { kind: "Name", value: "Error" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  RematchSourceMutation,
  RematchSourceMutationVariables
>;
export const LinkTorrentToLibraryDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "LinkTorrentToLibrary" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateTorrentInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateTorrent" },
            name: { kind: "Name", value: "updateTorrent" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Torrent" },
                  name: { kind: "Name", value: "torrent" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LibraryId" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  LinkTorrentToLibraryMutation,
  LinkTorrentToLibraryMutationVariables
>;
export const TorrentChangedDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "subscription",
      name: { kind: "Name", value: "TorrentChanged" },
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "TorrentChanged" },
            name: { kind: "Name", value: "torrentProgress" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Id" },
                  name: { kind: "Name", value: "id" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  TorrentChangedSubscription,
  TorrentChangedSubscriptionVariables
>;
export const CreateUnmatchedMediaFileFromTorrentDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "CreateUnmatchedMediaFileFromTorrent" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateMediaFileInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateMediaFile" },
            name: { kind: "Name", value: "createMediaFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "MediaFile" },
                  name: { kind: "Name", value: "mediaFile" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Path" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Metadata" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  CreateUnmatchedMediaFileFromTorrentMutation,
  CreateUnmatchedMediaFileFromTorrentMutationVariables
>;
export const AnalyzeMediaFileForTorrentDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "AnalyzeMediaFileForTorrent" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "MediaFileId" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Path" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            name: { kind: "Name", value: "AnalyzeMediaFile" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "MediaFileId" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "MediaFileId" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Path" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Path" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Success" } },
                { kind: "Field", name: { kind: "Name", value: "Queued" } },
                { kind: "Field", name: { kind: "Name", value: "Message" } },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  AnalyzeMediaFileForTorrentMutation,
  AnalyzeMediaFileForTorrentMutationVariables
>;
export const SettingsUsenetServersDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "query",
      name: { kind: "Name", value: "SettingsUsenetServers" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "OrderBy" },
          },
          type: {
            kind: "ListType",
            type: {
              kind: "NonNullType",
              type: {
                kind: "NamedType",
                name: { kind: "Name", value: "UsenetServerOrderByInput" },
              },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Page" } },
          type: {
            kind: "NamedType",
            name: { kind: "Name", value: "PageInput" },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UsenetServers" },
            name: { kind: "Name", value: "usenetServers" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "orderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Edges" },
                  name: { kind: "Name", value: "edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        alias: { kind: "Name", value: "Node" },
                        name: { kind: "Name", value: "node" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Id" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Name" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Host" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Port" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "UseSsl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Username" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Connections" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Priority" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Enabled" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "RetentionDays" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastSuccessAt" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "LastError" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "ErrorCount" },
                            },
                          ],
                        },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  SettingsUsenetServersQuery,
  SettingsUsenetServersQueryVariables
>;
export const SettingsUpdateUsenetServerDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "SettingsUpdateUsenetServer" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "UpdateUsenetServerInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "UpdateUsenetServer" },
            name: { kind: "Name", value: "updateUsenetServer" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "UsenetServer" },
                  name: { kind: "Name", value: "usenetServer" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Enabled" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Priority" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  SettingsUpdateUsenetServerMutation,
  SettingsUpdateUsenetServerMutationVariables
>;
export const SettingsCreateUsenetServerDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "SettingsCreateUsenetServer" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: {
            kind: "Variable",
            name: { kind: "Name", value: "Input" },
          },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "CreateUsenetServerInput" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "CreateUsenetServer" },
            name: { kind: "Name", value: "createUsenetServer" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "input" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Input" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "UsenetServer" },
                  name: { kind: "Name", value: "usenetServer" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Name" } },
                      { kind: "Field", name: { kind: "Name", value: "Host" } },
                      { kind: "Field", name: { kind: "Name", value: "Port" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "UseSsl" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Username" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Connections" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Priority" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Enabled" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "RetentionDays" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LastSuccessAt" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "LastError" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ErrorCount" },
                      },
                    ],
                  },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  SettingsCreateUsenetServerMutation,
  SettingsCreateUsenetServerMutationVariables
>;
export const SettingsDeleteUsenetServerDocument = {
  kind: "Document",
  definitions: [
    {
      kind: "OperationDefinition",
      operation: "mutation",
      name: { kind: "Name", value: "SettingsDeleteUsenetServer" },
      variableDefinitions: [
        {
          kind: "VariableDefinition",
          variable: { kind: "Variable", name: { kind: "Name", value: "Id" } },
          type: {
            kind: "NonNullType",
            type: {
              kind: "NamedType",
              name: { kind: "Name", value: "String" },
            },
          },
        },
      ],
      selectionSet: {
        kind: "SelectionSet",
        selections: [
          {
            kind: "Field",
            alias: { kind: "Name", value: "DeleteUsenetServer" },
            name: { kind: "Name", value: "deleteUsenetServer" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Success" },
                  name: { kind: "Name", value: "success" },
                },
                {
                  kind: "Field",
                  alias: { kind: "Name", value: "Error" },
                  name: { kind: "Name", value: "error" },
                },
              ],
            },
          },
        ],
      },
    },
  ],
} as unknown as DocumentNode<
  SettingsDeleteUsenetServerMutation,
  SettingsDeleteUsenetServerMutationVariables
>;
