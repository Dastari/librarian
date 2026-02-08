import type { TypedDocumentNode as DocumentNode } from "@apollo/client";
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = {
  [K in keyof T]: T[K];
};
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & {
  [SubKey in K]?: Maybe<T[SubKey]>;
};
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & {
  [SubKey in K]: Maybe<T[SubKey]>;
};
export type MakeEmpty<
  T extends { [key: string]: unknown },
  K extends keyof T,
> = { [_ in K]?: never };
export type Incremental<T> =
  | T
  | {
      [P in keyof T]?: P extends " $fragmentName" | "__typename" ? T[P] : never;
    };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string };
  String: { input: string; output: string };
  Boolean: { input: boolean; output: boolean };
  Int: { input: number; output: number };
  Float: { input: number; output: number };
};

export type AddAlbumInput = {
  LibraryId: Scalars["String"]["input"];
  MusicbrainzId: Scalars["String"]["input"];
};

export type AddAudiobookInput = {
  LibraryId: Scalars["String"]["input"];
  OpenlibraryId: Scalars["String"]["input"];
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
  Library?: Maybe<Library>;
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
};

/** Event for #struct_name changes (subscriptions) */
export type AlbumChangedEvent = {
  Action: ChangeAction;
  Album?: Maybe<Album>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type AlbumConnection = {
  /** The edges in this connection */
  Edges: Array<AlbumEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AlbumEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Album;
};

export type AlbumOperationResult = {
  Album?: Maybe<Album>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type AlbumOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  ReleaseDate?: InputMaybe<SortDirection>;
  SizeBytes?: InputMaybe<SortDirection>;
  SortName?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
  Year?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type AlbumResult = {
  Album?: Maybe<Album>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
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
  /** Logical AND of conditions */
  And?: InputMaybe<Array<AlbumWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<AlbumWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<AlbumWhereInput>>;
  ReleaseDate?: InputMaybe<DateFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  TotalDurationSecs?: InputMaybe<IntFilter>;
  TrackCount?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Year?: InputMaybe<IntFilter>;
};

export type AnalyzeMediaFileResult = {
  Message?: Maybe<Scalars["String"]["output"]>;
  Queued: Scalars["Boolean"]["output"];
  Success: Scalars["Boolean"]["output"];
};

/** AppLog Entity - application logs */
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
  Action: ChangeAction;
  AppLog?: Maybe<AppLog>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type AppLogConnection = {
  /** The edges in this connection */
  Edges: Array<AppLogEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AppLogEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: AppLog;
};

export type AppLogOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  Level?: InputMaybe<SortDirection>;
  Target?: InputMaybe<SortDirection>;
  Timestamp?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type AppLogResult = {
  AppLog?: Maybe<AppLog>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type AppLogWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<AppLogWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  Level?: InputMaybe<StringFilter>;
  Message?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<AppLogWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<AppLogWhereInput>>;
  SpanId?: InputMaybe<StringFilter>;
  SpanName?: InputMaybe<StringFilter>;
  Target?: InputMaybe<StringFilter>;
  Timestamp?: InputMaybe<DateFilter>;
};

/** AppSetting Entity - application settings */
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
  Action: ChangeAction;
  AppSetting?: Maybe<AppSetting>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type AppSettingConnection = {
  /** The edges in this connection */
  Edges: Array<AppSettingEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AppSettingEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: AppSetting;
};

export type AppSettingOrderByInput = {
  Category?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  Key?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type AppSettingResult = {
  AppSetting?: Maybe<AppSetting>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type AppSettingWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<AppSettingWhereInput>>;
  Category?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  Key?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<AppSettingWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<AppSettingWhereInput>>;
  UpdatedAt?: InputMaybe<DateFilter>;
};

export type Artist = {
  AlbumCount?: Maybe<Scalars["Int"]["output"]>;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Albums: AlbumConnection;
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
};

export type ArtistAlbumsArgs = {
  OrderBy?: InputMaybe<AlbumOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AlbumWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type ArtistChangedEvent = {
  Action: ChangeAction;
  Artist?: Maybe<Artist>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type ArtistConnection = {
  /** The edges in this connection */
  Edges: Array<ArtistEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ArtistEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Artist;
};

export type ArtistOrderByInput = {
  AlbumCount?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  SortName?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type ArtistResult = {
  Artist?: Maybe<Artist>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type ArtistWhereInput = {
  AlbumCount?: InputMaybe<IntFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<ArtistWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MusicbrainzId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<ArtistWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<ArtistWhereInput>>;
  TotalDurationSecs?: InputMaybe<IntFilter>;
  TrackCount?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
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
};

/** Event for #struct_name changes (subscriptions) */
export type ArtworkCacheChangedEvent = {
  Action: ChangeAction;
  ArtworkCache?: Maybe<ArtworkCache>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type ArtworkCacheConnection = {
  /** The edges in this connection */
  Edges: Array<ArtworkCacheEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ArtworkCacheEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: ArtworkCache;
};

export type ArtworkCacheOrderByInput = {
  ArtworkType?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  EntityType?: InputMaybe<SortDirection>;
  SizeBytes?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type ArtworkCacheResult = {
  ArtworkCache?: Maybe<ArtworkCache>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type ArtworkCacheWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<ArtworkCacheWhereInput>>;
  ArtworkType?: InputMaybe<StringFilter>;
  ContentHash?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EntityId?: InputMaybe<StringFilter>;
  EntityType?: InputMaybe<StringFilter>;
  Height?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MimeType?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<ArtworkCacheWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<ArtworkCacheWhereInput>>;
  SizeBytes?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Width?: InputMaybe<IntFilter>;
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
  Action: ChangeAction;
  AudioStream?: Maybe<AudioStream>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type AudioStreamConnection = {
  /** The edges in this connection */
  Edges: Array<AudioStreamEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AudioStreamEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: AudioStream;
};

export type AudioStreamOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  StreamIndex?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type AudioStreamResult = {
  AudioStream?: Maybe<AudioStream>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type AudioStreamWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<AudioStreamWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<AudioStreamWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<AudioStreamWhereInput>>;
  SampleRate?: InputMaybe<IntFilter>;
  StreamIndex?: InputMaybe<IntFilter>;
};

export type Audiobook = {
  Asin?: Maybe<Scalars["String"]["output"]>;
  AudibleId?: Maybe<Scalars["String"]["output"]>;
  AuthorName?: Maybe<Scalars["String"]["output"]>;
  AutoDownload: Scalars["Boolean"]["output"];
  AutoDownloadMode: AutoDownloadMode;
  ChapterCount?: Maybe<Scalars["Int"]["output"]>;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Chapters: ChapterConnection;
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
};

export type AudiobookChaptersArgs = {
  OrderBy?: InputMaybe<ChapterOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ChapterWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type AudiobookChangedEvent = {
  Action: ChangeAction;
  Audiobook?: Maybe<Audiobook>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type AudiobookConnection = {
  /** The edges in this connection */
  Edges: Array<AudiobookEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type AudiobookEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Audiobook;
};

export type AudiobookOperationResult = {
  Audiobook?: Maybe<Audiobook>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type AudiobookOrderByInput = {
  AuthorName?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  PublishedDate?: InputMaybe<SortDirection>;
  SizeBytes?: InputMaybe<SortDirection>;
  SortTitle?: InputMaybe<SortDirection>;
  Title?: InputMaybe<SortDirection>;
  TotalDurationSecs?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type AudiobookResult = {
  Audiobook?: Maybe<Audiobook>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
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
  /** Logical AND of conditions */
  And?: InputMaybe<Array<AudiobookWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<AudiobookWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<AudiobookWhereInput>>;
  PublishedDate?: InputMaybe<DateFilter>;
  Publisher?: InputMaybe<StringFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  Title?: InputMaybe<StringFilter>;
  TotalDurationSecs?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
};

export type AuthPayload = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  Tokens?: Maybe<AuthTokens>;
  User?: Maybe<AuthenticatedUser>;
};

/** Token pair returned after successful authentication */
export type AuthTokens = {
  /** Short-lived access token */
  AccessToken: Scalars["String"]["output"];
  /** Access token expiration in seconds */
  ExpiresIn: Scalars["Int"]["output"];
  /** Long-lived refresh token */
  RefreshToken: Scalars["String"]["output"];
  /** Token type (always "Bearer") */
  TokenType: Scalars["String"]["output"];
};

/** User info returned after successful authentication */
export type AuthenticatedUser = {
  AvatarUrl?: Maybe<Scalars["String"]["output"]>;
  DisplayName?: Maybe<Scalars["String"]["output"]>;
  Email?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  Role: Scalars["String"]["output"];
  Username: Scalars["String"]["output"];
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
/** Filter for boolean fields */
export type BoolFilter = {
  /** Equals */
  Eq?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** Is null */
  IsNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** Not equals (opposite of eq) */
  Ne?: InputMaybe<Scalars["Boolean"]["input"]>;
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
  Action: ChangeAction;
  CastDevice?: Maybe<CastDevice>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type CastDeviceConnection = {
  /** The edges in this connection */
  Edges: Array<CastDeviceEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type CastDeviceEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: CastDevice;
};

export type CastDeviceOperationResult = {
  device?: Maybe<LegacyCastDevice>;
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type CastDeviceOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  LastSeenAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type CastDeviceResult = {
  CastDevice?: Maybe<CastDevice>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type CastDeviceWhereInput = {
  Address?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<CastDeviceWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  DeviceType?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  IsFavorite?: InputMaybe<BoolFilter>;
  IsManual?: InputMaybe<BoolFilter>;
  LastSeenAt?: InputMaybe<DateFilter>;
  Model?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<CastDeviceWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<CastDeviceWhereInput>>;
  Port?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
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
  Action: ChangeAction;
  CastSession?: Maybe<CastSession>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type CastSessionConnection = {
  /** The edges in this connection */
  Edges: Array<CastSessionEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type CastSessionEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: CastSession;
};

export type CastSessionOperationResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  session?: Maybe<LegacyCastSession>;
  success: Scalars["Boolean"]["output"];
};

export type CastSessionOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  StartedAt?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type CastSessionResult = {
  CastSession?: Maybe<CastSession>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type CastSessionWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<CastSessionWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<CastSessionWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<CastSessionWhereInput>>;
  PlayerState?: InputMaybe<StringFilter>;
  StartedAt?: InputMaybe<DateFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Volume?: InputMaybe<IntFilter>;
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
  Action: ChangeAction;
  CastSetting?: Maybe<CastSetting>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type CastSettingConnection = {
  /** The edges in this connection */
  Edges: Array<CastSettingEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type CastSettingEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: CastSetting;
};

export type CastSettingOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type CastSettingResult = {
  CastSetting?: Maybe<CastSetting>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type CastSettingWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<CastSettingWhereInput>>;
  AutoDiscoveryEnabled?: InputMaybe<BoolFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DefaultVolume?: InputMaybe<IntFilter>;
  DiscoveryIntervalSeconds?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<CastSettingWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<CastSettingWhereInput>>;
  PreferredQuality?: InputMaybe<StringFilter>;
  TranscodeIncompatible?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
};

export type CastSettingsOperationResult = {
  error?: Maybe<Scalars["String"]["output"]>;
  settings?: Maybe<LegacyCastSettings>;
  success: Scalars["Boolean"]["output"];
};

/** Type of change for subscription events. */
export const ChangeAction = {
  Created: "Created",
  Deleted: "Deleted",
  Updated: "Updated",
} as const;

export type ChangeAction = (typeof ChangeAction)[keyof typeof ChangeAction];
export type Chapter = {
  AudiobookId: Scalars["String"]["output"];
  ChapterNumber: Scalars["Int"]["output"];
  CreatedAt: Scalars["String"]["output"];
  DurationSecs?: Maybe<Scalars["Int"]["output"]>;
  EndTimeSecs?: Maybe<Scalars["Float"]["output"]>;
  Id: Scalars["String"]["output"];
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  StartTimeSecs: Scalars["Float"]["output"];
  /**
   * Computed status based on playback, file availability, and download state
   *
   * Returns one of: PLAYING, PAUSED, AVAILABLE, DOWNLOADING, WANTED, MISSING
   */
  Status: ContentStatus;
  Title?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  Wanted: Scalars["Boolean"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type ChapterChangedEvent = {
  Action: ChangeAction;
  Chapter?: Maybe<Chapter>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type ChapterConnection = {
  /** The edges in this connection */
  Edges: Array<ChapterEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ChapterEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Chapter;
};

export type ChapterOrderByInput = {
  ChapterNumber?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  Title?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type ChapterResult = {
  Chapter?: Maybe<Chapter>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type ChapterWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<ChapterWhereInput>>;
  AudiobookId?: InputMaybe<StringFilter>;
  ChapterNumber?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DurationSecs?: InputMaybe<IntFilter>;
  EndTimeSecs?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<ChapterWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<ChapterWhereInput>>;
  StartTimeSecs?: InputMaybe<IntFilter>;
  Title?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Wanted?: InputMaybe<BoolFilter>;
};

export type ConfigureNetworkPathInput = {
  AttemptConnect?: InputMaybe<Scalars["Boolean"]["input"]>;
  MountPoint?: InputMaybe<Scalars["String"]["input"]>;
  Password?: InputMaybe<Scalars["String"]["input"]>;
  Path: Scalars["String"]["input"];
  Persist?: InputMaybe<Scalars["Boolean"]["input"]>;
  Username?: InputMaybe<Scalars["String"]["input"]>;
};

/** Content status for playable media items (episodes, movies, tracks, chapters) */
export const ContentStatus = {
  /** Content file is available (has media file) */
  AVAILABLE: "AVAILABLE",
  /** Content is currently downloading */
  DOWNLOADING: "DOWNLOADING",
  /** Content is missing (no file, not wanted) */
  MISSING: "MISSING",
  /** Content playback is paused */
  PAUSED: "PAUSED",
  /** Content is currently being played */
  PLAYING: "PLAYING",
  /** Content is wanted but not yet downloaded */
  WANTED: "WANTED",
} as const;

export type ContentStatus = (typeof ContentStatus)[keyof typeof ContentStatus];
export type CopyFilesInput = {
  Destination: Scalars["String"]["input"];
  Overwrite?: InputMaybe<Scalars["Boolean"]["input"]>;
  Sources: Array<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateAlbumInput = {
  AlbumType?: InputMaybe<Scalars["String"]["input"]>;
  ArtistId: Scalars["String"]["input"];
  AutoDownload: Scalars["Boolean"]["input"];
  AutoDownloadMode: AutoDownloadMode;
  Country?: InputMaybe<Scalars["String"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  DiscCount?: InputMaybe<Scalars["Int"]["input"]>;
  Genres: Array<Scalars["String"]["input"]>;
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

/** Input for creating a new #struct_name */
export type CreateAppLogInput = {
  Fields?: InputMaybe<Scalars["String"]["input"]>;
  Level: Scalars["String"]["input"];
  Message: Scalars["String"]["input"];
  SpanId?: InputMaybe<Scalars["String"]["input"]>;
  SpanName?: InputMaybe<Scalars["String"]["input"]>;
  Target: Scalars["String"]["input"];
  Timestamp: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateAppSettingInput = {
  Category: Scalars["String"]["input"];
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Key: Scalars["String"]["input"];
  Value: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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
  Narrators: Array<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PublishedDate?: InputMaybe<Scalars["String"]["input"]>;
  Publisher?: InputMaybe<Scalars["String"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
export type CreateCastSettingInput = {
  AutoDiscoveryEnabled: Scalars["Boolean"]["input"];
  DefaultVolume: Scalars["Float"]["input"];
  DiscoveryIntervalSeconds: Scalars["Int"]["input"];
  PreferredQuality?: InputMaybe<Scalars["String"]["input"]>;
  TranscodeIncompatible: Scalars["Boolean"]["input"];
};

/** Input for creating a new #struct_name */
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

export type CreateDirectoryInput = {
  Path: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
export type CreateInviteTokenInput = {
  AccessLevel: Scalars["String"]["input"];
  ApplyRestrictions: Scalars["Boolean"]["input"];
  CreatedBy: Scalars["String"]["input"];
  ExpiresAt?: InputMaybe<Scalars["String"]["input"]>;
  IsActive: Scalars["Boolean"]["input"];
  LibraryIds: Array<Scalars["String"]["input"]>;
  MaxUses?: InputMaybe<Scalars["Int"]["input"]>;
  RestrictionsTemplate?: InputMaybe<Scalars["String"]["input"]>;
  Role: Scalars["String"]["input"];
  Token: Scalars["String"]["input"];
  UseCount: Scalars["Int"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
export type CreateMediaChapterInput = {
  ChapterIndex: Scalars["Int"]["input"];
  EndSecs: Scalars["Float"]["input"];
  MediaFileId: Scalars["String"]["input"];
  StartSecs: Scalars["Float"]["input"];
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateMediaFileInput = {
  AddedAt: Scalars["String"]["input"];
  AudioChannels?: InputMaybe<Scalars["String"]["input"]>;
  AudioCodec?: InputMaybe<Scalars["String"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
  Container?: InputMaybe<Scalars["String"]["input"]>;
  ContentType?: InputMaybe<Scalars["String"]["input"]>;
  Duration?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  HdrType?: InputMaybe<Scalars["String"]["input"]>;
  Height?: InputMaybe<Scalars["Int"]["input"]>;
  IsHdr: Scalars["Boolean"]["input"];
  LibraryId: Scalars["String"]["input"];
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

/** Input for creating a new #struct_name */
export type CreateMovieInput = {
  CastNames: Array<Scalars["String"]["input"]>;
  Certification?: InputMaybe<Scalars["String"]["input"]>;
  CollectionId?: InputMaybe<Scalars["Int"]["input"]>;
  CollectionName?: InputMaybe<Scalars["String"]["input"]>;
  CollectionPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Director?: InputMaybe<Scalars["String"]["input"]>;
  DownloadStatus?: InputMaybe<Scalars["String"]["input"]>;
  Genres: Array<Scalars["String"]["input"]>;
  HasFile: Scalars["Boolean"]["input"];
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Monitored: Scalars["Boolean"]["input"];
  OriginalTitle?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  ProductionCountries: Array<Scalars["String"]["input"]>;
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  SpokenLanguages: Array<Scalars["String"]["input"]>;
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

/** Input for creating a new #struct_name */
export type CreateNamingPatternInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  IsDefault: Scalars["Boolean"]["input"];
  IsSystem: Scalars["Boolean"]["input"];
  LibraryType: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Pattern: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
export type CreatePlaybackProgressInput = {
  CurrentPosition: Scalars["Float"]["input"];
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  IsWatched: Scalars["Boolean"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  ProgressPercent: Scalars["Float"]["input"];
  UserId: Scalars["String"]["input"];
  WatchedAt?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
export type CreateRefreshTokenInput = {
  DeviceInfo?: InputMaybe<Scalars["String"]["input"]>;
  ExpiresAt: Scalars["String"]["input"];
  IpAddress?: InputMaybe<Scalars["String"]["input"]>;
  LastUsedAt?: InputMaybe<Scalars["String"]["input"]>;
  TokenHash: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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
  ShowGenres: Array<Scalars["String"]["input"]>;
  ShowName: Scalars["String"]["input"];
  ShowNetwork?: InputMaybe<Scalars["String"]["input"]>;
  ShowPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Summary?: InputMaybe<Scalars["String"]["input"]>;
  TvmazeEpisodeId: Scalars["Int"]["input"];
  TvmazeShowId: Scalars["Int"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateScheduleSyncStateInput = {
  CountryCode: Scalars["String"]["input"];
  LastSyncDays: Scalars["Int"]["input"];
  LastSyncedAt: Scalars["String"]["input"];
  SyncError?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateShowInput = {
  AutoDownload: Scalars["Boolean"]["input"];
  AutoDownloadMode: AutoDownloadMode;
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  ContentRating?: InputMaybe<Scalars["String"]["input"]>;
  Genres: Array<Scalars["String"]["input"]>;
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

/** Input for creating a new #struct_name */
export type CreateSourceInput = {
  Credentials: Scalars["String"]["input"];
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
};

/** Input for creating a new #struct_name */
export type CreateSourcePriorityRuleInput = {
  Enabled: Scalars["Boolean"]["input"];
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  PriorityOrder: Array<Scalars["String"]["input"]>;
  SearchAllSources: Scalars["Boolean"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
export type CreateTorrentInput = {
  AddedAt: Scalars["String"]["input"];
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  DownloadPath?: InputMaybe<Scalars["String"]["input"]>;
  DownloadedBytes: Scalars["Int"]["input"];
  ExcludedFiles: Array<Scalars["Int"]["input"]>;
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

/** Input for creating a new #struct_name */
export type CreateTorznabCategoryInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  ParentId?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
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

/** Input for creating a new #struct_name */
export type CreateUserInput = {
  AvatarUrl?: InputMaybe<Scalars["String"]["input"]>;
  DisplayName?: InputMaybe<Scalars["String"]["input"]>;
  Email?: InputMaybe<Scalars["String"]["input"]>;
  IsActive: Scalars["Boolean"]["input"];
  LastLoginAt?: InputMaybe<Scalars["String"]["input"]>;
  Role: Scalars["String"]["input"];
  Username: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
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

/** Filter for date/timestamp fields */
export type DateFilter = {
  /** Between two dates (inclusive) */
  Between?: InputMaybe<DateRange>;
  /** Equals */
  Eq?: InputMaybe<Scalars["String"]["input"]>;
  /** After (greater than) */
  Gt?: InputMaybe<Scalars["String"]["input"]>;
  /** After or on (greater than or equal) */
  Gte?: InputMaybe<Scalars["String"]["input"]>;
  /** Greater than or equal to relative date */
  GteRelative?: InputMaybe<RelativeDate>;
  /** In the future (after today) */
  InFuture?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** In the past (before today) */
  InPast?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** Is null */
  IsNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** Is today */
  IsToday?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** Before (less than) */
  Lt?: InputMaybe<Scalars["String"]["input"]>;
  /** Before or on (less than or equal) */
  Lte?: InputMaybe<Scalars["String"]["input"]>;
  /** Less than or equal to relative date */
  LteRelative?: InputMaybe<RelativeDate>;
  /** Not equals */
  Ne?: InputMaybe<Scalars["String"]["input"]>;
  /** Within the last N days (inclusive of today) */
  RecentDays?: InputMaybe<Scalars["Int"]["input"]>;
  /** Within the next N days (inclusive of today) */
  WithinDays?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Date range for between queries */
export type DateRange = {
  /** End of range (inclusive) */
  End?: InputMaybe<Scalars["String"]["input"]>;
  /** Start of range (inclusive) */
  Start?: InputMaybe<Scalars["String"]["input"]>;
};

/** Result of bulk delete by Where filter */
export type DeleteAlbumsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAppLogsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAppSettingsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteArtistsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteArtworkCachesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAudioStreamsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteAudiobooksResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteCastDevicesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteCastSessionsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteCastSettingsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteChaptersResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteEpisodesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type DeleteFilesInput = {
  Paths: Array<Scalars["String"]["input"]>;
  Recursive?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Result of bulk delete by Where filter */
export type DeleteInviteTokensResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteLibrariesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMediaChaptersResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMediaFilesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteMoviesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteNamingPatternsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteNotificationsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeletePendingFileMatchesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeletePlaybackProgressesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeletePlaybackSessionsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteRefreshTokensResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteRssFeedItemsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteRssFeedsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteScheduleCachesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteScheduleSyncStatesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteShowsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteSourcePriorityRulesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteSourcesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteSubtitlesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTorrentFilesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTorrentsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTorznabCategoriesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteTracksResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteUsenetDownloadsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteUsenetServersResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteUsersResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteVideoStreamsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

export type Episode = {
  AbsoluteNumber?: Maybe<Scalars["Int"]["output"]>;
  AirDate?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Episode: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  MediaFile?: Maybe<MediaFile>;
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  Season: Scalars["Int"]["output"];
  ShowId: Scalars["String"]["output"];
  /**
   * Computed status based on playback, file availability, and download state
   *
   * Returns one of: PLAYING, PAUSED, AVAILABLE, DOWNLOADING, WANTED, MISSING
   */
  Status: ContentStatus;
  Title?: Maybe<Scalars["String"]["output"]>;
  TmdbId?: Maybe<Scalars["Int"]["output"]>;
  TvdbId?: Maybe<Scalars["Int"]["output"]>;
  TvmazeId?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  Wanted: Scalars["Boolean"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type EpisodeChangedEvent = {
  Action: ChangeAction;
  Episode?: Maybe<Episode>;
  Id: Scalars["String"]["output"];
};

/** Connection containing edges and page info */
export type EpisodeConnection = {
  /** The edges in this connection */
  Edges: Array<EpisodeEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type EpisodeEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Episode;
};

export type EpisodeOrderByInput = {
  AirDate?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  Episode?: InputMaybe<SortDirection>;
  Season?: InputMaybe<SortDirection>;
  Title?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type EpisodeResult = {
  Episode?: Maybe<Episode>;
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type EpisodeWhereInput = {
  AbsoluteNumber?: InputMaybe<IntFilter>;
  AirDate?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<EpisodeWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  Episode?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<EpisodeWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<EpisodeWhereInput>>;
  Runtime?: InputMaybe<IntFilter>;
  Season?: InputMaybe<IntFilter>;
  ShowId?: InputMaybe<StringFilter>;
  Title?: InputMaybe<StringFilter>;
  TmdbId?: InputMaybe<IntFilter>;
  TvdbId?: InputMaybe<IntFilter>;
  TvmazeId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Wanted?: InputMaybe<BoolFilter>;
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

/** Filter for integer fields */
export type IntFilter = {
  /** Equals */
  Eq?: InputMaybe<Scalars["Int"]["input"]>;
  /** Greater than */
  Gt?: InputMaybe<Scalars["Int"]["input"]>;
  /** Greater than or equal */
  Gte?: InputMaybe<Scalars["Int"]["input"]>;
  /** In list */
  In?: InputMaybe<Array<Scalars["Int"]["input"]>>;
  /** Is null */
  IsNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** Less than */
  Lt?: InputMaybe<Scalars["Int"]["input"]>;
  /** Less than or equal */
  Lte?: InputMaybe<Scalars["Int"]["input"]>;
  /** Not equals */
  Ne?: InputMaybe<Scalars["Int"]["input"]>;
  /** Not in list */
  NotIn?: InputMaybe<Array<Scalars["Int"]["input"]>>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  InviteToken?: Maybe<InviteToken>;
};

/** Connection containing edges and page info */
export type InviteTokenConnection = {
  /** The edges in this connection */
  Edges: Array<InviteTokenEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type InviteTokenEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: InviteToken;
};

export type InviteTokenOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type InviteTokenResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  InviteToken?: Maybe<InviteToken>;
  Success: Scalars["Boolean"]["output"];
};

export type InviteTokenWhereInput = {
  AccessLevel?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<InviteTokenWhereInput>>;
  ApplyRestrictions?: InputMaybe<BoolFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  CreatedBy?: InputMaybe<StringFilter>;
  ExpiresAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IsActive?: InputMaybe<BoolFilter>;
  MaxUses?: InputMaybe<IntFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<InviteTokenWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<InviteTokenWhereInput>>;
  Role?: InputMaybe<StringFilter>;
  Token?: InputMaybe<StringFilter>;
  UseCount?: InputMaybe<IntFilter>;
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
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Albums: AlbumConnection;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Audiobooks: AudiobookConnection;
  AutoOrganize: Scalars["Boolean"]["output"];
  AutoScan: Scalars["Boolean"]["output"];
  Color?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Icon?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  LastScannedAt?: Maybe<Scalars["String"]["output"]>;
  LibraryType: Scalars["String"]["output"];
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  MediaFiles: MediaFileConnection;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Movies: MovieConnection;
  Name: Scalars["String"]["output"];
  NamingPattern: Scalars["String"]["output"];
  Path: Scalars["String"]["output"];
  ScanIntervalMinutes: Scalars["Int"]["output"];
  Scanning: Scalars["Boolean"]["output"];
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Shows: ShowConnection;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  WatchForChanges: Scalars["Boolean"]["output"];
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryAlbumsArgs = {
  OrderBy?: InputMaybe<AlbumOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AlbumWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryAudiobooksArgs = {
  OrderBy?: InputMaybe<AudiobookOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AudiobookWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryMediaFilesArgs = {
  OrderBy?: InputMaybe<MediaFileOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<MediaFileWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryMoviesArgs = {
  OrderBy?: InputMaybe<MovieOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<MovieWhereInput>;
};

/**
 * Library entity representing a media library.
 *
 * Relations (Shows, Movies, Artists, etc.) are automatically generated
 * by the GraphQLRelations macro and use DataLoader for N+1 prevention.
 */
export type LibraryShowsArgs = {
  OrderBy?: InputMaybe<ShowOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ShowWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type LibraryChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Library?: Maybe<Library>;
};

/** Connection containing edges and page info */
export type LibraryConnection = {
  /** The edges in this connection */
  Edges: Array<LibraryEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type LibraryEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Library;
};

export type LibraryOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  Id?: InputMaybe<SortDirection>;
  LastScannedAt?: InputMaybe<SortDirection>;
  LibraryType?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
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
  Error?: Maybe<Scalars["String"]["output"]>;
  Library?: Maybe<Library>;
  Success: Scalars["Boolean"]["output"];
};

export type LibraryWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<LibraryWhereInput>>;
  AutoOrganize?: InputMaybe<BoolFilter>;
  AutoScan?: InputMaybe<BoolFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastScannedAt?: InputMaybe<DateFilter>;
  LibraryType?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  NamingPattern?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<LibraryWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<LibraryWhereInput>>;
  Path?: InputMaybe<StringFilter>;
  ScanIntervalMinutes?: InputMaybe<IntFilter>;
  Scanning?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  WatchForChanges?: InputMaybe<BoolFilter>;
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

export type MatchMediaFileInput = {
  ChapterId?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId: Scalars["String"]["input"];
  Methods?: InputMaybe<Array<MatchMethod>>;
  MovieId?: InputMaybe<Scalars["String"]["input"]>;
  TrackId?: InputMaybe<Scalars["String"]["input"]>;
};

export type MatchMediaFileResult = {
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  MediaChapter?: Maybe<MediaChapter>;
};

/** Connection containing edges and page info */
export type MediaChapterConnection = {
  /** The edges in this connection */
  Edges: Array<MediaChapterEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MediaChapterEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: MediaChapter;
};

export type MediaChapterOrderByInput = {
  ChapterIndex?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type MediaChapterResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  MediaChapter?: Maybe<MediaChapter>;
  Success: Scalars["Boolean"]["output"];
};

export type MediaChapterWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<MediaChapterWhereInput>>;
  ChapterIndex?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EndSecs?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<MediaChapterWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<MediaChapterWhereInput>>;
  StartSecs?: InputMaybe<IntFilter>;
  Title?: InputMaybe<StringFilter>;
};

export type MediaFile = {
  AddedAt: Scalars["String"]["output"];
  AudioChannels?: Maybe<Scalars["String"]["output"]>;
  AudioCodec?: Maybe<Scalars["String"]["output"]>;
  Bitrate?: Maybe<Scalars["Int"]["output"]>;
  Container?: Maybe<Scalars["String"]["output"]>;
  ContentType?: Maybe<Scalars["String"]["output"]>;
  Duration?: Maybe<Scalars["Int"]["output"]>;
  EpisodeId?: Maybe<Scalars["String"]["output"]>;
  HdrType?: Maybe<Scalars["String"]["output"]>;
  Height?: Maybe<Scalars["Int"]["output"]>;
  Id: Scalars["String"]["output"];
  IsHdr: Scalars["Boolean"]["output"];
  LibraryId: Scalars["String"]["output"];
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  MediaFile?: Maybe<MediaFile>;
};

/** Connection containing edges and page info */
export type MediaFileConnection = {
  /** The edges in this connection */
  Edges: Array<MediaFileEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MediaFileEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: MediaFile;
};

export type MediaFileOrderByInput = {
  AddedAt?: InputMaybe<SortDirection>;
  Duration?: InputMaybe<SortDirection>;
  Path?: InputMaybe<SortDirection>;
  Resolution?: InputMaybe<SortDirection>;
  Size?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type MediaFileResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  MediaFile?: Maybe<MediaFile>;
  Success: Scalars["Boolean"]["output"];
};

export type MediaFileWhereInput = {
  AddedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<MediaFileWhereInput>>;
  AudioChannels?: InputMaybe<StringFilter>;
  AudioCodec?: InputMaybe<StringFilter>;
  Bitrate?: InputMaybe<IntFilter>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<MediaFileWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<MediaFileWhereInput>>;
  Path?: InputMaybe<StringFilter>;
  Resolution?: InputMaybe<StringFilter>;
  Size?: InputMaybe<IntFilter>;
  TrackId?: InputMaybe<StringFilter>;
  VideoCodec?: InputMaybe<StringFilter>;
  Width?: InputMaybe<IntFilter>;
};

export type MoveFilesInput = {
  Destination: Scalars["String"]["input"];
  Overwrite?: InputMaybe<Scalars["Boolean"]["input"]>;
  Sources: Array<Scalars["String"]["input"]>;
};

export type Movie = {
  /** Get backdrop URL, preferring cached version if available */
  BackdropUrl?: Maybe<Scalars["String"]["output"]>;
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
  MediaFile?: Maybe<MediaFile>;
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  Monitored: Scalars["Boolean"]["output"];
  OriginalTitle?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  /** Get poster URL, preferring cached version if available */
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  ProductionCountries: Array<Scalars["String"]["output"]>;
  ReleaseDate?: Maybe<Scalars["String"]["output"]>;
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  SortTitle?: Maybe<Scalars["String"]["output"]>;
  SpokenLanguages: Array<Scalars["String"]["output"]>;
  /**
   * Computed status based on playback, file availability, and download state
   *
   * Returns one of: PLAYING, PAUSED, AVAILABLE, DOWNLOADING, WANTED, MISSING
   */
  Status: ContentStatus;
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
};

/** Event for #struct_name changes (subscriptions) */
export type MovieChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Movie?: Maybe<Movie>;
};

/** Connection containing edges and page info */
export type MovieConnection = {
  /** The edges in this connection */
  Edges: Array<MovieEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type MovieEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Movie;
};

/** Result of movie operations */
export type MovieOperationResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Movie?: Maybe<Movie>;
  Success: Scalars["Boolean"]["output"];
};

export type MovieOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  ReleaseDate?: InputMaybe<SortDirection>;
  Runtime?: InputMaybe<SortDirection>;
  SortTitle?: InputMaybe<SortDirection>;
  Title?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
  Year?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type MovieResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Movie?: Maybe<Movie>;
  Success: Scalars["Boolean"]["output"];
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
  /** Logical AND of conditions */
  And?: InputMaybe<Array<MovieWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<MovieWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<MovieWhereInput>>;
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
};

export type MutationRoot = {
  /** Add an album to a library by fetching metadata from MusicBrainz. */
  AddAlbum: AlbumOperationResult;
  /** Add an audiobook to a library by fetching metadata from OpenLibrary. */
  AddAudiobook: AudiobookOperationResult;
  AddCastDevice: CastDeviceOperationResult;
  /** Add a movie to a library by fetching metadata from TMDB */
  AddMovie: MovieOperationResult;
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
  /** Create a new #struct_name_str */
  CreateAlbum: AlbumResult;
  /** Create a new #struct_name_str */
  CreateAppLog: AppLogResult;
  /** Create a new #struct_name_str */
  CreateAppSetting: AppSettingResult;
  /** Create a new #struct_name_str */
  CreateArtist: ArtistResult;
  /** Create a new #struct_name_str */
  CreateArtworkCache: ArtworkCacheResult;
  /** Create a new #struct_name_str */
  CreateAudioStream: AudioStreamResult;
  /** Create a new #struct_name_str */
  CreateAudiobook: AudiobookResult;
  /** Create a new #struct_name_str */
  CreateCastDevice: CastDeviceResult;
  /** Create a new #struct_name_str */
  CreateCastSession: CastSessionResult;
  /** Create a new #struct_name_str */
  CreateCastSetting: CastSettingResult;
  /** Create a new #struct_name_str */
  CreateChapter: ChapterResult;
  CreateDirectory: FileOperationPayload;
  /** Create a new #struct_name_str */
  CreateEpisode: EpisodeResult;
  /** Create a new #struct_name_str */
  CreateInviteToken: InviteTokenResult;
  /** Create a new #struct_name_str */
  CreateLibrary: LibraryResult;
  /** Create a new #struct_name_str */
  CreateMediaChapter: MediaChapterResult;
  /** Create a new #struct_name_str */
  CreateMediaFile: MediaFileResult;
  /** Create a new #struct_name_str */
  CreateMovie: MovieResult;
  /** Create a new #struct_name_str */
  CreateNamingPattern: NamingPatternResult;
  /** Create a new #struct_name_str */
  CreateNotification: NotificationResult;
  /** Create a new #struct_name_str */
  CreatePendingFileMatch: PendingFileMatchResult;
  /** Create a new #struct_name_str */
  CreatePlaybackProgress: PlaybackProgressResult;
  /** Create a new #struct_name_str */
  CreatePlaybackSession: PlaybackSessionResult;
  /** Create a new #struct_name_str */
  CreateRefreshToken: RefreshTokenResult;
  /** Create a new #struct_name_str */
  CreateRssFeed: RssFeedResult;
  /** Create a new #struct_name_str */
  CreateRssFeedItem: RssFeedItemResult;
  /** Create a new #struct_name_str */
  CreateScheduleCache: ScheduleCacheResult;
  /** Create a new #struct_name_str */
  CreateScheduleSyncState: ScheduleSyncStateResult;
  /** Create a new #struct_name_str */
  CreateShow: ShowResult;
  /** Create a new #struct_name_str */
  CreateSource: SourceResult;
  /** Create a new #struct_name_str */
  CreateSourcePriorityRule: SourcePriorityRuleResult;
  /** Create a new #struct_name_str */
  CreateSubtitle: SubtitleResult;
  /** Create a new #struct_name_str */
  CreateTorrent: TorrentResult;
  /** Create a new #struct_name_str */
  CreateTorrentFile: TorrentFileResult;
  /** Create a new #struct_name_str */
  CreateTorznabCategory: TorznabCategoryResult;
  /** Create a new #struct_name_str */
  CreateTrack: TrackResult;
  /** Create a new #struct_name_str */
  CreateUsenetDownload: UsenetDownloadResult;
  /** Create a new #struct_name_str */
  CreateUsenetServer: UsenetServerResult;
  /** Create a new #struct_name_str */
  CreateUser: UserResult;
  /** Create a new #struct_name_str */
  CreateVideoStream: VideoStreamResult;
  /** Delete a #struct_name_str */
  DeleteAlbum: AlbumResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteAlbums: DeleteAlbumsResult;
  /** Delete a #struct_name_str */
  DeleteAppLog: AppLogResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteAppLogs: DeleteAppLogsResult;
  /** Delete a #struct_name_str */
  DeleteAppSetting: AppSettingResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteAppSettings: DeleteAppSettingsResult;
  /** Delete a #struct_name_str */
  DeleteArtist: ArtistResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteArtists: DeleteArtistsResult;
  /** Delete a #struct_name_str */
  DeleteArtworkCache: ArtworkCacheResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteArtworkCaches: DeleteArtworkCachesResult;
  /** Delete a #struct_name_str */
  DeleteAudioStream: AudioStreamResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteAudioStreams: DeleteAudioStreamsResult;
  /** Delete a #struct_name_str */
  DeleteAudiobook: AudiobookResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteAudiobooks: DeleteAudiobooksResult;
  /** Delete a #struct_name_str */
  DeleteCastDevice: CastDeviceResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteCastDevices: DeleteCastDevicesResult;
  /** Delete a #struct_name_str */
  DeleteCastSession: CastSessionResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteCastSessions: DeleteCastSessionsResult;
  /** Delete a #struct_name_str */
  DeleteCastSetting: CastSettingResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteCastSettings: DeleteCastSettingsResult;
  /** Delete a #struct_name_str */
  DeleteChapter: ChapterResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteChapters: DeleteChaptersResult;
  /** Delete a #struct_name_str */
  DeleteEpisode: EpisodeResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteEpisodes: DeleteEpisodesResult;
  DeleteFiles: FileOperationPayload;
  /** Delete a #struct_name_str */
  DeleteInviteToken: InviteTokenResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteInviteTokens: DeleteInviteTokensResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteLibraries: DeleteLibrariesResult;
  /** Delete a #struct_name_str */
  DeleteLibrary: LibraryResult;
  /** Delete a #struct_name_str */
  DeleteMediaChapter: MediaChapterResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteMediaChapters: DeleteMediaChaptersResult;
  /** Delete a #struct_name_str */
  DeleteMediaFile: MediaFileResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteMediaFiles: DeleteMediaFilesResult;
  /** Delete a #struct_name_str */
  DeleteMovie: MovieResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteMovies: DeleteMoviesResult;
  /** Delete a #struct_name_str */
  DeleteNamingPattern: NamingPatternResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteNamingPatterns: DeleteNamingPatternsResult;
  /** Delete a #struct_name_str */
  DeleteNotification: NotificationResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteNotifications: DeleteNotificationsResult;
  /** Delete a #struct_name_str */
  DeletePendingFileMatch: PendingFileMatchResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeletePendingFileMatches: DeletePendingFileMatchesResult;
  /** Delete a #struct_name_str */
  DeletePlaybackProgress: PlaybackProgressResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeletePlaybackProgresses: DeletePlaybackProgressesResult;
  /** Delete a #struct_name_str */
  DeletePlaybackSession: PlaybackSessionResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeletePlaybackSessions: DeletePlaybackSessionsResult;
  /** Delete a #struct_name_str */
  DeleteRefreshToken: RefreshTokenResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteRefreshTokens: DeleteRefreshTokensResult;
  /** Delete a #struct_name_str */
  DeleteRssFeed: RssFeedResult;
  /** Delete a #struct_name_str */
  DeleteRssFeedItem: RssFeedItemResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteRssFeedItems: DeleteRssFeedItemsResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteRssFeeds: DeleteRssFeedsResult;
  /** Delete a #struct_name_str */
  DeleteScheduleCache: ScheduleCacheResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteScheduleCaches: DeleteScheduleCachesResult;
  /** Delete a #struct_name_str */
  DeleteScheduleSyncState: ScheduleSyncStateResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteScheduleSyncStates: DeleteScheduleSyncStatesResult;
  /** Delete a #struct_name_str */
  DeleteShow: ShowResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteShows: DeleteShowsResult;
  /** Delete a #struct_name_str */
  DeleteSource: SourceResult;
  /** Delete a #struct_name_str */
  DeleteSourcePriorityRule: SourcePriorityRuleResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteSourcePriorityRules: DeleteSourcePriorityRulesResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteSources: DeleteSourcesResult;
  /** Delete a #struct_name_str */
  DeleteSubtitle: SubtitleResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteSubtitles: DeleteSubtitlesResult;
  /** Delete a #struct_name_str */
  DeleteTorrent: TorrentResult;
  /** Delete a #struct_name_str */
  DeleteTorrentFile: TorrentFileResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteTorrentFiles: DeleteTorrentFilesResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteTorrents: DeleteTorrentsResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteTorznabCategories: DeleteTorznabCategoriesResult;
  /** Delete a #struct_name_str */
  DeleteTorznabCategory: TorznabCategoryResult;
  /** Delete a #struct_name_str */
  DeleteTrack: TrackResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteTracks: DeleteTracksResult;
  /** Delete a #struct_name_str */
  DeleteUsenetDownload: UsenetDownloadResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteUsenetDownloads: DeleteUsenetDownloadsResult;
  /** Delete a #struct_name_str */
  DeleteUsenetServer: UsenetServerResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteUsenetServers: DeleteUsenetServersResult;
  /** Delete a #struct_name_str */
  DeleteUser: UserResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteUsers: DeleteUsersResult;
  /** Delete a #struct_name_str */
  DeleteVideoStream: VideoStreamResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteVideoStreams: DeleteVideoStreamsResult;
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
  /** Update an existing #struct_name_str */
  UpdateAlbum: AlbumResult;
  /** Update an existing #struct_name_str */
  UpdateAppLog: AppLogResult;
  /** Update an existing #struct_name_str */
  UpdateAppSetting: AppSettingResult;
  /** Update an existing #struct_name_str */
  UpdateArtist: ArtistResult;
  /** Update an existing #struct_name_str */
  UpdateArtworkCache: ArtworkCacheResult;
  /** Update an existing #struct_name_str */
  UpdateAudioStream: AudioStreamResult;
  /** Update an existing #struct_name_str */
  UpdateAudiobook: AudiobookResult;
  /** Update an existing #struct_name_str */
  UpdateCastDevice: CastDeviceResult;
  /** Update an existing #struct_name_str */
  UpdateCastSession: CastSessionResult;
  /** Update an existing #struct_name_str */
  UpdateCastSetting: CastSettingResult;
  UpdateCastSettings: CastSettingsOperationResult;
  /** Update an existing #struct_name_str */
  UpdateChapter: ChapterResult;
  /** Update an existing #struct_name_str */
  UpdateEpisode: EpisodeResult;
  /** Update an existing #struct_name_str */
  UpdateInviteToken: InviteTokenResult;
  /** Update an existing #struct_name_str */
  UpdateLibrary: LibraryResult;
  /** Update an existing #struct_name_str */
  UpdateMediaChapter: MediaChapterResult;
  /** Update an existing #struct_name_str */
  UpdateMediaFile: MediaFileResult;
  /** Update an existing #struct_name_str */
  UpdateMovie: MovieResult;
  /** Update an existing #struct_name_str */
  UpdateNamingPattern: NamingPatternResult;
  /** Update an existing #struct_name_str */
  UpdateNotification: NotificationResult;
  /** Update an existing #struct_name_str */
  UpdatePendingFileMatch: PendingFileMatchResult;
  /** Update an existing #struct_name_str */
  UpdatePlaybackProgress: PlaybackProgressResult;
  /** Update an existing #struct_name_str */
  UpdatePlaybackSession: PlaybackSessionResult;
  /** Update an existing #struct_name_str */
  UpdateRefreshToken: RefreshTokenResult;
  /** Update an existing #struct_name_str */
  UpdateRssFeed: RssFeedResult;
  /** Update an existing #struct_name_str */
  UpdateRssFeedItem: RssFeedItemResult;
  /** Update an existing #struct_name_str */
  UpdateScheduleCache: ScheduleCacheResult;
  /** Update an existing #struct_name_str */
  UpdateScheduleSyncState: ScheduleSyncStateResult;
  /** Update an existing #struct_name_str */
  UpdateShow: ShowResult;
  /** Update an existing #struct_name_str */
  UpdateSource: SourceResult;
  /** Update source priorities (reorder) */
  UpdateSourcePriorities: SourceMutationResult;
  /** Update an existing #struct_name_str */
  UpdateSourcePriorityRule: SourcePriorityRuleResult;
  /** Update an existing #struct_name_str */
  UpdateSubtitle: SubtitleResult;
  /** Update an existing #struct_name_str */
  UpdateTorrent: TorrentResult;
  /** Update an existing #struct_name_str */
  UpdateTorrentFile: TorrentFileResult;
  /** Update an existing #struct_name_str */
  UpdateTorznabCategory: TorznabCategoryResult;
  /** Update an existing #struct_name_str */
  UpdateTrack: TrackResult;
  /** Update an existing #struct_name_str */
  UpdateUsenetDownload: UsenetDownloadResult;
  /** Update an existing #struct_name_str */
  UpdateUsenetServer: UsenetServerResult;
  /** Update an existing #struct_name_str */
  UpdateUser: UserResult;
  /** Update an existing #struct_name_str */
  UpdateVideoStream: VideoStreamResult;
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

export type MutationRootCreateAlbumArgs = {
  Input: CreateAlbumInput;
};

export type MutationRootCreateAppLogArgs = {
  Input: CreateAppLogInput;
};

export type MutationRootCreateAppSettingArgs = {
  Input: CreateAppSettingInput;
};

export type MutationRootCreateArtistArgs = {
  Input: CreateArtistInput;
};

export type MutationRootCreateArtworkCacheArgs = {
  Input: CreateArtworkCacheInput;
};

export type MutationRootCreateAudioStreamArgs = {
  Input: CreateAudioStreamInput;
};

export type MutationRootCreateAudiobookArgs = {
  Input: CreateAudiobookInput;
};

export type MutationRootCreateCastDeviceArgs = {
  Input: CreateCastDeviceInput;
};

export type MutationRootCreateCastSessionArgs = {
  Input: CreateCastSessionInput;
};

export type MutationRootCreateCastSettingArgs = {
  Input: CreateCastSettingInput;
};

export type MutationRootCreateChapterArgs = {
  Input: CreateChapterInput;
};

export type MutationRootCreateDirectoryArgs = {
  Input: CreateDirectoryInput;
};

export type MutationRootCreateEpisodeArgs = {
  Input: CreateEpisodeInput;
};

export type MutationRootCreateInviteTokenArgs = {
  Input: CreateInviteTokenInput;
};

export type MutationRootCreateLibraryArgs = {
  Input: CreateLibraryInput;
};

export type MutationRootCreateMediaChapterArgs = {
  Input: CreateMediaChapterInput;
};

export type MutationRootCreateMediaFileArgs = {
  Input: CreateMediaFileInput;
};

export type MutationRootCreateMovieArgs = {
  Input: CreateMovieInput;
};

export type MutationRootCreateNamingPatternArgs = {
  Input: CreateNamingPatternInput;
};

export type MutationRootCreateNotificationArgs = {
  Input: CreateNotificationInput;
};

export type MutationRootCreatePendingFileMatchArgs = {
  Input: CreatePendingFileMatchInput;
};

export type MutationRootCreatePlaybackProgressArgs = {
  Input: CreatePlaybackProgressInput;
};

export type MutationRootCreatePlaybackSessionArgs = {
  Input: CreatePlaybackSessionInput;
};

export type MutationRootCreateRefreshTokenArgs = {
  Input: CreateRefreshTokenInput;
};

export type MutationRootCreateRssFeedArgs = {
  Input: CreateRssFeedInput;
};

export type MutationRootCreateRssFeedItemArgs = {
  Input: CreateRssFeedItemInput;
};

export type MutationRootCreateScheduleCacheArgs = {
  Input: CreateScheduleCacheInput;
};

export type MutationRootCreateScheduleSyncStateArgs = {
  Input: CreateScheduleSyncStateInput;
};

export type MutationRootCreateShowArgs = {
  Input: CreateShowInput;
};

export type MutationRootCreateSourceArgs = {
  Input: CreateSourceInput;
};

export type MutationRootCreateSourcePriorityRuleArgs = {
  Input: CreateSourcePriorityRuleInput;
};

export type MutationRootCreateSubtitleArgs = {
  Input: CreateSubtitleInput;
};

export type MutationRootCreateTorrentArgs = {
  Input: CreateTorrentInput;
};

export type MutationRootCreateTorrentFileArgs = {
  Input: CreateTorrentFileInput;
};

export type MutationRootCreateTorznabCategoryArgs = {
  Input: CreateTorznabCategoryInput;
};

export type MutationRootCreateTrackArgs = {
  Input: CreateTrackInput;
};

export type MutationRootCreateUsenetDownloadArgs = {
  Input: CreateUsenetDownloadInput;
};

export type MutationRootCreateUsenetServerArgs = {
  Input: CreateUsenetServerInput;
};

export type MutationRootCreateUserArgs = {
  Input: CreateUserInput;
};

export type MutationRootCreateVideoStreamArgs = {
  Input: CreateVideoStreamInput;
};

export type MutationRootDeleteAlbumArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteAlbumsArgs = {
  Where?: InputMaybe<AlbumWhereInput>;
};

export type MutationRootDeleteAppLogArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteAppLogsArgs = {
  Where?: InputMaybe<AppLogWhereInput>;
};

export type MutationRootDeleteAppSettingArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteAppSettingsArgs = {
  Where?: InputMaybe<AppSettingWhereInput>;
};

export type MutationRootDeleteArtistArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteArtistsArgs = {
  Where?: InputMaybe<ArtistWhereInput>;
};

export type MutationRootDeleteArtworkCacheArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteArtworkCachesArgs = {
  Where?: InputMaybe<ArtworkCacheWhereInput>;
};

export type MutationRootDeleteAudioStreamArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteAudioStreamsArgs = {
  Where?: InputMaybe<AudioStreamWhereInput>;
};

export type MutationRootDeleteAudiobookArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteAudiobooksArgs = {
  Where?: InputMaybe<AudiobookWhereInput>;
};

export type MutationRootDeleteCastDeviceArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteCastDevicesArgs = {
  Where?: InputMaybe<CastDeviceWhereInput>;
};

export type MutationRootDeleteCastSessionArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteCastSessionsArgs = {
  Where?: InputMaybe<CastSessionWhereInput>;
};

export type MutationRootDeleteCastSettingArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteCastSettingsArgs = {
  Where?: InputMaybe<CastSettingWhereInput>;
};

export type MutationRootDeleteChapterArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteChaptersArgs = {
  Where?: InputMaybe<ChapterWhereInput>;
};

export type MutationRootDeleteEpisodeArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteEpisodesArgs = {
  Where?: InputMaybe<EpisodeWhereInput>;
};

export type MutationRootDeleteFilesArgs = {
  Input: DeleteFilesInput;
};

export type MutationRootDeleteInviteTokenArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteInviteTokensArgs = {
  Where?: InputMaybe<InviteTokenWhereInput>;
};

export type MutationRootDeleteLibrariesArgs = {
  Where?: InputMaybe<LibraryWhereInput>;
};

export type MutationRootDeleteLibraryArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteMediaChapterArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteMediaChaptersArgs = {
  Where?: InputMaybe<MediaChapterWhereInput>;
};

export type MutationRootDeleteMediaFileArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteMediaFilesArgs = {
  Where?: InputMaybe<MediaFileWhereInput>;
};

export type MutationRootDeleteMovieArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteMoviesArgs = {
  Where?: InputMaybe<MovieWhereInput>;
};

export type MutationRootDeleteNamingPatternArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteNamingPatternsArgs = {
  Where?: InputMaybe<NamingPatternWhereInput>;
};

export type MutationRootDeleteNotificationArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteNotificationsArgs = {
  Where?: InputMaybe<NotificationWhereInput>;
};

export type MutationRootDeletePendingFileMatchArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeletePendingFileMatchesArgs = {
  Where?: InputMaybe<PendingFileMatchWhereInput>;
};

export type MutationRootDeletePlaybackProgressArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeletePlaybackProgressesArgs = {
  Where?: InputMaybe<PlaybackProgressWhereInput>;
};

export type MutationRootDeletePlaybackSessionArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeletePlaybackSessionsArgs = {
  Where?: InputMaybe<PlaybackSessionWhereInput>;
};

export type MutationRootDeleteRefreshTokenArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteRefreshTokensArgs = {
  Where?: InputMaybe<RefreshTokenWhereInput>;
};

export type MutationRootDeleteRssFeedArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteRssFeedItemArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteRssFeedItemsArgs = {
  Where?: InputMaybe<RssFeedItemWhereInput>;
};

export type MutationRootDeleteRssFeedsArgs = {
  Where?: InputMaybe<RssFeedWhereInput>;
};

export type MutationRootDeleteScheduleCacheArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteScheduleCachesArgs = {
  Where?: InputMaybe<ScheduleCacheWhereInput>;
};

export type MutationRootDeleteScheduleSyncStateArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteScheduleSyncStatesArgs = {
  Where?: InputMaybe<ScheduleSyncStateWhereInput>;
};

export type MutationRootDeleteShowArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteShowsArgs = {
  Where?: InputMaybe<ShowWhereInput>;
};

export type MutationRootDeleteSourceArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteSourcePriorityRuleArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteSourcePriorityRulesArgs = {
  Where?: InputMaybe<SourcePriorityRuleWhereInput>;
};

export type MutationRootDeleteSourcesArgs = {
  Where?: InputMaybe<SourceWhereInput>;
};

export type MutationRootDeleteSubtitleArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteSubtitlesArgs = {
  Where?: InputMaybe<SubtitleWhereInput>;
};

export type MutationRootDeleteTorrentArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteTorrentFileArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteTorrentFilesArgs = {
  Where?: InputMaybe<TorrentFileWhereInput>;
};

export type MutationRootDeleteTorrentsArgs = {
  Where?: InputMaybe<TorrentWhereInput>;
};

export type MutationRootDeleteTorznabCategoriesArgs = {
  Where?: InputMaybe<TorznabCategoryWhereInput>;
};

export type MutationRootDeleteTorznabCategoryArgs = {
  Id: Scalars["Int"]["input"];
};

export type MutationRootDeleteTrackArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteTracksArgs = {
  Where?: InputMaybe<TrackWhereInput>;
};

export type MutationRootDeleteUsenetDownloadArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteUsenetDownloadsArgs = {
  Where?: InputMaybe<UsenetDownloadWhereInput>;
};

export type MutationRootDeleteUsenetServerArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteUsenetServersArgs = {
  Where?: InputMaybe<UsenetServerWhereInput>;
};

export type MutationRootDeleteUserArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteUsersArgs = {
  Where?: InputMaybe<UserWhereInput>;
};

export type MutationRootDeleteVideoStreamArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteVideoStreamsArgs = {
  Where?: InputMaybe<VideoStreamWhereInput>;
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

export type MutationRootUpdateAlbumArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateAlbumInput;
};

export type MutationRootUpdateAppLogArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateAppLogInput;
};

export type MutationRootUpdateAppSettingArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateAppSettingInput;
};

export type MutationRootUpdateArtistArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateArtistInput;
};

export type MutationRootUpdateArtworkCacheArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateArtworkCacheInput;
};

export type MutationRootUpdateAudioStreamArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateAudioStreamInput;
};

export type MutationRootUpdateAudiobookArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateAudiobookInput;
};

export type MutationRootUpdateCastDeviceArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateCastDeviceInput;
};

export type MutationRootUpdateCastSessionArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateCastSessionInput;
};

export type MutationRootUpdateCastSettingArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateCastSettingInput;
};

export type MutationRootUpdateCastSettingsArgs = {
  input: LegacyUpdateCastSettingsInput;
};

export type MutationRootUpdateChapterArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateChapterInput;
};

export type MutationRootUpdateEpisodeArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateEpisodeInput;
};

export type MutationRootUpdateInviteTokenArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateInviteTokenInput;
};

export type MutationRootUpdateLibraryArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateLibraryInput;
};

export type MutationRootUpdateMediaChapterArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateMediaChapterInput;
};

export type MutationRootUpdateMediaFileArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateMediaFileInput;
};

export type MutationRootUpdateMovieArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateMovieInput;
};

export type MutationRootUpdateNamingPatternArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateNamingPatternInput;
};

export type MutationRootUpdateNotificationArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateNotificationInput;
};

export type MutationRootUpdatePendingFileMatchArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdatePendingFileMatchInput;
};

export type MutationRootUpdatePlaybackProgressArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdatePlaybackProgressInput;
};

export type MutationRootUpdatePlaybackSessionArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdatePlaybackSessionInput;
};

export type MutationRootUpdateRefreshTokenArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateRefreshTokenInput;
};

export type MutationRootUpdateRssFeedArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateRssFeedInput;
};

export type MutationRootUpdateRssFeedItemArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateRssFeedItemInput;
};

export type MutationRootUpdateScheduleCacheArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateScheduleCacheInput;
};

export type MutationRootUpdateScheduleSyncStateArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateScheduleSyncStateInput;
};

export type MutationRootUpdateShowArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateShowInput;
};

export type MutationRootUpdateSourceArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateSourceInput;
};

export type MutationRootUpdateSourcePrioritiesArgs = {
  Input: UpdateSourcePrioritiesInput;
};

export type MutationRootUpdateSourcePriorityRuleArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateSourcePriorityRuleInput;
};

export type MutationRootUpdateSubtitleArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateSubtitleInput;
};

export type MutationRootUpdateTorrentArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateTorrentInput;
};

export type MutationRootUpdateTorrentFileArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateTorrentFileInput;
};

export type MutationRootUpdateTorznabCategoryArgs = {
  Id: Scalars["Int"]["input"];
  Input: UpdateTorznabCategoryInput;
};

export type MutationRootUpdateTrackArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateTrackInput;
};

export type MutationRootUpdateUsenetDownloadArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateUsenetDownloadInput;
};

export type MutationRootUpdateUsenetServerArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateUsenetServerInput;
};

export type MutationRootUpdateUserArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateUserInput;
};

export type MutationRootUpdateVideoStreamArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateVideoStreamInput;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  NamingPattern?: Maybe<NamingPattern>;
};

/** Connection containing edges and page info */
export type NamingPatternConnection = {
  /** The edges in this connection */
  Edges: Array<NamingPatternEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type NamingPatternEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: NamingPattern;
};

export type NamingPatternOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  LibraryType?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type NamingPatternResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  NamingPattern?: Maybe<NamingPattern>;
  Success: Scalars["Boolean"]["output"];
};

export type NamingPatternWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<NamingPatternWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IsDefault?: InputMaybe<BoolFilter>;
  IsSystem?: InputMaybe<BoolFilter>;
  LibraryType?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<NamingPatternWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<NamingPatternWhereInput>>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Notification?: Maybe<Notification>;
};

/** Connection containing edges and page info */
export type NotificationConnection = {
  /** The edges in this connection */
  Edges: Array<NotificationEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type NotificationEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Notification;
};

export type NotificationOrderByInput = {
  Category?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  NotificationType?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type NotificationResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Notification?: Maybe<Notification>;
  Success: Scalars["Boolean"]["output"];
};

export type NotificationWhereInput = {
  ActionType?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<NotificationWhereInput>>;
  Category?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<NotificationWhereInput>;
  NotificationType?: InputMaybe<StringFilter>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<NotificationWhereInput>>;
  PendingMatchId?: InputMaybe<StringFilter>;
  ReadAt?: InputMaybe<DateFilter>;
  Resolution?: InputMaybe<StringFilter>;
  ResolvedAt?: InputMaybe<DateFilter>;
  Title?: InputMaybe<StringFilter>;
  TorrentId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
};

export type OrganizeMediaFileInput = {
  MediaFileId: Scalars["String"]["input"];
};

export type OrganizeMediaFileResult = {
  NewPath?: Maybe<Scalars["String"]["output"]>;
  OldPath?: Maybe<Scalars["String"]["output"]>;
  Reason?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

/** Information about pagination in a connection */
export type PageInfo = {
  /** Cursor of the last item in this page */
  EndCursor?: Maybe<Scalars["String"]["output"]>;
  /** When paginating forwards, are there more items? */
  HasNextPage: Scalars["Boolean"]["output"];
  /** When paginating backwards, are there more items? */
  HasPreviousPage: Scalars["Boolean"]["output"];
  /** Cursor of the first item in this page */
  StartCursor?: Maybe<Scalars["String"]["output"]>;
  /** Total count of items (if available) */
  TotalCount?: Maybe<Scalars["Int"]["output"]>;
};

/** Pagination input for offset-based pagination. */
export type PageInput = {
  /** Maximum number of items to return (default: 25, max: 100) */
  Limit?: InputMaybe<Scalars["Int"]["input"]>;
  /** Number of items to skip */
  Offset?: InputMaybe<Scalars["Int"]["input"]>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  PendingFileMatch?: Maybe<PendingFileMatch>;
};

/** Connection containing edges and page info */
export type PendingFileMatchConnection = {
  /** The edges in this connection */
  Edges: Array<PendingFileMatchEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type PendingFileMatchEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: PendingFileMatch;
};

export type PendingFileMatchOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  FileSize?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type PendingFileMatchResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  PendingFileMatch?: Maybe<PendingFileMatch>;
  Success: Scalars["Boolean"]["output"];
};

export type PendingFileMatchWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<PendingFileMatchWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<PendingFileMatchWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<PendingFileMatchWhereInput>>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  PlaybackProgress?: Maybe<PlaybackProgress>;
};

/** Connection containing edges and page info */
export type PlaybackProgressConnection = {
  /** The edges in this connection */
  Edges: Array<PlaybackProgressEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type PlaybackProgressEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: PlaybackProgress;
};

export type PlaybackProgressOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type PlaybackProgressResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  PlaybackProgress?: Maybe<PlaybackProgress>;
  Success: Scalars["Boolean"]["output"];
};

export type PlaybackProgressWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<PlaybackProgressWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  CurrentPosition?: InputMaybe<IntFilter>;
  Duration?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IsWatched?: InputMaybe<BoolFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<PlaybackProgressWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<PlaybackProgressWhereInput>>;
  ProgressPercent?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  WatchedAt?: InputMaybe<DateFilter>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  PlaybackSession?: Maybe<PlaybackSession>;
};

/** Connection containing edges and page info */
export type PlaybackSessionConnection = {
  /** The edges in this connection */
  Edges: Array<PlaybackSessionEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type PlaybackSessionEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: PlaybackSession;
};

export type PlaybackSessionOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  LastUpdatedAt?: InputMaybe<SortDirection>;
  StartedAt?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type PlaybackSessionResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  PlaybackSession?: Maybe<PlaybackSession>;
  Success: Scalars["Boolean"]["output"];
};

export type PlaybackSessionWhereInput = {
  AlbumId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<PlaybackSessionWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<PlaybackSessionWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<PlaybackSessionWhereInput>>;
  StartedAt?: InputMaybe<DateFilter>;
  TrackId?: InputMaybe<StringFilter>;
  TvShowId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Volume?: InputMaybe<IntFilter>;
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
  /** Get a single #struct_name_str by ID */
  Album?: Maybe<Album>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Albums: AlbumConnection;
  /** Get a single #struct_name_str by ID */
  AppLog?: Maybe<AppLog>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  AppLogs: AppLogConnection;
  /** Get a single #struct_name_str by ID */
  AppSetting?: Maybe<AppSetting>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  AppSettings: AppSettingConnection;
  /** Get a single #struct_name_str by ID */
  Artist?: Maybe<Artist>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Artists: ArtistConnection;
  /** Get a single #struct_name_str by ID */
  ArtworkCache?: Maybe<ArtworkCache>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  ArtworkCaches: ArtworkCacheConnection;
  /** Get a single #struct_name_str by ID */
  AudioStream?: Maybe<AudioStream>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  AudioStreams: AudioStreamConnection;
  /** Get a single #struct_name_str by ID */
  Audiobook?: Maybe<Audiobook>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Audiobooks: AudiobookConnection;
  /** Get available source definitions (e.g., IPTorrents, Newznab, etc.) */
  AvailableSourceDefinitions: Array<SourceDefinitionInfo>;
  /** Browse a directory on the server. Requires authentication. */
  BrowseDirectory: BrowseDirectoryResult;
  /** Get a single #struct_name_str by ID */
  CastDevice?: Maybe<CastDevice>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  CastDevices: CastDeviceConnection;
  /** Get a single #struct_name_str by ID */
  CastSession?: Maybe<CastSession>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  CastSessions: CastSessionConnection;
  /** Get a single #struct_name_str by ID */
  CastSetting?: Maybe<CastSetting>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  CastSettings: CastSettingConnection;
  /** Get a single #struct_name_str by ID */
  Chapter?: Maybe<Chapter>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Chapters: ChapterConnection;
  /** Get a single #struct_name_str by ID */
  Episode?: Maybe<Episode>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Episodes: EpisodeConnection;
  FilesystemRuntimeInfo: FilesystemRuntimeInfo;
  /** Get a single #struct_name_str by ID */
  InviteToken?: Maybe<InviteToken>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  InviteTokens: InviteTokenConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Libraries: LibraryConnection;
  /** Get a single #struct_name_str by ID */
  Library?: Maybe<Library>;
  LibraryPathAvailability: Array<LibraryPathAvailability>;
  /** Get a single live torrent by numeric id */
  LiveTorrent?: Maybe<LiveTorrent>;
  /** Get all torrents with live state from the torrent client */
  LiveTorrents: Array<LiveTorrent>;
  /** Current authenticated user (requires valid JWT). Returns null if not authenticated. */
  Me?: Maybe<MeUser>;
  /** Get a single #struct_name_str by ID */
  MediaChapter?: Maybe<MediaChapter>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  MediaChapters: MediaChapterConnection;
  /** Get a single #struct_name_str by ID */
  MediaFile?: Maybe<MediaFile>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  MediaFiles: MediaFileConnection;
  /** Get a single #struct_name_str by ID */
  Movie?: Maybe<Movie>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Movies: MovieConnection;
  /** Get a single #struct_name_str by ID */
  NamingPattern?: Maybe<NamingPattern>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  NamingPatterns: NamingPatternConnection;
  /** True if no admin user exists yet (first-time setup required). */
  NeedsSetup: Scalars["Boolean"]["output"];
  /** Get a single #struct_name_str by ID */
  Notification?: Maybe<Notification>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Notifications: NotificationConnection;
  /** Get a single #struct_name_str by ID */
  PendingFileMatch?: Maybe<PendingFileMatch>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  PendingFileMatches: PendingFileMatchConnection;
  /** Get a single #struct_name_str by ID */
  PlaybackProgress?: Maybe<PlaybackProgress>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  PlaybackProgresses: PlaybackProgressConnection;
  /** Get a single #struct_name_str by ID */
  PlaybackSession?: Maybe<PlaybackSession>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  PlaybackSessions: PlaybackSessionConnection;
  /** Get a single #struct_name_str by ID */
  RefreshToken?: Maybe<RefreshToken>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  RefreshTokens: RefreshTokenConnection;
  /** Get a single #struct_name_str by ID */
  RssFeed?: Maybe<RssFeed>;
  /** Get a single #struct_name_str by ID */
  RssFeedItem?: Maybe<RssFeedItem>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  RssFeedItems: RssFeedItemConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  RssFeeds: RssFeedConnection;
  /** Get a single #struct_name_str by ID */
  ScheduleCache?: Maybe<ScheduleCache>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  ScheduleCaches: ScheduleCacheConnection;
  /** Get a single #struct_name_str by ID */
  ScheduleSyncState?: Maybe<ScheduleSyncState>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  ScheduleSyncStates: ScheduleSyncStateConnection;
  /** Search albums on MusicBrainz. */
  SearchAlbums: Array<AlbumSearchResult>;
  /** Search audiobooks on OpenLibrary. */
  SearchAudiobooks: Array<AudiobookSearchResult>;
  /** Search for movies on TMDB */
  SearchMovies: Array<MovieSearchResult>;
  /** Search across all enabled sources */
  SearchSources: SourceSearchResultSet;
  /** Search for TV shows on TVMaze */
  SearchTvShows: Array<TvShowSearchResult>;
  /** Get a single #struct_name_str by ID */
  Show?: Maybe<Show>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Shows: ShowConnection;
  /** Get a single #struct_name_str by ID */
  Source?: Maybe<Source>;
  /** Get a single #struct_name_str by ID */
  SourcePriorityRule?: Maybe<SourcePriorityRule>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  SourcePriorityRules: SourcePriorityRuleConnection;
  /** Get setting definitions for a source definition */
  SourceSettingDefinitions: Array<SourceSettingDefinition>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Sources: SourceConnection;
  /** Get a single #struct_name_str by ID */
  Subtitle?: Maybe<Subtitle>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Subtitles: SubtitleConnection;
  /** Get a single #struct_name_str by ID */
  Torrent?: Maybe<Torrent>;
  /** Get a single #struct_name_str by ID */
  TorrentFile?: Maybe<TorrentFile>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  TorrentFiles: TorrentFileConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Torrents: TorrentConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  TorznabCategories: TorznabCategoryConnection;
  /** Get a single #struct_name_str by ID */
  TorznabCategory?: Maybe<TorznabCategory>;
  /** Get a single #struct_name_str by ID */
  Track?: Maybe<Track>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Tracks: TrackConnection;
  /** Get a single #struct_name_str by ID */
  UsenetDownload?: Maybe<UsenetDownload>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  UsenetDownloads: UsenetDownloadConnection;
  /** Get a single #struct_name_str by ID */
  UsenetServer?: Maybe<UsenetServer>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  UsenetServers: UsenetServerConnection;
  /** Get a single #struct_name_str by ID */
  User?: Maybe<User>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Users: UserConnection;
  /** Get a single #struct_name_str by ID */
  VideoStream?: Maybe<VideoStream>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  VideoStreams: VideoStreamConnection;
};

export type QueryRootAlbumArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootAlbumsArgs = {
  OrderBy?: InputMaybe<Array<AlbumOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AlbumWhereInput>;
};

export type QueryRootAppLogArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootAppLogsArgs = {
  OrderBy?: InputMaybe<Array<AppLogOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AppLogWhereInput>;
};

export type QueryRootAppSettingArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootAppSettingsArgs = {
  OrderBy?: InputMaybe<Array<AppSettingOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AppSettingWhereInput>;
};

export type QueryRootArtistArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootArtistsArgs = {
  OrderBy?: InputMaybe<Array<ArtistOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ArtistWhereInput>;
};

export type QueryRootArtworkCacheArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootArtworkCachesArgs = {
  OrderBy?: InputMaybe<Array<ArtworkCacheOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ArtworkCacheWhereInput>;
};

export type QueryRootAudioStreamArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootAudioStreamsArgs = {
  OrderBy?: InputMaybe<Array<AudioStreamOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AudioStreamWhereInput>;
};

export type QueryRootAudiobookArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootAudiobooksArgs = {
  OrderBy?: InputMaybe<Array<AudiobookOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<AudiobookWhereInput>;
};

export type QueryRootBrowseDirectoryArgs = {
  Input?: InputMaybe<BrowseDirectoryInput>;
};

export type QueryRootCastDeviceArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootCastDevicesArgs = {
  OrderBy?: InputMaybe<Array<CastDeviceOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<CastDeviceWhereInput>;
};

export type QueryRootCastSessionArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootCastSessionsArgs = {
  OrderBy?: InputMaybe<Array<CastSessionOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<CastSessionWhereInput>;
};

export type QueryRootCastSettingArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootCastSettingsArgs = {
  OrderBy?: InputMaybe<Array<CastSettingOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<CastSettingWhereInput>;
};

export type QueryRootChapterArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootChaptersArgs = {
  OrderBy?: InputMaybe<Array<ChapterOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ChapterWhereInput>;
};

export type QueryRootEpisodeArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootEpisodesArgs = {
  OrderBy?: InputMaybe<Array<EpisodeOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<EpisodeWhereInput>;
};

export type QueryRootInviteTokenArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootInviteTokensArgs = {
  OrderBy?: InputMaybe<Array<InviteTokenOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<InviteTokenWhereInput>;
};

export type QueryRootLibrariesArgs = {
  OrderBy?: InputMaybe<Array<LibraryOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<LibraryWhereInput>;
};

export type QueryRootLibraryArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootLibraryPathAvailabilityArgs = {
  Input: LibraryPathAvailabilityInput;
};

export type QueryRootLiveTorrentArgs = {
  Id: Scalars["Int"]["input"];
};

export type QueryRootMediaChapterArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootMediaChaptersArgs = {
  OrderBy?: InputMaybe<Array<MediaChapterOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<MediaChapterWhereInput>;
};

export type QueryRootMediaFileArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootMediaFilesArgs = {
  OrderBy?: InputMaybe<Array<MediaFileOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<MediaFileWhereInput>;
};

export type QueryRootMovieArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootMoviesArgs = {
  OrderBy?: InputMaybe<Array<MovieOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<MovieWhereInput>;
};

export type QueryRootNamingPatternArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootNamingPatternsArgs = {
  OrderBy?: InputMaybe<Array<NamingPatternOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<NamingPatternWhereInput>;
};

export type QueryRootNotificationArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootNotificationsArgs = {
  OrderBy?: InputMaybe<Array<NotificationOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<NotificationWhereInput>;
};

export type QueryRootPendingFileMatchArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootPendingFileMatchesArgs = {
  OrderBy?: InputMaybe<Array<PendingFileMatchOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<PendingFileMatchWhereInput>;
};

export type QueryRootPlaybackProgressArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootPlaybackProgressesArgs = {
  OrderBy?: InputMaybe<Array<PlaybackProgressOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<PlaybackProgressWhereInput>;
};

export type QueryRootPlaybackSessionArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootPlaybackSessionsArgs = {
  OrderBy?: InputMaybe<Array<PlaybackSessionOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<PlaybackSessionWhereInput>;
};

export type QueryRootRefreshTokenArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootRefreshTokensArgs = {
  OrderBy?: InputMaybe<Array<RefreshTokenOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<RefreshTokenWhereInput>;
};

export type QueryRootRssFeedArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootRssFeedItemArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootRssFeedItemsArgs = {
  OrderBy?: InputMaybe<Array<RssFeedItemOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<RssFeedItemWhereInput>;
};

export type QueryRootRssFeedsArgs = {
  OrderBy?: InputMaybe<Array<RssFeedOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<RssFeedWhereInput>;
};

export type QueryRootScheduleCacheArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootScheduleCachesArgs = {
  OrderBy?: InputMaybe<Array<ScheduleCacheOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ScheduleCacheWhereInput>;
};

export type QueryRootScheduleSyncStateArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootScheduleSyncStatesArgs = {
  OrderBy?: InputMaybe<Array<ScheduleSyncStateOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ScheduleSyncStateWhereInput>;
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

export type QueryRootShowArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootShowsArgs = {
  OrderBy?: InputMaybe<Array<ShowOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ShowWhereInput>;
};

export type QueryRootSourceArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootSourcePriorityRuleArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootSourcePriorityRulesArgs = {
  OrderBy?: InputMaybe<Array<SourcePriorityRuleOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<SourcePriorityRuleWhereInput>;
};

export type QueryRootSourceSettingDefinitionsArgs = {
  DefinitionId: Scalars["String"]["input"];
};

export type QueryRootSourcesArgs = {
  OrderBy?: InputMaybe<Array<SourceOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<SourceWhereInput>;
};

export type QueryRootSubtitleArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootSubtitlesArgs = {
  OrderBy?: InputMaybe<Array<SubtitleOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<SubtitleWhereInput>;
};

export type QueryRootTorrentArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootTorrentFileArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootTorrentFilesArgs = {
  OrderBy?: InputMaybe<Array<TorrentFileOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<TorrentFileWhereInput>;
};

export type QueryRootTorrentsArgs = {
  OrderBy?: InputMaybe<Array<TorrentOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<TorrentWhereInput>;
};

export type QueryRootTorznabCategoriesArgs = {
  OrderBy?: InputMaybe<Array<TorznabCategoryOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<TorznabCategoryWhereInput>;
};

export type QueryRootTorznabCategoryArgs = {
  Id: Scalars["Int"]["input"];
};

export type QueryRootTrackArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootTracksArgs = {
  OrderBy?: InputMaybe<Array<TrackOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<TrackWhereInput>;
};

export type QueryRootUsenetDownloadArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootUsenetDownloadsArgs = {
  OrderBy?: InputMaybe<Array<UsenetDownloadOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<UsenetDownloadWhereInput>;
};

export type QueryRootUsenetServerArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootUsenetServersArgs = {
  OrderBy?: InputMaybe<Array<UsenetServerOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<UsenetServerWhereInput>;
};

export type QueryRootUserArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootUsersArgs = {
  OrderBy?: InputMaybe<Array<UserOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<UserWhereInput>;
};

export type QueryRootVideoStreamArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootVideoStreamsArgs = {
  OrderBy?: InputMaybe<Array<VideoStreamOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<VideoStreamWhereInput>;
};

export type RefreshToken = {
  CreatedAt: Scalars["String"]["output"];
  DeviceInfo?: Maybe<Scalars["String"]["output"]>;
  ExpiresAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  IpAddress?: Maybe<Scalars["String"]["output"]>;
  LastUsedAt?: Maybe<Scalars["String"]["output"]>;
  TokenHash: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type RefreshTokenChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  RefreshToken?: Maybe<RefreshToken>;
};

/** Connection containing edges and page info */
export type RefreshTokenConnection = {
  /** The edges in this connection */
  Edges: Array<RefreshTokenEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type RefreshTokenEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: RefreshToken;
};

/** GraphQL input for refresh token mutation. */
export type RefreshTokenInput = {
  RefreshToken: Scalars["String"]["input"];
};

export type RefreshTokenOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  ExpiresAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type RefreshTokenResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  RefreshToken?: Maybe<RefreshToken>;
  Success: Scalars["Boolean"]["output"];
};

export type RefreshTokenWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<RefreshTokenWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  ExpiresAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastUsedAt?: InputMaybe<DateFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<RefreshTokenWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<RefreshTokenWhereInput>>;
  TokenHash?: InputMaybe<StringFilter>;
  UserId?: InputMaybe<StringFilter>;
};

/** GraphQL input for user registration (PascalCase field names). */
export type RegisterUserInput = {
  Email: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Password: Scalars["String"]["input"];
};

/** Relative date specification for date arithmetic */
export type RelativeDate = {
  /** Number of days ago (positive = past) */
  DaysAgo?: InputMaybe<Scalars["Int"]["input"]>;
  /** Number of days from now (positive = future) */
  DaysFromNow?: InputMaybe<Scalars["Int"]["input"]>;
  /** Use today's date */
  Today?: InputMaybe<Scalars["Boolean"]["input"]>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  RssFeed?: Maybe<RssFeed>;
};

/** Connection containing edges and page info */
export type RssFeedConnection = {
  /** The edges in this connection */
  Edges: Array<RssFeedEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type RssFeedEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: RssFeed;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  RssFeedItem?: Maybe<RssFeedItem>;
};

/** Connection containing edges and page info */
export type RssFeedItemConnection = {
  /** The edges in this connection */
  Edges: Array<RssFeedItemEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type RssFeedItemEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: RssFeedItem;
};

export type RssFeedItemOrderByInput = {
  PubDate?: InputMaybe<SortDirection>;
  SeenAt?: InputMaybe<SortDirection>;
  Title?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type RssFeedItemResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  RssFeedItem?: Maybe<RssFeedItem>;
  Success: Scalars["Boolean"]["output"];
};

export type RssFeedItemWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<RssFeedItemWhereInput>>;
  FeedId?: InputMaybe<StringFilter>;
  Guid?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  LinkHash?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<RssFeedItemWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<RssFeedItemWhereInput>>;
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
};

export type RssFeedOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  LastPolledAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type RssFeedResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  RssFeed?: Maybe<RssFeed>;
  Success: Scalars["Boolean"]["output"];
};

export type RssFeedWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<RssFeedWhereInput>>;
  ConsecutiveFailures?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  LastPolledAt?: InputMaybe<DateFilter>;
  LastSuccessfulAt?: InputMaybe<DateFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<RssFeedWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<RssFeedWhereInput>>;
  PollIntervalMinutes?: InputMaybe<IntFilter>;
  PostDownloadAction?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Url?: InputMaybe<StringFilter>;
  UserId?: InputMaybe<StringFilter>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  ScheduleCache?: Maybe<ScheduleCache>;
};

/** Connection containing edges and page info */
export type ScheduleCacheConnection = {
  /** The edges in this connection */
  Edges: Array<ScheduleCacheEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ScheduleCacheEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: ScheduleCache;
};

export type ScheduleCacheOrderByInput = {
  AirDate?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  EpisodeName?: InputMaybe<SortDirection>;
  EpisodeNumber?: InputMaybe<SortDirection>;
  Season?: InputMaybe<SortDirection>;
  ShowName?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type ScheduleCacheResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  ScheduleCache?: Maybe<ScheduleCache>;
  Success: Scalars["Boolean"]["output"];
};

export type ScheduleCacheWhereInput = {
  AirDate?: InputMaybe<DateFilter>;
  AirStamp?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<ScheduleCacheWhereInput>>;
  CountryCode?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EpisodeName?: InputMaybe<StringFilter>;
  EpisodeNumber?: InputMaybe<IntFilter>;
  EpisodeType?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<ScheduleCacheWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<ScheduleCacheWhereInput>>;
  Runtime?: InputMaybe<IntFilter>;
  Season?: InputMaybe<IntFilter>;
  ShowName?: InputMaybe<StringFilter>;
  ShowNetwork?: InputMaybe<StringFilter>;
  TvmazeEpisodeId?: InputMaybe<IntFilter>;
  TvmazeShowId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  ScheduleSyncState?: Maybe<ScheduleSyncState>;
};

/** Connection containing edges and page info */
export type ScheduleSyncStateConnection = {
  /** The edges in this connection */
  Edges: Array<ScheduleSyncStateEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ScheduleSyncStateEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: ScheduleSyncState;
};

export type ScheduleSyncStateOrderByInput = {
  CountryCode?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  LastSyncedAt?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type ScheduleSyncStateResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  ScheduleSyncState?: Maybe<ScheduleSyncState>;
  Success: Scalars["Boolean"]["output"];
};

export type ScheduleSyncStateWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<ScheduleSyncStateWhereInput>>;
  CountryCode?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastSyncDays?: InputMaybe<IntFilter>;
  LastSyncedAt?: InputMaybe<DateFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<ScheduleSyncStateWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<ScheduleSyncStateWhereInput>>;
  UpdatedAt?: InputMaybe<DateFilter>;
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
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Episodes: EpisodeConnection;
  Genres: Array<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  ImdbId?: Maybe<Scalars["String"]["output"]>;
  /** Get related #graphql_name */
  Library?: Maybe<Library>;
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
export type ShowEpisodesArgs = {
  OrderBy?: InputMaybe<EpisodeOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<EpisodeWhereInput>;
};

/** Event for #struct_name changes (subscriptions) */
export type ShowChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Show?: Maybe<Show>;
};

/** Connection containing edges and page info */
export type ShowConnection = {
  /** The edges in this connection */
  Edges: Array<ShowEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type ShowEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Show;
};

export type ShowOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  SortName?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
  Year?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type ShowResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Show?: Maybe<Show>;
  Success: Scalars["Boolean"]["output"];
};

export type ShowWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<ShowWhereInput>>;
  AutoDownload?: InputMaybe<BoolFilter>;
  ContentRating?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  ImdbId?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  Network?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<ShowWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<ShowWhereInput>>;
  Runtime?: InputMaybe<IntFilter>;
  TmdbId?: InputMaybe<IntFilter>;
  TvdbId?: InputMaybe<IntFilter>;
  TvmazeId?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Year?: InputMaybe<IntFilter>;
};

/** Fuzzy matching filter for string similarity */
export type SimilarFilter = {
  /**
   * Minimum similarity threshold (0.0-1.0, default 0.6)
   * 1.0 = exact match, 0.0 = any match
   */
  Threshold?: InputMaybe<Scalars["Float"]["input"]>;
  /** The text to match against */
  Value: Scalars["String"]["input"];
};

/** Sort direction for ORDER BY clauses. */
export const SortDirection = {
  /** Ascending order (A-Z, 1-9, oldest-newest) */
  Asc: "Asc",
  /** Descending order (Z-A, 9-1, newest-oldest) */
  Desc: "Desc",
} as const;

export type SortDirection = (typeof SortDirection)[keyof typeof SortDirection];
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Source?: Maybe<Source>;
};

/** Connection containing edges and page info */
export type SourceConnection = {
  /** The edges in this connection */
  Edges: Array<SourceEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
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
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Source;
};

/** Generic success/error result for source mutations */
export type SourceMutationResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

export type SourceOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  Priority?: InputMaybe<SortDirection>;
  SourceType?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  SourcePriorityRule?: Maybe<SourcePriorityRule>;
};

/** Connection containing edges and page info */
export type SourcePriorityRuleConnection = {
  /** The edges in this connection */
  Edges: Array<SourcePriorityRuleEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type SourcePriorityRuleEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: SourcePriorityRule;
};

export type SourcePriorityRuleOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type SourcePriorityRuleResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  SourcePriorityRule?: Maybe<SourcePriorityRule>;
  Success: Scalars["Boolean"]["output"];
};

export type SourcePriorityRuleWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<SourcePriorityRuleWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  Id?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  LibraryType?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<SourcePriorityRuleWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<SourcePriorityRuleWhereInput>>;
  SearchAllSources?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
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
  Error?: Maybe<Scalars["String"]["output"]>;
  Source?: Maybe<Source>;
  Success: Scalars["Boolean"]["output"];
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
  /** Logical AND of conditions */
  And?: InputMaybe<Array<SourceWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  DefinitionId?: InputMaybe<StringFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  ErrorCount?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  LastErrorAt?: InputMaybe<DateFilter>;
  LastSuccessAt?: InputMaybe<DateFilter>;
  MediaTypes?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<SourceWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<SourceWhereInput>>;
  Priority?: InputMaybe<IntFilter>;
  SourceType?: InputMaybe<StringFilter>;
  SupportsBookSearch?: InputMaybe<BoolFilter>;
  SupportsMovieSearch?: InputMaybe<BoolFilter>;
  SupportsMusicSearch?: InputMaybe<BoolFilter>;
  SupportsSearch?: InputMaybe<BoolFilter>;
  SupportsTvSearch?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
};

/** Filter for string fields */
export type StringFilter = {
  /** Contains substring (case-insensitive) */
  Contains?: InputMaybe<Scalars["String"]["input"]>;
  /** Ends with */
  EndsWith?: InputMaybe<Scalars["String"]["input"]>;
  /** Equals */
  Eq?: InputMaybe<Scalars["String"]["input"]>;
  /** In list */
  In?: InputMaybe<Array<Scalars["String"]["input"]>>;
  /** Is null */
  IsNull?: InputMaybe<Scalars["Boolean"]["input"]>;
  /** Not equals */
  Ne?: InputMaybe<Scalars["String"]["input"]>;
  /** Not in list */
  NotIn?: InputMaybe<Array<Scalars["String"]["input"]>>;
  /**
   * Fuzzy/similar match with optional threshold (0.0-1.0, default 0.6)
   * Uses normalized Levenshtein distance for scoring
   */
  Similar?: InputMaybe<SimilarFilter>;
  /** Starts with */
  StartsWith?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for entity change subscriptions. */
export type SubscriptionFilterInput = {
  /** Only receive events of these types */
  Actions?: InputMaybe<Array<ChangeAction>>;
  /** Only receive events for entities matching this ID */
  Id?: InputMaybe<Scalars["String"]["input"]>;
};

export type SubscriptionRoot = {
  /** Subscribe to #struct_name_str changes */
  AlbumChanged: AlbumChangedEvent;
  /** Subscribe to #struct_name_str changes */
  AppLogChanged: AppLogChangedEvent;
  /** Subscribe to #struct_name_str changes */
  AppSettingChanged: AppSettingChangedEvent;
  /** Subscribe to #struct_name_str changes */
  ArtistChanged: ArtistChangedEvent;
  /** Subscribe to #struct_name_str changes */
  ArtworkCacheChanged: ArtworkCacheChangedEvent;
  /** Subscribe to #struct_name_str changes */
  AudioStreamChanged: AudioStreamChangedEvent;
  /** Subscribe to #struct_name_str changes */
  AudiobookChanged: AudiobookChangedEvent;
  /** Subscribe to #struct_name_str changes */
  CastDeviceChanged: CastDeviceChangedEvent;
  /** Subscribe to #struct_name_str changes */
  CastSessionChanged: CastSessionChangedEvent;
  /** Subscribe to #struct_name_str changes */
  CastSettingChanged: CastSettingChangedEvent;
  /** Subscribe to #struct_name_str changes */
  ChapterChanged: ChapterChangedEvent;
  /** Subscribe to #struct_name_str changes */
  EpisodeChanged: EpisodeChangedEvent;
  /**
   * Subscribe to filesystem change events (create/delete/copy/move/rename).
   * Fires when any filesystem mutation completes. Optional path filter.
   */
  FilesystemChanged: FilesystemChangeEvent;
  /** Subscribe to #struct_name_str changes */
  InviteTokenChanged: InviteTokenChangedEvent;
  /** Subscribe to #struct_name_str changes */
  LibraryChanged: LibraryChangedEvent;
  /** Subscribe to #struct_name_str changes */
  MediaChapterChanged: MediaChapterChangedEvent;
  /** Subscribe to #struct_name_str changes */
  MediaFileChanged: MediaFileChangedEvent;
  /** Subscribe to #struct_name_str changes */
  MovieChanged: MovieChangedEvent;
  /** Subscribe to #struct_name_str changes */
  NamingPatternChanged: NamingPatternChangedEvent;
  /** Subscribe to #struct_name_str changes */
  NotificationChanged: NotificationChangedEvent;
  /** Subscribe to #struct_name_str changes */
  PendingFileMatchChanged: PendingFileMatchChangedEvent;
  /** Subscribe to #struct_name_str changes */
  PlaybackProgressChanged: PlaybackProgressChangedEvent;
  /** Subscribe to #struct_name_str changes */
  PlaybackSessionChanged: PlaybackSessionChangedEvent;
  /** Subscribe to #struct_name_str changes */
  RefreshTokenChanged: RefreshTokenChangedEvent;
  /** Subscribe to #struct_name_str changes */
  RssFeedChanged: RssFeedChangedEvent;
  /** Subscribe to #struct_name_str changes */
  RssFeedItemChanged: RssFeedItemChangedEvent;
  /** Subscribe to #struct_name_str changes */
  ScheduleCacheChanged: ScheduleCacheChangedEvent;
  /** Subscribe to #struct_name_str changes */
  ScheduleSyncStateChanged: ScheduleSyncStateChangedEvent;
  /** Subscribe to #struct_name_str changes */
  ShowChanged: ShowChangedEvent;
  /** Subscribe to #struct_name_str changes */
  SourceChanged: SourceChangedEvent;
  /** Subscribe to #struct_name_str changes */
  SourcePriorityRuleChanged: SourcePriorityRuleChangedEvent;
  /** Subscribe to #struct_name_str changes */
  SubtitleChanged: SubtitleChangedEvent;
  /** Subscribe to #struct_name_str changes */
  TorrentFileChanged: TorrentFileChangedEvent;
  /** Subscribe to #struct_name_str changes */
  TorznabCategoryChanged: TorznabCategoryChangedEvent;
  /** Subscribe to #struct_name_str changes */
  TrackChanged: TrackChangedEvent;
  /** Subscribe to #struct_name_str changes */
  UsenetDownloadChanged: UsenetDownloadChangedEvent;
  /** Subscribe to #struct_name_str changes */
  UsenetServerChanged: UsenetServerChangedEvent;
  /** Subscribe to #struct_name_str changes */
  UserChanged: UserChangedEvent;
  /** Subscribe to #struct_name_str changes */
  VideoStreamChanged: VideoStreamChangedEvent;
  torrentAdded: TorrentAddedEvent;
  torrentCompleted: TorrentCompletedEvent;
  torrentProgress: TorrentProgress;
  torrentRemoved: TorrentRemovedEvent;
};

export type SubscriptionRootAlbumChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootAppLogChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootAppSettingChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootArtistChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootArtworkCacheChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootAudioStreamChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootAudiobookChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootCastDeviceChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootCastSessionChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootCastSettingChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootChapterChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootEpisodeChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootFilesystemChangedArgs = {
  Path?: InputMaybe<Scalars["String"]["input"]>;
};

export type SubscriptionRootInviteTokenChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootLibraryChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootMediaChapterChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootMediaFileChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootMovieChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootNamingPatternChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootNotificationChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootPendingFileMatchChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootPlaybackProgressChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootPlaybackSessionChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootRefreshTokenChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootRssFeedChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootRssFeedItemChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootScheduleCacheChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootScheduleSyncStateChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootShowChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootSourceChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootSourcePriorityRuleChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootSubtitleChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootTorrentFileChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootTorznabCategoryChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootTrackChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootUsenetDownloadChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootUsenetServerChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootUserChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootVideoStreamChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Subtitle?: Maybe<Subtitle>;
};

/** Connection containing edges and page info */
export type SubtitleConnection = {
  /** The edges in this connection */
  Edges: Array<SubtitleEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type SubtitleEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Subtitle;
};

export type SubtitleOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type SubtitleResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Subtitle?: Maybe<Subtitle>;
  Success: Scalars["Boolean"]["output"];
};

export type SubtitleWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<SubtitleWhereInput>>;
  Codec?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DownloadedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IsDefault?: InputMaybe<BoolFilter>;
  IsForced?: InputMaybe<BoolFilter>;
  IsHearingImpaired?: InputMaybe<BoolFilter>;
  Language?: InputMaybe<StringFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<SubtitleWhereInput>;
  OpensubtitlesId?: InputMaybe<StringFilter>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<SubtitleWhereInput>>;
  SourceType?: InputMaybe<StringFilter>;
  StreamIndex?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
};

export type Torrent = {
  AddedAt: Scalars["String"]["output"];
  CompletedAt?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  DownloadPath?: Maybe<Scalars["String"]["output"]>;
  DownloadedBytes: Scalars["Int"]["output"];
  ExcludedFiles: Array<Scalars["Int"]["output"]>;
  /**
   * Get related #graphql_name with optional filtering, sorting, and pagination.
   *
   * When no arguments are provided, uses DataLoader to batch queries and
   * avoid N+1 when loading relations for multiple parent entities.
   * When filter/sort/pagination arguments are provided, uses direct
   * database query for full SQL support.
   */
  Files: TorrentFileConnection;
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
};

export type TorrentFilesArgs = {
  OrderBy?: InputMaybe<TorrentFileOrderByInput>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<TorrentFileWhereInput>;
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
  Edges: Array<TorrentEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TorrentEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Torrent;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  TorrentFile?: Maybe<TorrentFile>;
};

/** Connection containing edges and page info */
export type TorrentFileConnection = {
  /** The edges in this connection */
  Edges: Array<TorrentFileEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TorrentFileEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: TorrentFile;
};

export type TorrentFileOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  FileIndex?: InputMaybe<SortDirection>;
  FileSize?: InputMaybe<SortDirection>;
  Progress?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type TorrentFileResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  TorrentFile?: Maybe<TorrentFile>;
};

export type TorrentFileWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<TorrentFileWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  DownloadedBytes?: InputMaybe<IntFilter>;
  FileIndex?: InputMaybe<IntFilter>;
  FilePath?: InputMaybe<StringFilter>;
  FileSize?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IsExcluded?: InputMaybe<BoolFilter>;
  MediaFileId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<TorrentFileWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<TorrentFileWhereInput>>;
  Progress?: InputMaybe<IntFilter>;
  RelativePath?: InputMaybe<StringFilter>;
  TorrentId?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
};

export type TorrentOrderByInput = {
  AddedAt?: InputMaybe<SortDirection>;
  CreatedAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  Progress?: InputMaybe<SortDirection>;
  State?: InputMaybe<SortDirection>;
  TotalBytes?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
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
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  Torrent?: Maybe<Torrent>;
};

export type TorrentWhereInput = {
  AddedAt?: InputMaybe<DateFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<TorrentWhereInput>>;
  CompletedAt?: InputMaybe<DateFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  DownloadedBytes?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  InfoHash?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<TorrentWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<TorrentWhereInput>>;
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
};

export type TorznabCategory = {
  Description?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["Int"]["output"];
  Name: Scalars["String"]["output"];
  ParentId?: Maybe<Scalars["Int"]["output"]>;
};

/** Event for #struct_name changes (subscriptions) */
export type TorznabCategoryChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["Int"]["output"];
  TorznabCategory?: Maybe<TorznabCategory>;
};

/** Connection containing edges and page info */
export type TorznabCategoryConnection = {
  /** The edges in this connection */
  Edges: Array<TorznabCategoryEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TorznabCategoryEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: TorznabCategory;
};

export type TorznabCategoryOrderByInput = {
  Id?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type TorznabCategoryResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  TorznabCategory?: Maybe<TorznabCategory>;
};

export type TorznabCategoryWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<TorznabCategoryWhereInput>>;
  Id?: InputMaybe<IntFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<TorznabCategoryWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<TorznabCategoryWhereInput>>;
  ParentId?: InputMaybe<IntFilter>;
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
  /**
   * Computed status based on playback, file availability, and download state
   *
   * Returns one of: PLAYING, PAUSED, AVAILABLE, DOWNLOADING, WANTED, MISSING
   */
  Status: ContentStatus;
  Title: Scalars["String"]["output"];
  TrackNumber: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  Wanted: Scalars["Boolean"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type TrackChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Track?: Maybe<Track>;
};

/** Connection containing edges and page info */
export type TrackConnection = {
  /** The edges in this connection */
  Edges: Array<TrackEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type TrackEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: Track;
};

export type TrackOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  DiscNumber?: InputMaybe<SortDirection>;
  DurationSecs?: InputMaybe<SortDirection>;
  Title?: InputMaybe<SortDirection>;
  TrackNumber?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type TrackResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  Track?: Maybe<Track>;
};

export type TrackWhereInput = {
  AlbumId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<TrackWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<TrackWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<TrackWhereInput>>;
  Title?: InputMaybe<StringFilter>;
  TrackNumber?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Wanted?: InputMaybe<BoolFilter>;
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

/** Input for updating an existing #struct_name */
export type UpdateAlbumInput = {
  AlbumType?: InputMaybe<Scalars["String"]["input"]>;
  ArtistId?: InputMaybe<Scalars["String"]["input"]>;
  AutoDownload?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoDownloadMode?: InputMaybe<AutoDownloadMode>;
  Country?: InputMaybe<Scalars["String"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  DiscCount?: InputMaybe<Scalars["Int"]["input"]>;
  Genres?: InputMaybe<Array<Scalars["String"]["input"]>>;
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

/** Input for updating an existing #struct_name */
export type UpdateAppLogInput = {
  Fields?: InputMaybe<Scalars["String"]["input"]>;
  Level?: InputMaybe<Scalars["String"]["input"]>;
  Message?: InputMaybe<Scalars["String"]["input"]>;
  SpanId?: InputMaybe<Scalars["String"]["input"]>;
  SpanName?: InputMaybe<Scalars["String"]["input"]>;
  Target?: InputMaybe<Scalars["String"]["input"]>;
  Timestamp?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
export type UpdateAppSettingInput = {
  Category?: InputMaybe<Scalars["String"]["input"]>;
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Key?: InputMaybe<Scalars["String"]["input"]>;
  Value?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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
  Narrators?: InputMaybe<Array<Scalars["String"]["input"]>>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PublishedDate?: InputMaybe<Scalars["String"]["input"]>;
  Publisher?: InputMaybe<Scalars["String"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
export type UpdateCastSettingInput = {
  AutoDiscoveryEnabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  DefaultVolume?: InputMaybe<Scalars["Float"]["input"]>;
  DiscoveryIntervalSeconds?: InputMaybe<Scalars["Int"]["input"]>;
  PreferredQuality?: InputMaybe<Scalars["String"]["input"]>;
  TranscodeIncompatible?: InputMaybe<Scalars["Boolean"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
export type UpdateInviteTokenInput = {
  AccessLevel?: InputMaybe<Scalars["String"]["input"]>;
  ApplyRestrictions?: InputMaybe<Scalars["Boolean"]["input"]>;
  CreatedBy?: InputMaybe<Scalars["String"]["input"]>;
  ExpiresAt?: InputMaybe<Scalars["String"]["input"]>;
  IsActive?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryIds?: InputMaybe<Array<Scalars["String"]["input"]>>;
  MaxUses?: InputMaybe<Scalars["Int"]["input"]>;
  RestrictionsTemplate?: InputMaybe<Scalars["String"]["input"]>;
  Role?: InputMaybe<Scalars["String"]["input"]>;
  Token?: InputMaybe<Scalars["String"]["input"]>;
  UseCount?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
export type UpdateMediaChapterInput = {
  ChapterIndex?: InputMaybe<Scalars["Int"]["input"]>;
  EndSecs?: InputMaybe<Scalars["Float"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  StartSecs?: InputMaybe<Scalars["Float"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
export type UpdateMediaFileInput = {
  AddedAt?: InputMaybe<Scalars["String"]["input"]>;
  AudioChannels?: InputMaybe<Scalars["String"]["input"]>;
  AudioCodec?: InputMaybe<Scalars["String"]["input"]>;
  Bitrate?: InputMaybe<Scalars["Int"]["input"]>;
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

/** Input for updating an existing #struct_name */
export type UpdateMovieInput = {
  CastNames?: InputMaybe<Array<Scalars["String"]["input"]>>;
  Certification?: InputMaybe<Scalars["String"]["input"]>;
  CollectionId?: InputMaybe<Scalars["Int"]["input"]>;
  CollectionName?: InputMaybe<Scalars["String"]["input"]>;
  CollectionPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Director?: InputMaybe<Scalars["String"]["input"]>;
  DownloadStatus?: InputMaybe<Scalars["String"]["input"]>;
  Genres?: InputMaybe<Array<Scalars["String"]["input"]>>;
  HasFile?: InputMaybe<Scalars["Boolean"]["input"]>;
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Monitored?: InputMaybe<Scalars["Boolean"]["input"]>;
  OriginalTitle?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  ProductionCountries?: InputMaybe<Array<Scalars["String"]["input"]>>;
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  SpokenLanguages?: InputMaybe<Array<Scalars["String"]["input"]>>;
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

/** Input for updating an existing #struct_name */
export type UpdateNamingPatternInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  IsDefault?: InputMaybe<Scalars["Boolean"]["input"]>;
  IsSystem?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Pattern?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
export type UpdatePlaybackProgressInput = {
  CurrentPosition?: InputMaybe<Scalars["Float"]["input"]>;
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  IsWatched?: InputMaybe<Scalars["Boolean"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  ProgressPercent?: InputMaybe<Scalars["Float"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  WatchedAt?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
export type UpdateRefreshTokenInput = {
  DeviceInfo?: InputMaybe<Scalars["String"]["input"]>;
  ExpiresAt?: InputMaybe<Scalars["String"]["input"]>;
  IpAddress?: InputMaybe<Scalars["String"]["input"]>;
  LastUsedAt?: InputMaybe<Scalars["String"]["input"]>;
  TokenHash?: InputMaybe<Scalars["String"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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
  ShowGenres?: InputMaybe<Array<Scalars["String"]["input"]>>;
  ShowName?: InputMaybe<Scalars["String"]["input"]>;
  ShowNetwork?: InputMaybe<Scalars["String"]["input"]>;
  ShowPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Summary?: InputMaybe<Scalars["String"]["input"]>;
  TvmazeEpisodeId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeShowId?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Input for updating an existing #struct_name */
export type UpdateScheduleSyncStateInput = {
  CountryCode?: InputMaybe<Scalars["String"]["input"]>;
  LastSyncDays?: InputMaybe<Scalars["Int"]["input"]>;
  LastSyncedAt?: InputMaybe<Scalars["String"]["input"]>;
  SyncError?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
export type UpdateShowInput = {
  AutoDownload?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoDownloadMode?: InputMaybe<AutoDownloadMode>;
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  ContentRating?: InputMaybe<Scalars["String"]["input"]>;
  Genres?: InputMaybe<Array<Scalars["String"]["input"]>>;
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

/** Input for updating an existing #struct_name */
export type UpdateSourceInput = {
  Credentials?: InputMaybe<Scalars["String"]["input"]>;
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
};

/** Input for updating source priorities */
export type UpdateSourcePrioritiesInput = {
  /** Source IDs in the desired priority order (first = highest priority) */
  SourceIds: Array<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
export type UpdateSourcePriorityRuleInput = {
  Enabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  PriorityOrder?: InputMaybe<Array<Scalars["String"]["input"]>>;
  SearchAllSources?: InputMaybe<Scalars["Boolean"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
export type UpdateTorrentInput = {
  AddedAt?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  DownloadPath?: InputMaybe<Scalars["String"]["input"]>;
  DownloadedBytes?: InputMaybe<Scalars["Int"]["input"]>;
  ExcludedFiles?: InputMaybe<Array<Scalars["Int"]["input"]>>;
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

/** Input for updating an existing #struct_name */
export type UpdateTorznabCategoryInput = {
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  ParentId?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
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

/** Input for updating an existing #struct_name */
export type UpdateUserInput = {
  AvatarUrl?: InputMaybe<Scalars["String"]["input"]>;
  DisplayName?: InputMaybe<Scalars["String"]["input"]>;
  Email?: InputMaybe<Scalars["String"]["input"]>;
  IsActive?: InputMaybe<Scalars["Boolean"]["input"]>;
  LastLoginAt?: InputMaybe<Scalars["String"]["input"]>;
  Role?: InputMaybe<Scalars["String"]["input"]>;
  Username?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  UsenetDownload?: Maybe<UsenetDownload>;
};

/** Connection containing edges and page info */
export type UsenetDownloadConnection = {
  /** The edges in this connection */
  Edges: Array<UsenetDownloadEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type UsenetDownloadEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: UsenetDownload;
};

export type UsenetDownloadOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  NzbName?: InputMaybe<SortDirection>;
  SizeBytes?: InputMaybe<SortDirection>;
  State?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type UsenetDownloadResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  UsenetDownload?: Maybe<UsenetDownload>;
};

export type UsenetDownloadWhereInput = {
  AlbumId?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<UsenetDownloadWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<UsenetDownloadWhereInput>;
  NzbHash?: InputMaybe<StringFilter>;
  NzbName?: InputMaybe<StringFilter>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<UsenetDownloadWhereInput>>;
  PostProcessStatus?: InputMaybe<StringFilter>;
  RetryCount?: InputMaybe<IntFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  State?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  UsenetServer?: Maybe<UsenetServer>;
};

/** Connection containing edges and page info */
export type UsenetServerConnection = {
  /** The edges in this connection */
  Edges: Array<UsenetServerEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type UsenetServerEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: UsenetServer;
};

export type UsenetServerOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  Priority?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type UsenetServerResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  UsenetServer?: Maybe<UsenetServer>;
};

export type UsenetServerWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<UsenetServerWhereInput>>;
  Connections?: InputMaybe<IntFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  ErrorCount?: InputMaybe<IntFilter>;
  Host?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  LastSuccessAt?: InputMaybe<DateFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<UsenetServerWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<UsenetServerWhereInput>>;
  Port?: InputMaybe<IntFilter>;
  Priority?: InputMaybe<IntFilter>;
  RetentionDays?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UseSsl?: InputMaybe<BoolFilter>;
  UserId?: InputMaybe<StringFilter>;
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
};

/** Event for #struct_name changes (subscriptions) */
export type UserChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  User?: Maybe<User>;
};

/** Connection containing edges and page info */
export type UserConnection = {
  /** The edges in this connection */
  Edges: Array<UserEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type UserEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: User;
};

export type UserOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  LastLoginAt?: InputMaybe<SortDirection>;
  Role?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
  Username?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type UserResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  User?: Maybe<User>;
};

export type UserWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<UserWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  DisplayName?: InputMaybe<StringFilter>;
  Email?: InputMaybe<StringFilter>;
  Id?: InputMaybe<StringFilter>;
  IsActive?: InputMaybe<BoolFilter>;
  LastLoginAt?: InputMaybe<DateFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<UserWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<UserWhereInput>>;
  Role?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  Username?: InputMaybe<StringFilter>;
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
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  VideoStream?: Maybe<VideoStream>;
};

/** Connection containing edges and page info */
export type VideoStreamConnection = {
  /** The edges in this connection */
  Edges: Array<VideoStreamEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type VideoStreamEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: VideoStream;
};

export type VideoStreamOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  StreamIndex?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type VideoStreamResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
  VideoStream?: Maybe<VideoStream>;
};

export type VideoStreamWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<VideoStreamWhereInput>>;
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
  /** Logical NOT of condition */
  Not?: InputMaybe<VideoStreamWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<VideoStreamWhereInput>>;
  StreamIndex?: InputMaybe<IntFilter>;
  Width?: InputMaybe<IntFilter>;
};

export type PlaybackSyncIntervalQueryVariables = Exact<{
  Key: Scalars["String"]["input"];
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
    Error?: string | null;
    AppSetting?: { Id: string; Key: string; Value: string } | null;
  };
};

export type UpdateAppSettingMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateAppSettingInput;
}>;

export type UpdateAppSettingMutation = {
  UpdateAppSetting: {
    Success: boolean;
    Error?: string | null;
    AppSetting?: { Id: string; Key: string; Value: string } | null;
  };
};

export type NeedsSetupQueryVariables = Exact<{ [key: string]: never }>;

export type NeedsSetupQuery = { NeedsSetup: boolean };

export type MeQueryVariables = Exact<{ [key: string]: never }>;

export type MeQuery = {
  Me?: {
    Id: string;
    Email?: string | null;
    Username: string;
    Role: string;
    DisplayName?: string | null;
  } | null;
};

export type LoginMutationVariables = Exact<{
  input: LoginInput;
}>;

export type LoginMutation = {
  Login: {
    Success: boolean;
    Error?: string | null;
    User?: {
      Id: string;
      Email?: string | null;
      Username: string;
      Role: string;
      DisplayName?: string | null;
    } | null;
    Tokens?: {
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
    Error?: string | null;
    User?: {
      Id: string;
      Email?: string | null;
      Username: string;
      Role: string;
      DisplayName?: string | null;
    } | null;
    Tokens?: {
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
    Error?: string | null;
    Tokens?: {
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
  Logout: { Success: boolean; Error?: string | null };
};

export type CastDevicesQueryVariables = Exact<{
  Where?: InputMaybe<CastDeviceWhereInput>;
  OrderBy?: InputMaybe<Array<CastDeviceOrderByInput> | CastDeviceOrderByInput>;
  Page?: InputMaybe<PageInput>;
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
        Model?: string | null;
        DeviceType: string;
        IsFavorite: boolean;
        IsManual: boolean;
        LastSeenAt?: string | null;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount?: number | null };
  };
};

export type CastSessionsQueryVariables = Exact<{
  Where?: InputMaybe<CastSessionWhereInput>;
  OrderBy?: InputMaybe<
    Array<CastSessionOrderByInput> | CastSessionOrderByInput
  >;
  Page?: InputMaybe<PageInput>;
}>;

export type CastSessionsQuery = {
  CastSessions: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        DeviceId?: string | null;
        MediaFileId?: string | null;
        EpisodeId?: string | null;
        StreamUrl: string;
        PlayerState: string;
        CurrentPosition: number;
        Duration?: number | null;
        Volume: number;
        IsMuted: boolean;
        StartedAt: string;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount?: number | null };
  };
};

export type CastSettingsQueryVariables = Exact<{
  Where?: InputMaybe<CastSettingWhereInput>;
  OrderBy?: InputMaybe<
    Array<CastSettingOrderByInput> | CastSettingOrderByInput
  >;
  Page?: InputMaybe<PageInput>;
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
        PreferredQuality?: string | null;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount?: number | null };
  };
};

export type CreateCastDeviceMutationVariables = Exact<{
  Input: CreateCastDeviceInput;
}>;

export type CreateCastDeviceMutation = {
  CreateCastDevice: {
    Success: boolean;
    Error?: string | null;
    CastDevice?: {
      Id: string;
      Name: string;
      Address: string;
      Port: number;
      Model?: string | null;
      DeviceType: string;
      IsFavorite: boolean;
      IsManual: boolean;
      LastSeenAt?: string | null;
    } | null;
  };
};

export type UpdateCastDeviceMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateCastDeviceInput;
}>;

export type UpdateCastDeviceMutation = {
  UpdateCastDevice: {
    Success: boolean;
    Error?: string | null;
    CastDevice?: {
      Id: string;
      Name: string;
      Address: string;
      Port: number;
      Model?: string | null;
      DeviceType: string;
      IsFavorite: boolean;
      IsManual: boolean;
      LastSeenAt?: string | null;
    } | null;
  };
};

export type DeleteCastDeviceMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteCastDeviceMutation = {
  DeleteCastDevice: { Success: boolean; Error?: string | null };
};

export type CreateCastSettingMutationVariables = Exact<{
  Input: CreateCastSettingInput;
}>;

export type CreateCastSettingMutation = {
  CreateCastSetting: {
    Success: boolean;
    Error?: string | null;
    CastSetting?: {
      Id: string;
      AutoDiscoveryEnabled: boolean;
      DiscoveryIntervalSeconds: number;
      DefaultVolume: number;
      TranscodeIncompatible: boolean;
      PreferredQuality?: string | null;
    } | null;
  };
};

export type UpdateCastSettingMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateCastSettingInput;
}>;

export type UpdateCastSettingMutation = {
  UpdateCastSetting: {
    Success: boolean;
    Error?: string | null;
    CastSetting?: {
      Id: string;
      AutoDiscoveryEnabled: boolean;
      DiscoveryIntervalSeconds: number;
      DefaultVolume: number;
      TranscodeIncompatible: boolean;
      PreferredQuality?: string | null;
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
    model?: string | null;
    deviceType: string;
    isFavorite: boolean;
    isManual: boolean;
    isConnected: boolean;
    lastSeenAt?: string | null;
  }>;
};

export type CastMediaOpMutationVariables = Exact<{
  input: CastMediaInput;
}>;

export type CastMediaOpMutation = {
  CastMedia: {
    success: boolean;
    error?: string | null;
    session?: {
      id: string;
      deviceId?: string | null;
      deviceName?: string | null;
      mediaFileId?: string | null;
      episodeId?: string | null;
      streamUrl: string;
      playerState: string;
      currentTime: number;
      duration?: number | null;
      volume: number;
      isMuted: boolean;
      startedAt: string;
    } | null;
  };
};

export type CastPlayOpMutationVariables = Exact<{
  sessionId: Scalars["String"]["input"];
}>;

export type CastPlayOpMutation = {
  CastPlay: {
    success: boolean;
    error?: string | null;
    session?: { id: string; playerState: string; currentTime: number } | null;
  };
};

export type CastPauseOpMutationVariables = Exact<{
  sessionId: Scalars["String"]["input"];
}>;

export type CastPauseOpMutation = {
  CastPause: {
    success: boolean;
    error?: string | null;
    session?: { id: string; playerState: string; currentTime: number } | null;
  };
};

export type CastStopOpMutationVariables = Exact<{
  sessionId: Scalars["String"]["input"];
}>;

export type CastStopOpMutation = {
  CastStop: { success: boolean; error?: string | null };
};

export type CastSeekOpMutationVariables = Exact<{
  sessionId: Scalars["String"]["input"];
  position: Scalars["Float"]["input"];
}>;

export type CastSeekOpMutation = {
  CastSeek: {
    success: boolean;
    error?: string | null;
    session?: { id: string; playerState: string; currentTime: number } | null;
  };
};

export type CastSetVolumeOpMutationVariables = Exact<{
  sessionId: Scalars["String"]["input"];
  volume: Scalars["Float"]["input"];
}>;

export type CastSetVolumeOpMutation = {
  CastSetVolume: {
    success: boolean;
    error?: string | null;
    session?: { id: string; volume: number; isMuted: boolean } | null;
  };
};

export type CastSetMutedOpMutationVariables = Exact<{
  sessionId: Scalars["String"]["input"];
  muted: Scalars["Boolean"]["input"];
}>;

export type CastSetMutedOpMutation = {
  CastSetMuted: {
    success: boolean;
    error?: string | null;
    session?: { id: string; volume: number; isMuted: boolean } | null;
  };
};

export type DashboardShowsQueryVariables = Exact<{
  Where?: InputMaybe<ShowWhereInput>;
  Page?: InputMaybe<PageInput>;
  OrderBy?: InputMaybe<Array<ShowOrderByInput> | ShowOrderByInput>;
}>;

export type DashboardShowsQuery = {
  Shows: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        LibraryId: string;
        Name: string;
        SortName?: string | null;
        Year?: number | null;
        TvmazeId?: number | null;
        TmdbId?: number | null;
        TvdbId?: number | null;
        ImdbId?: string | null;
        Overview?: string | null;
        Network?: string | null;
        Runtime?: number | null;
        PosterUrl?: string | null;
        BackdropUrl?: string | null;
        Path?: string | null;
        Genres: Array<string>;
        CreatedAt: string;
      };
    }>;
    PageInfo: { TotalCount?: number | null };
  };
};

export type DashboardScheduleCachesQueryVariables = Exact<{
  Where?: InputMaybe<ScheduleCacheWhereInput>;
  OrderBy?: InputMaybe<
    Array<ScheduleCacheOrderByInput> | ScheduleCacheOrderByInput
  >;
  Page?: InputMaybe<PageInput>;
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
        EpisodeType?: string | null;
        AirDate: string;
        AirTime?: string | null;
        AirStamp?: string | null;
        Runtime?: number | null;
        EpisodeImageUrl?: string | null;
        Summary?: string | null;
        TvmazeShowId: number;
        ShowName: string;
        ShowNetwork?: string | null;
        ShowPosterUrl?: string | null;
        ShowGenres: Array<string>;
        CountryCode: string;
      };
    }>;
    PageInfo: { TotalCount?: number | null };
  };
};

export type MediaFilePropertiesQueryVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type MediaFilePropertiesQuery = {
  MediaFile?: {
    Id: string;
    LibraryId: string;
    Path: string;
    RelativePath?: string | null;
    OriginalName?: string | null;
    Size: number;
    Container?: string | null;
    VideoCodec?: string | null;
    AudioCodec?: string | null;
    Resolution?: string | null;
    IsHdr: boolean;
    HdrType?: string | null;
    Width?: number | null;
    Height?: number | null;
    Duration?: number | null;
    Bitrate?: number | null;
    AudioChannels?: string | null;
    EpisodeId?: string | null;
    MovieId?: string | null;
    TrackId?: string | null;
    ContentType?: string | null;
    AddedAt: string;
  } | null;
  VideoStreams: {
    Edges: Array<{
      Node: {
        Id: string;
        StreamIndex: number;
        Codec: string;
        CodecLongName?: string | null;
        Width: number;
        Height: number;
        AspectRatio?: string | null;
        FrameRate?: string | null;
        Bitrate?: number | null;
        PixelFormat?: string | null;
        HdrType?: string | null;
        BitDepth?: number | null;
        Language?: string | null;
        Title?: string | null;
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
        CodecLongName?: string | null;
        Channels: number;
        ChannelLayout?: string | null;
        SampleRate?: number | null;
        Bitrate?: number | null;
        BitDepth?: number | null;
        Language?: string | null;
        Title?: string | null;
        IsDefault: boolean;
        IsCommentary: boolean;
      };
    }>;
  };
  Subtitles: {
    Edges: Array<{
      Node: {
        Id: string;
        StreamIndex?: number | null;
        SourceType: string;
        Codec?: string | null;
        CodecLongName?: string | null;
        Language?: string | null;
        Title?: string | null;
        IsDefault: boolean;
        IsForced: boolean;
        IsHearingImpaired: boolean;
        FilePath?: string | null;
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
        Title?: string | null;
      };
    }>;
  };
};

export type MediaFileByPathLookupQueryVariables = Exact<{
  Path: Scalars["String"]["input"];
}>;

export type MediaFileByPathLookupQuery = {
  MediaFiles: { Edges: Array<{ Node: { Id: string; Path: string } }> };
};

export type MediaFileMetadataQueryVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type MediaFileMetadataQuery = {
  MediaFile?: { Id: string; Metadata?: string | null } | null;
};

export type BrowseDirectoryQueryVariables = Exact<{
  Input?: InputMaybe<BrowseDirectoryInput>;
}>;

export type BrowseDirectoryQuery = {
  BrowseDirectory: {
    CurrentPath: string;
    ParentPath?: string | null;
    IsLibraryPath: boolean;
    LibraryId?: string | null;
    Entries: Array<{
      Name: string;
      Path: string;
      IsDir: boolean;
      Size: number;
      SizeFormatted: string;
      Readable: boolean;
      Writable: boolean;
      MimeType?: string | null;
      ModifiedAt?: string | null;
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
    DefaultLinuxMountBase?: string | null;
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
    Message?: string | null;
  }>;
};

export type ConfigureNetworkPathMutationVariables = Exact<{
  Input: ConfigureNetworkPathInput;
}>;

export type ConfigureNetworkPathMutation = {
  ConfigureNetworkPath: {
    Success: boolean;
    Error?: string | null;
    ResolvedPath: string;
    Connected: boolean;
    Stored: boolean;
    Message?: string | null;
  };
};

export type ReconnectLibraryPathMutationVariables = Exact<{
  Path: Scalars["String"]["input"];
}>;

export type ReconnectLibraryPathMutation = {
  ReconnectLibraryPath: {
    Success: boolean;
    Error?: string | null;
    ResolvedPath: string;
    Connected: boolean;
    Stored: boolean;
    Message?: string | null;
  };
};

export type LibrariesQueryVariables = Exact<{
  Where?: InputMaybe<LibraryWhereInput>;
  OrderBy?: InputMaybe<Array<LibraryOrderByInput> | LibraryOrderByInput>;
  Page?: InputMaybe<PageInput>;
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
        Icon?: string | null;
        Color?: string | null;
        AutoScan: boolean;
        ScanIntervalMinutes: number;
        WatchForChanges: boolean;
        AutoOrganize: boolean;
        NamingPattern: string;
        Scanning: boolean;
        LastScannedAt?: string | null;
        CreatedAt: string;
        UpdatedAt: string;
        Shows: { PageInfo: { TotalCount?: number | null } };
        ShowArtwork: {
          Edges: Array<{ Node: { Id: string; PosterUrl?: string | null } }>;
        };
        Movies: { PageInfo: { TotalCount?: number | null } };
        MovieArtwork: {
          Edges: Array<{ Node: { Id: string; PosterUrl?: string | null } }>;
        };
        Albums: { PageInfo: { TotalCount?: number | null } };
        AlbumArtwork: {
          Edges: Array<{ Node: { Id: string; CoverUrl?: string | null } }>;
        };
        Audiobooks: { PageInfo: { TotalCount?: number | null } };
        AudiobookArtwork: {
          Edges: Array<{ Node: { Id: string; CoverUrl?: string | null } }>;
        };
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount?: number | null };
  };
};

export type LibraryChangedSubscriptionVariables = Exact<{
  Filter?: InputMaybe<SubscriptionFilterInput>;
}>;

export type LibraryChangedSubscription = {
  LibraryChanged: {
    Action: ChangeAction;
    Id: string;
    Library?: {
      Id: string;
      Name: string;
      Path: string;
      LibraryType: string;
      Icon?: string | null;
      Color?: string | null;
      AutoScan: boolean;
      ScanIntervalMinutes: number;
      WatchForChanges: boolean;
      Scanning: boolean;
      LastScannedAt?: string | null;
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
    Error?: string | null;
    Library?: {
      Id: string;
      Name: string;
      Path: string;
      LibraryType: string;
      Icon?: string | null;
      Color?: string | null;
    } | null;
  };
};

export type DeleteLibraryMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteLibraryMutation = {
  DeleteLibrary: { Success: boolean; Error?: string | null };
};

export type ScanLibraryMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type ScanLibraryMutation = {
  ScanLibrary: { Success: boolean; Status: string; Message?: string | null };
};

export type LibraryAlbumsTabQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type LibraryAlbumsTabQuery = {
  Albums: {
    Edges: Array<{
      Node: {
        Id: string;
        ArtistId: string;
        LibraryId: string;
        Name: string;
        SortName?: string | null;
        Year?: number | null;
        MusicbrainzId?: string | null;
        AlbumType?: string | null;
        Genres: Array<string>;
        Label?: string | null;
        Country?: string | null;
        ReleaseDate?: string | null;
        CoverUrl?: string | null;
        TrackCount?: number | null;
        DiscCount?: number | null;
        TotalDurationSecs?: number | null;
        HasFiles: boolean;
        SizeBytes?: number | null;
        Path?: string | null;
      };
    }>;
  };
};

export type LibraryArtistsTabQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type LibraryArtistsTabQuery = {
  Artists: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string;
        Name: string;
        SortName?: string | null;
        MusicbrainzId?: string | null;
      };
    }>;
  };
};

export type LibraryAudiobooksTabQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type LibraryAudiobooksTabQuery = {
  Audiobooks: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string;
        Title: string;
        SortTitle?: string | null;
        Isbn?: string | null;
        Description?: string | null;
        Publisher?: string | null;
        Language?: string | null;
        Narrators: Array<string>;
        CoverUrl?: string | null;
        HasFiles: boolean;
        SizeBytes?: number | null;
        Path?: string | null;
        ChapterCount?: number | null;
        TotalDurationSecs?: number | null;
        AuthorName?: string | null;
      };
    }>;
  };
};

export type LibraryUnmatchedMediaFilesTabQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type LibraryUnmatchedMediaFilesTabQuery = {
  MediaFiles: {
    Edges: Array<{
      Node: {
        Id: string;
        LibraryId: string;
        Path: string;
        RelativePath?: string | null;
        OriginalName?: string | null;
        Size: number;
        Container?: string | null;
        VideoCodec?: string | null;
        AudioCodec?: string | null;
        Resolution?: string | null;
        IsHdr: boolean;
        HdrType?: string | null;
        Width?: number | null;
        Height?: number | null;
        Duration?: number | null;
        EpisodeId?: string | null;
        AddedAt: string;
      };
    }>;
  };
};

export type LibraryDetailRouteQueryVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type LibraryDetailRouteQuery = {
  Library?: {
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
  Id: Scalars["String"]["input"];
  Input: UpdateLibraryInput;
}>;

export type UpdateLibraryRouteMutation = {
  UpdateLibrary: {
    Success: boolean;
    Error?: string | null;
    Library?: { Id: string } | null;
  };
};

export type DeleteShowRouteMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteShowRouteMutation = {
  DeleteShow: { Success: boolean; Error?: string | null };
};

export type AppLogsQueryVariables = Exact<{
  Where?: InputMaybe<AppLogWhereInput>;
  OrderBy?: InputMaybe<Array<AppLogOrderByInput> | AppLogOrderByInput>;
  Page?: InputMaybe<PageInput>;
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
        Fields?: string | null;
        SpanName?: string | null;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount?: number | null };
  };
};

export type AppLogChangedSubscriptionVariables = Exact<{
  Filter?: InputMaybe<SubscriptionFilterInput>;
}>;

export type AppLogChangedSubscription = {
  AppLogChanged: {
    Action: ChangeAction;
    Id: string;
    AppLog?: {
      Id: string;
      Timestamp: string;
      Level: string;
      Target: string;
      Message: string;
      Fields?: string | null;
      SpanName?: string | null;
    } | null;
  };
};

export type DeleteAppLogsMutationVariables = Exact<{
  Where: AppLogWhereInput;
}>;

export type DeleteAppLogsMutation = {
  DeleteAppLogs: {
    success: boolean;
    error?: string | null;
    DeletedCount: number;
  };
};

export type ManualMatchShowsByLibraryQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type ManualMatchShowsByLibraryQuery = {
  Shows: {
    Edges: Array<{
      Node: {
        Id: string;
        Name: string;
        Year?: number | null;
        Episodes: {
          Edges: Array<{
            Node: {
              Id: string;
              Season: number;
              Episode: number;
              Title?: string | null;
            };
          }>;
        };
      };
    }>;
  };
};

export type ManualMatchMoviesByLibraryQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type ManualMatchMoviesByLibraryQuery = {
  Movies: {
    Edges: Array<{ Node: { Id: string; Title: string; Year?: number | null } }>;
  };
};

export type ManualMatchAlbumsByLibraryQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type ManualMatchAlbumsByLibraryQuery = {
  Albums: {
    Edges: Array<{ Node: { Id: string; Name: string; Year?: number | null } }>;
  };
  Tracks: {
    Edges: Array<{
      Node: {
        Id: string;
        AlbumId: string;
        ArtistName?: string | null;
        TrackNumber: number;
        Title: string;
      };
    }>;
  };
};

export type ManualMatchAudiobooksByLibraryQueryVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
}>;

export type ManualMatchAudiobooksByLibraryQuery = {
  Audiobooks: {
    Edges: Array<{
      Node: {
        Id: string;
        Title: string;
        AuthorName?: string | null;
        Chapters: {
          Edges: Array<{
            Node: { Id: string; ChapterNumber: number; Title?: string | null };
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
    MatchedId?: string | null;
    MatchedType?: string | null;
    Reason?: string | null;
  };
};

export type SearchAlbumsQueryVariables = Exact<{
  Query: Scalars["String"]["input"];
  IncludeEps?: InputMaybe<Scalars["Boolean"]["input"]>;
  IncludeSingles?: InputMaybe<Scalars["Boolean"]["input"]>;
  IncludeCompilations?: InputMaybe<Scalars["Boolean"]["input"]>;
  IncludeLive?: InputMaybe<Scalars["Boolean"]["input"]>;
  IncludeSoundtracks?: InputMaybe<Scalars["Boolean"]["input"]>;
}>;

export type SearchAlbumsQuery = {
  SearchAlbums: Array<{
    Provider: string;
    ProviderId: string;
    Title: string;
    ArtistName?: string | null;
    Year?: number | null;
    AlbumType?: string | null;
    CoverUrl?: string | null;
    Score?: number | null;
  }>;
};

export type SearchAudiobooksQueryVariables = Exact<{
  Query: Scalars["String"]["input"];
}>;

export type SearchAudiobooksQuery = {
  SearchAudiobooks: Array<{
    Provider: string;
    ProviderId: string;
    Title: string;
    AuthorName?: string | null;
    Year?: number | null;
    CoverUrl?: string | null;
    Isbn?: string | null;
    Description?: string | null;
  }>;
};

export type AddAlbumMutationVariables = Exact<{
  Input: AddAlbumInput;
}>;

export type AddAlbumMutation = {
  AddAlbum: { Success: boolean; Error?: string | null };
};

export type AddAudiobookMutationVariables = Exact<{
  Input: AddAudiobookInput;
}>;

export type AddAudiobookMutation = {
  AddAudiobook: { Success: boolean; Error?: string | null };
};

export type AddTorrentMutationVariables = Exact<{
  Input: AddTorrentInput;
}>;

export type AddTorrentMutation = {
  AddTorrent: {
    Success: boolean;
    Error?: string | null;
    Torrent?: { Id: number; Name: string } | null;
  };
};

export type AlbumDetailRouteQueryVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type AlbumDetailRouteQuery = {
  Album?: {
    Id: string;
    ArtistId: string;
    LibraryId: string;
    Name: string;
    SortName?: string | null;
    Year?: number | null;
    MusicbrainzId?: string | null;
    AlbumType?: string | null;
    Genres: Array<string>;
    Label?: string | null;
    Country?: string | null;
    ReleaseDate?: string | null;
    CoverUrl?: string | null;
    TrackCount?: number | null;
    DiscCount?: number | null;
    TotalDurationSecs?: number | null;
    HasFiles: boolean;
    SizeBytes?: number | null;
    Path?: string | null;
  } | null;
  Tracks: {
    Edges: Array<{
      Node: {
        Id: string;
        AlbumId: string;
        LibraryId: string;
        Title: string;
        TrackNumber: number;
        DiscNumber?: number | null;
        MusicbrainzId?: string | null;
        Isrc?: string | null;
        DurationSecs?: number | null;
        Explicit: boolean;
        ArtistName?: string | null;
        ArtistId?: string | null;
        MediaFileId?: string | null;
        Status: ContentStatus;
      };
    }>;
  };
};

export type DeleteAlbumRouteMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteAlbumRouteMutation = {
  DeleteAlbum: { Success: boolean; Error?: string | null };
};

export type AudiobookDetailRouteQueryVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type AudiobookDetailRouteQuery = {
  Audiobook?: {
    Id: string;
    LibraryId: string;
    Title: string;
    SortTitle?: string | null;
    Isbn?: string | null;
    Description?: string | null;
    Publisher?: string | null;
    Language?: string | null;
    Narrators: Array<string>;
    TotalDurationSecs?: number | null;
    CoverUrl?: string | null;
    HasFiles: boolean;
    SizeBytes?: number | null;
    Path?: string | null;
    Chapters: {
      Edges: Array<{
        Node: {
          Id: string;
          AudiobookId: string;
          ChapterNumber: number;
          Title?: string | null;
          StartTimeSecs: number;
          EndTimeSecs?: number | null;
          DurationSecs?: number | null;
          MediaFileId?: string | null;
          Status: ContentStatus;
        };
      }>;
    };
  } | null;
};

export type DeleteAudiobookRouteMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteAudiobookRouteMutation = {
  DeleteAudiobook: { Success: boolean; Error?: string | null };
};

export type SearchMoviesQueryVariables = Exact<{
  Query: Scalars["String"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
}>;

export type SearchMoviesQuery = {
  SearchMovies: Array<{
    Provider: string;
    ProviderId: number;
    Title: string;
    OriginalTitle?: string | null;
    Year?: number | null;
    Overview?: string | null;
    PosterUrl?: string | null;
    BackdropUrl?: string | null;
    ImdbId?: string | null;
    VoteAverage?: number | null;
    Popularity?: number | null;
  }>;
};

export type AddMovieMutationVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
  Input: AddMovieInput;
}>;

export type AddMovieMutation = {
  AddMovie: {
    Success: boolean;
    Error?: string | null;
    Movie?: {
      Id: string;
      LibraryId: string;
      Title: string;
      Year?: number | null;
      TmdbId?: number | null;
      ImdbId?: string | null;
      Overview?: string | null;
      PosterUrl?: string | null;
      BackdropUrl?: string | null;
      Monitored: boolean;
      MediaFileId?: string | null;
    } | null;
  };
};

export type MovieChangedSubscriptionVariables = Exact<{
  Filter?: InputMaybe<SubscriptionFilterInput>;
}>;

export type MovieChangedSubscription = {
  MovieChanged: {
    Action: ChangeAction;
    Id: string;
    Movie?: { LibraryId: string } | null;
  };
};

export type MovieDetailRouteQueryVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type MovieDetailRouteQuery = {
  Movie?: {
    Id: string;
    LibraryId: string;
    Title: string;
    SortTitle?: string | null;
    OriginalTitle?: string | null;
    Year?: number | null;
    TmdbId?: number | null;
    ImdbId?: string | null;
    Status: ContentStatus;
    Overview?: string | null;
    Tagline?: string | null;
    Runtime?: number | null;
    Genres: Array<string>;
    Director?: string | null;
    CastNames: Array<string>;
    PosterUrl?: string | null;
    BackdropUrl?: string | null;
    Monitored: boolean;
    MediaFileId?: string | null;
    CollectionId?: number | null;
    CollectionName?: string | null;
    CollectionPosterUrl?: string | null;
    TmdbRating?: string | null;
    TmdbVoteCount?: number | null;
    Certification?: string | null;
    ReleaseDate?: string | null;
    ProductionCountries: Array<string>;
    SpokenLanguages: Array<string>;
    MediaFile?: { Id: string; Size: number; Duration?: number | null } | null;
  } | null;
};

export type RefreshMovieRouteMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type RefreshMovieRouteMutation = {
  RefreshMovie: {
    Success: boolean;
    Error?: string | null;
    Movie?: {
      Id: string;
      Title: string;
      Overview?: string | null;
      Tagline?: string | null;
      PosterUrl?: string | null;
      BackdropUrl?: string | null;
      TmdbRating?: string | null;
      TmdbVoteCount?: number | null;
    } | null;
  };
};

export type DeleteMovieModalMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteMovieModalMutation = {
  DeleteMovie: { Success: boolean; Error?: string | null };
};

export type OrganizationNamingPatternsQueryVariables = Exact<{
  OrderBy?: InputMaybe<
    Array<NamingPatternOrderByInput> | NamingPatternOrderByInput
  >;
  Page?: InputMaybe<PageInput>;
}>;

export type OrganizationNamingPatternsQuery = {
  NamingPatterns: {
    Edges: Array<{
      Node: {
        Id: string;
        Name: string;
        Pattern: string;
        Description?: string | null;
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
    Error?: string | null;
    NamingPattern?: {
      Id: string;
      Name: string;
      Pattern: string;
      Description?: string | null;
      LibraryType: string;
      IsDefault: boolean;
      IsSystem: boolean;
    } | null;
  };
};

export type OrganizationUpdateNamingPatternMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateNamingPatternInput;
}>;

export type OrganizationUpdateNamingPatternMutation = {
  UpdateNamingPattern: {
    Success: boolean;
    Error?: string | null;
    NamingPattern?: {
      Id: string;
      Name: string;
      Pattern: string;
      Description?: string | null;
      LibraryType: string;
      IsDefault: boolean;
      IsSystem: boolean;
    } | null;
  };
};

export type OrganizationDeleteNamingPatternMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type OrganizationDeleteNamingPatternMutation = {
  DeleteNamingPattern: { Success: boolean; Error?: string | null };
};

export type NotificationsQueryVariables = Exact<{
  Where?: InputMaybe<NotificationWhereInput>;
  OrderBy?: InputMaybe<
    Array<NotificationOrderByInput> | NotificationOrderByInput
  >;
  Page?: InputMaybe<PageInput>;
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
        LibraryId?: string | null;
        TorrentId?: string | null;
        MediaFileId?: string | null;
        PendingMatchId?: string | null;
        ActionType?: string | null;
        ActionData?: string | null;
        ReadAt?: string | null;
        ResolvedAt?: string | null;
        Resolution?: string | null;
        CreatedAt: string;
        UpdatedAt: string;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount?: number | null };
  };
};

export type NotificationChangedSubscriptionVariables = Exact<{
  Filter?: InputMaybe<SubscriptionFilterInput>;
}>;

export type NotificationChangedSubscription = {
  NotificationChanged: {
    Action: ChangeAction;
    Id: string;
    Notification?: {
      Id: string;
      ReadAt?: string | null;
      ResolvedAt?: string | null;
      Resolution?: string | null;
    } | null;
  };
};

export type UpdateNotificationMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateNotificationInput;
}>;

export type UpdateNotificationMutation = {
  UpdateNotification: {
    Success: boolean;
    Error?: string | null;
    Notification?: {
      Id: string;
      ReadAt?: string | null;
      ResolvedAt?: string | null;
      Resolution?: string | null;
    } | null;
  };
};

export type DeleteNotificationMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteNotificationMutation = {
  DeleteNotification: { Success: boolean; Error?: string | null };
};

export type PlaybackSessionsQueryVariables = Exact<{
  Where?: InputMaybe<PlaybackSessionWhereInput>;
  OrderBy?: InputMaybe<
    Array<PlaybackSessionOrderByInput> | PlaybackSessionOrderByInput
  >;
  Page?: InputMaybe<PageInput>;
}>;

export type PlaybackSessionsQuery = {
  PlaybackSessions: {
    Edges: Array<{
      Cursor: string;
      Node: {
        Id: string;
        UserId: string;
        MediaFileId?: string | null;
        CurrentPosition: number;
        Duration?: number | null;
        Volume: number;
        IsMuted: boolean;
        IsPlaying: boolean;
        StartedAt: string;
        LastUpdatedAt: string;
        CompletedAt?: string | null;
        CreatedAt: string;
        UpdatedAt: string;
      };
    }>;
    PageInfo: { HasNextPage: boolean; TotalCount?: number | null };
  };
};

export type ShowPlaybackProgressByMediaQueryVariables = Exact<{
  Where?: InputMaybe<PlaybackProgressWhereInput>;
  Page?: InputMaybe<PageInput>;
  OrderBy?: InputMaybe<
    Array<PlaybackProgressOrderByInput> | PlaybackProgressOrderByInput
  >;
}>;

export type ShowPlaybackProgressByMediaQuery = {
  PlaybackProgresses: {
    Edges: Array<{
      Node: {
        Id: string;
        MediaFileId?: string | null;
        CurrentPosition: number;
        Duration?: number | null;
        ProgressPercent: number;
        IsWatched: boolean;
        UpdatedAt: string;
      };
    }>;
  };
};

export type PlaybackProgressByMediaFileContextQueryVariables = Exact<{
  Where?: InputMaybe<PlaybackProgressWhereInput>;
  Page?: InputMaybe<PageInput>;
  OrderBy?: InputMaybe<
    Array<PlaybackProgressOrderByInput> | PlaybackProgressOrderByInput
  >;
}>;

export type PlaybackProgressByMediaFileContextQuery = {
  PlaybackProgresses: {
    Edges: Array<{
      Node: {
        Id: string;
        UserId: string;
        MediaFileId?: string | null;
        CurrentPosition: number;
        Duration?: number | null;
        ProgressPercent: number;
        IsWatched: boolean;
        WatchedAt?: string | null;
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
    Error?: string | null;
    PlaybackSession?: {
      Id: string;
      UserId: string;
      ContentType?: string | null;
      MediaFileId?: string | null;
      EpisodeId?: string | null;
      MovieId?: string | null;
      TrackId?: string | null;
      AudiobookId?: string | null;
      TvShowId?: string | null;
      AlbumId?: string | null;
      CurrentPosition: number;
      Duration?: number | null;
      Volume: number;
      IsMuted: boolean;
      IsPlaying: boolean;
      StartedAt: string;
      LastUpdatedAt: string;
      CompletedAt?: string | null;
      CreatedAt: string;
      UpdatedAt: string;
    } | null;
  };
};

export type UpdatePlaybackSessionContextMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdatePlaybackSessionInput;
}>;

export type UpdatePlaybackSessionContextMutation = {
  UpdatePlaybackSession: {
    Success: boolean;
    Error?: string | null;
    PlaybackSession?: {
      Id: string;
      UserId: string;
      ContentType?: string | null;
      MediaFileId?: string | null;
      EpisodeId?: string | null;
      MovieId?: string | null;
      TrackId?: string | null;
      AudiobookId?: string | null;
      TvShowId?: string | null;
      AlbumId?: string | null;
      CurrentPosition: number;
      Duration?: number | null;
      Volume: number;
      IsMuted: boolean;
      IsPlaying: boolean;
      StartedAt: string;
      LastUpdatedAt: string;
      CompletedAt?: string | null;
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
    Error?: string | null;
    PlaybackProgress?: {
      Id: string;
      UserId: string;
      MediaFileId?: string | null;
      CurrentPosition: number;
      Duration?: number | null;
      ProgressPercent: number;
      IsWatched: boolean;
      WatchedAt?: string | null;
      CreatedAt: string;
      UpdatedAt: string;
    } | null;
  };
};

export type UpdatePlaybackProgressContextMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdatePlaybackProgressInput;
}>;

export type UpdatePlaybackProgressContextMutation = {
  UpdatePlaybackProgress: {
    Success: boolean;
    Error?: string | null;
    PlaybackProgress?: {
      Id: string;
      UserId: string;
      MediaFileId?: string | null;
      CurrentPosition: number;
      Duration?: number | null;
      ProgressPercent: number;
      IsWatched: boolean;
      WatchedAt?: string | null;
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
        Year?: number | null;
        PosterUrl?: string | null;
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
        Year?: number | null;
        PosterUrl?: string | null;
        Status: ContentStatus;
      };
    }>;
  };
};

export type SearchTvShowsQueryVariables = Exact<{
  Query: Scalars["String"]["input"];
}>;

export type SearchTvShowsQuery = {
  SearchTvShows: Array<{
    Provider: string;
    ProviderId: number;
    Name: string;
    Year?: number | null;
    Status?: string | null;
    Network?: string | null;
    Overview?: string | null;
    PosterUrl?: string | null;
    TvdbId?: number | null;
    ImdbId?: string | null;
    Score?: number | null;
  }>;
};

export type AddTvShowMutationVariables = Exact<{
  LibraryId: Scalars["String"]["input"];
  Input: AddTvShowInput;
}>;

export type AddTvShowMutation = {
  AddTvShow: {
    Success: boolean;
    Error?: string | null;
    Show?: { Id: string; Name: string; PosterUrl?: string | null } | null;
  };
};

export type ShowChangedSubscriptionVariables = Exact<{
  Filter?: InputMaybe<SubscriptionFilterInput>;
}>;

export type ShowChangedSubscription = {
  ShowChanged: {
    Action: ChangeAction;
    Id: string;
    Show?: { LibraryId: string } | null;
  };
};

export type ShowDetailRouteQueryVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type ShowDetailRouteQuery = {
  Show?: {
    Id: string;
    LibraryId: string;
    Name: string;
    SortName?: string | null;
    Year?: number | null;
    TvmazeId?: number | null;
    TmdbId?: number | null;
    TvdbId?: number | null;
    ImdbId?: string | null;
    Overview?: string | null;
    Network?: string | null;
    Runtime?: number | null;
    Genres: Array<string>;
    PosterUrl?: string | null;
    BackdropUrl?: string | null;
    AutoDownload: boolean;
    AutoDownloadMode: AutoDownloadMode;
    Path?: string | null;
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
          AbsoluteNumber?: number | null;
          Title?: string | null;
          Overview?: string | null;
          AirDate?: string | null;
          Runtime?: number | null;
          TvmazeId?: number | null;
          TmdbId?: number | null;
          TvdbId?: number | null;
          MediaFileId?: string | null;
          Wanted: boolean;
          Status: ContentStatus;
          CreatedAt: string;
          UpdatedAt: string;
          MediaFile?: {
            Id: string;
            Size: number;
            Duration?: number | null;
            Resolution?: string | null;
            VideoCodec?: string | null;
            AudioCodec?: string | null;
            AudioChannels?: string | null;
            IsHdr: boolean;
            HdrType?: string | null;
          } | null;
        };
      }>;
    };
  } | null;
};

export type RefreshShowRouteMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type RefreshShowRouteMutation = {
  RefreshShow: {
    Success: boolean;
    Error?: string | null;
    Show?: {
      Id: string;
      Name: string;
      Overview?: string | null;
      PosterUrl?: string | null;
      BackdropUrl?: string | null;
    } | null;
  };
};

export type SourcesQueryVariables = Exact<{
  Where?: InputMaybe<SourceWhereInput>;
  OrderBy?: InputMaybe<Array<SourceOrderByInput> | SourceOrderByInput>;
  Page?: InputMaybe<PageInput>;
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
        SiteUrl?: string | null;
        SupportsSearch: boolean;
        SupportsTvSearch: boolean;
        SupportsMovieSearch: boolean;
        SupportsMusicSearch: boolean;
        SupportsBookSearch: boolean;
        Settings?: string | null;
        LastError?: string | null;
        ErrorCount: number;
        LastSuccessAt?: string | null;
        LastErrorAt?: string | null;
        CreatedAt: string;
        UpdatedAt: string;
      };
    }>;
    PageInfo: {
      TotalCount?: number | null;
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
  DefinitionId: Scalars["String"]["input"];
}>;

export type SourceSettingDefinitionsQuery = {
  SourceSettingDefinitions: Array<{
    Key: string;
    Label: string;
    SettingType: string;
    DefaultValue?: string | null;
    Options?: Array<{ Value: string; Label: string }> | null;
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
      Error?: string | null;
      Releases: Array<{
        Title: string;
        Guid: string;
        Link?: string | null;
        MagnetUri?: string | null;
        InfoHash?: string | null;
        Details?: string | null;
        PublishDate: string;
        Categories: Array<number>;
        Size?: number | null;
        SizeFormatted?: string | null;
        Seeders?: number | null;
        Leechers?: number | null;
        Peers?: number | null;
        Grabs?: number | null;
        IsFreeleech: boolean;
        ImdbId?: string | null;
        Poster?: string | null;
        Description?: string | null;
        SourceId?: string | null;
        SourceName?: string | null;
      }>;
    }>;
  };
};

export type CreateSourceMutationVariables = Exact<{
  Input: CreateSourceInput;
}>;

export type CreateSourceMutation = {
  CreateSource: { Success: boolean; Error?: string | null };
};

export type UpdateSourceMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateSourceInput;
}>;

export type UpdateSourceMutation = {
  UpdateSource: { Success: boolean; Error?: string | null };
};

export type DeleteSourceMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type DeleteSourceMutation = {
  DeleteSource: { Success: boolean; Error?: string | null };
};

export type TestSourceMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type TestSourceMutation = {
  TestSource: {
    Success: boolean;
    Error?: string | null;
    ReleasesFound?: number | null;
    ElapsedMs?: number | null;
  };
};

export type UpdateSourcePrioritiesMutationVariables = Exact<{
  Input: UpdateSourcePrioritiesInput;
}>;

export type UpdateSourcePrioritiesMutation = {
  UpdateSourcePriorities: { Success: boolean; Error?: string | null };
};

export type ActiveDownloadCountQueryVariables = Exact<{
  Where?: InputMaybe<TorrentWhereInput>;
  Page?: InputMaybe<PageInput>;
}>;

export type ActiveDownloadCountQuery = {
  Torrents: { PageInfo: { TotalCount?: number | null } };
};

export type DownloadsTorrentsQueryVariables = Exact<{
  Where?: InputMaybe<TorrentWhereInput>;
  Page?: InputMaybe<PageInput>;
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
    PageInfo: { TotalCount?: number | null; HasNextPage: boolean };
  };
};

export type TorrentByInfoHashWithFilesQueryVariables = Exact<{
  Where?: InputMaybe<TorrentWhereInput>;
  Page?: InputMaybe<PageInput>;
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
  Where?: InputMaybe<PendingFileMatchWhereInput>;
  Page?: InputMaybe<PageInput>;
}>;

export type PendingFileMatchesBySourceQuery = {
  PendingFileMatches: {
    Edges: Array<{
      Node: {
        Id: string;
        SourceType: string;
        SourceId?: string | null;
        SourceFileIndex?: number | null;
        SourcePath: string;
        FileSize: number;
        EpisodeId?: string | null;
        MovieId?: string | null;
        TrackId?: string | null;
        ChapterId?: string | null;
        MatchType?: string | null;
        MatchConfidence?: number | null;
        ParsedResolution?: string | null;
        ParsedCodec?: string | null;
        ParsedSource?: string | null;
        ParsedAudio?: string | null;
        CopiedAt?: string | null;
        CopyError?: string | null;
      };
    }>;
  };
};

export type PauseTorrentByInfoHashMutationVariables = Exact<{
  InfoHash: Scalars["String"]["input"];
}>;

export type PauseTorrentByInfoHashMutation = {
  PauseTorrentByInfoHash: { Success: boolean; Error?: string | null };
};

export type ResumeTorrentByInfoHashMutationVariables = Exact<{
  InfoHash: Scalars["String"]["input"];
}>;

export type ResumeTorrentByInfoHashMutation = {
  ResumeTorrentByInfoHash: { Success: boolean; Error?: string | null };
};

export type RemoveTorrentByInfoHashMutationVariables = Exact<{
  InfoHash: Scalars["String"]["input"];
  DeleteFiles?: InputMaybe<Scalars["Boolean"]["input"]>;
}>;

export type RemoveTorrentByInfoHashMutation = {
  RemoveTorrentByInfoHash: { Success: boolean; Error?: string | null };
};

export type ProcessSourceMutationVariables = Exact<{
  SourceType: Scalars["String"]["input"];
  SourceId: Scalars["String"]["input"];
}>;

export type ProcessSourceMutation = {
  ProcessSource: {
    Success: boolean;
    FilesProcessed: number;
    FilesFailed: number;
    Messages: Array<string>;
    Error?: string | null;
  };
};

export type RematchSourceMutationVariables = Exact<{
  SourceType: Scalars["String"]["input"];
  SourceId: Scalars["String"]["input"];
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
}>;

export type RematchSourceMutation = {
  RematchSource: {
    Success: boolean;
    MatchCount: number;
    Error?: string | null;
  };
};

export type LinkTorrentToLibraryMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateTorrentInput;
}>;

export type LinkTorrentToLibraryMutation = {
  UpdateTorrent: {
    Success: boolean;
    Error?: string | null;
    Torrent?: { Id: string; LibraryId?: string | null } | null;
  };
};

export type TorrentChangedSubscriptionVariables = Exact<{
  [key: string]: never;
}>;

export type TorrentChangedSubscription = { TorrentChanged: { Id: number } };

export type SettingsUsenetServersQueryVariables = Exact<{
  OrderBy?: InputMaybe<
    Array<UsenetServerOrderByInput> | UsenetServerOrderByInput
  >;
  Page?: InputMaybe<PageInput>;
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
        Username?: string | null;
        Connections: number;
        Priority: number;
        Enabled: boolean;
        RetentionDays?: number | null;
        LastSuccessAt?: string | null;
        LastError?: string | null;
        ErrorCount: number;
      };
    }>;
  };
};

export type SettingsUpdateUsenetServerMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
  Input: UpdateUsenetServerInput;
}>;

export type SettingsUpdateUsenetServerMutation = {
  UpdateUsenetServer: {
    Success: boolean;
    Error?: string | null;
    UsenetServer?: { Id: string; Enabled: boolean; Priority: number } | null;
  };
};

export type SettingsCreateUsenetServerMutationVariables = Exact<{
  Input: CreateUsenetServerInput;
}>;

export type SettingsCreateUsenetServerMutation = {
  CreateUsenetServer: {
    Success: boolean;
    Error?: string | null;
    UsenetServer?: {
      Id: string;
      Name: string;
      Host: string;
      Port: number;
      UseSsl: boolean;
      Username?: string | null;
      Connections: number;
      Priority: number;
      Enabled: boolean;
      RetentionDays?: number | null;
      LastSuccessAt?: string | null;
      LastError?: string | null;
      ErrorCount: number;
    } | null;
  };
};

export type SettingsDeleteUsenetServerMutationVariables = Exact<{
  Id: Scalars["String"]["input"];
}>;

export type SettingsDeleteUsenetServerMutation = {
  DeleteUsenetServer: { Success: boolean; Error?: string | null };
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
            name: { kind: "Name", value: "AppSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "1" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "AppSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "20" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "AppSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "50" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "AppSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "100" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "CreateAppSetting" },
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
                  name: { kind: "Name", value: "AppSetting" },
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
            name: { kind: "Name", value: "UpdateAppSetting" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "AppSetting" },
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
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Email" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Username" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Role" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "DisplayName" },
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
                        name: { kind: "Name", value: "AccessToken" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "RefreshToken" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ExpiresIn" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TokenType" },
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
                      { kind: "Field", name: { kind: "Name", value: "Id" } },
                      { kind: "Field", name: { kind: "Name", value: "Email" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Username" },
                      },
                      { kind: "Field", name: { kind: "Name", value: "Role" } },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "DisplayName" },
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
                        name: { kind: "Name", value: "AccessToken" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "RefreshToken" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ExpiresIn" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TokenType" },
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
                        name: { kind: "Name", value: "AccessToken" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "RefreshToken" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "ExpiresIn" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TokenType" },
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
            name: { kind: "Name", value: "CastDevices" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "CastSessions" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "CastSettings" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "CreateCastDevice" },
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
                  name: { kind: "Name", value: "CastDevice" },
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
            name: { kind: "Name", value: "UpdateCastDevice" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CastDevice" },
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
            name: { kind: "Name", value: "DeleteCastDevice" },
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
            name: { kind: "Name", value: "CreateCastSetting" },
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
                  name: { kind: "Name", value: "CastSetting" },
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
            name: { kind: "Name", value: "UpdateCastSetting" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "CastSetting" },
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
            name: { kind: "Name", value: "Shows" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "ScheduleCaches" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "MediaFile" },
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
            name: { kind: "Name", value: "VideoStreams" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "StreamIndex" },
                          value: { kind: "EnumValue", value: "Asc" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "200" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "AudioStreams" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "StreamIndex" },
                          value: { kind: "EnumValue", value: "Asc" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "200" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Subtitles" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "CreatedAt" },
                          value: { kind: "EnumValue", value: "Asc" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "200" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "MediaChapters" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "ChapterIndex" },
                          value: { kind: "EnumValue", value: "Asc" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "MediaFiles" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "1" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "MediaFile" },
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
            name: { kind: "Name", value: "Libraries" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Shows" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "PageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "TotalCount",
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
                              name: { kind: "Name", value: "Shows" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "OrderBy" },
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
                                          value: "Desc",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "Page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Offset" },
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
                                    name: { kind: "Name", value: "Edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Movies" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "PageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "TotalCount",
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
                              name: { kind: "Name", value: "Movies" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "OrderBy" },
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
                                          value: "Desc",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "Page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Offset" },
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
                                    name: { kind: "Name", value: "Edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Albums" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "PageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "TotalCount",
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
                              name: { kind: "Name", value: "Albums" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "OrderBy" },
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
                                          value: "Desc",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "Page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Offset" },
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
                                    name: { kind: "Name", value: "Edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Audiobooks" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "PageInfo" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: {
                                            kind: "Name",
                                            value: "TotalCount",
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
                              name: { kind: "Name", value: "Audiobooks" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "OrderBy" },
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
                                          value: "Desc",
                                        },
                                      },
                                    ],
                                  },
                                },
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "Page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Limit" },
                                        value: { kind: "IntValue", value: "8" },
                                      },
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Offset" },
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
                                    name: { kind: "Name", value: "Edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "LibraryChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Action" } },
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Library" },
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
            name: { kind: "Name", value: "CreateLibrary" },
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
                  name: { kind: "Name", value: "Library" },
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
            name: { kind: "Name", value: "DeleteLibrary" },
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
                { kind: "Field", name: { kind: "Name", value: "Status" } },
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
            name: { kind: "Name", value: "Albums" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Artists" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Audiobooks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "MediaFiles" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                            name: { kind: "Name", value: "IsNull" },
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
                            name: { kind: "Name", value: "IsNull" },
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
                            name: { kind: "Name", value: "IsNull" },
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
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "AddedAt" },
                          value: { kind: "EnumValue", value: "Desc" },
                        },
                      ],
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "2000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Library" },
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
            name: { kind: "Name", value: "UpdateLibrary" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Library" },
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
            name: { kind: "Name", value: "DeleteShow" },
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
            name: { kind: "Name", value: "AppLogs" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "AppLogChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Action" } },
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "AppLog" },
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
            name: { kind: "Name", value: "DeleteAppLogs" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                  name: { kind: "Name", value: "DeletedCount" },
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
            name: { kind: "Name", value: "Shows" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Episodes" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Movies" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Albums" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Tracks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "5000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Audiobooks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "500" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Chapters" },
                              selectionSet: {
                                kind: "SelectionSet",
                                selections: [
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "Edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Album" },
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
            name: { kind: "Name", value: "Tracks" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
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
                            name: { kind: "Name", value: "Eq" },
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
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "ObjectValue",
                  fields: [
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Limit" },
                      value: { kind: "IntValue", value: "5000" },
                    },
                    {
                      kind: "ObjectField",
                      name: { kind: "Name", value: "Offset" },
                      value: { kind: "IntValue", value: "0" },
                    },
                  ],
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "ListValue",
                  values: [
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "DiscNumber" },
                          value: { kind: "EnumValue", value: "Asc" },
                        },
                      ],
                    },
                    {
                      kind: "ObjectValue",
                      fields: [
                        {
                          kind: "ObjectField",
                          name: { kind: "Name", value: "TrackNumber" },
                          value: { kind: "EnumValue", value: "Asc" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Status" },
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
            name: { kind: "Name", value: "DeleteAlbum" },
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
            name: { kind: "Name", value: "Audiobook" },
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
                  name: { kind: "Name", value: "Chapters" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "Page" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "Limit" },
                            value: { kind: "IntValue", value: "5000" },
                          },
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "Offset" },
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
                        name: { kind: "Name", value: "Edges" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Node" },
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
                                    name: { kind: "Name", value: "Status" },
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
            name: { kind: "Name", value: "DeleteAudiobook" },
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
                        name: { kind: "Name", value: "PosterUrl" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "BackdropUrl" },
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
            name: { kind: "Name", value: "MovieChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Action" } },
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Movie" },
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
            name: { kind: "Name", value: "Movie" },
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
                { kind: "Field", name: { kind: "Name", value: "Status" } },
                { kind: "Field", name: { kind: "Name", value: "Overview" } },
                { kind: "Field", name: { kind: "Name", value: "Tagline" } },
                { kind: "Field", name: { kind: "Name", value: "Runtime" } },
                { kind: "Field", name: { kind: "Name", value: "Genres" } },
                { kind: "Field", name: { kind: "Name", value: "Director" } },
                { kind: "Field", name: { kind: "Name", value: "CastNames" } },
                { kind: "Field", name: { kind: "Name", value: "PosterUrl" } },
                { kind: "Field", name: { kind: "Name", value: "BackdropUrl" } },
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
                {
                  kind: "Field",
                  name: { kind: "Name", value: "MediaFile" },
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
                        name: { kind: "Name", value: "PosterUrl" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "BackdropUrl" },
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
            name: { kind: "Name", value: "DeleteMovie" },
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
            name: { kind: "Name", value: "NamingPatterns" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "CreateNamingPattern" },
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
                  name: { kind: "Name", value: "NamingPattern" },
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
            name: { kind: "Name", value: "UpdateNamingPattern" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "NamingPattern" },
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
            name: { kind: "Name", value: "DeleteNamingPattern" },
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
            name: { kind: "Name", value: "Notifications" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "NotificationChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Action" } },
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Notification" },
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
            name: { kind: "Name", value: "UpdateNotification" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Notification" },
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
            name: { kind: "Name", value: "DeleteNotification" },
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
            name: { kind: "Name", value: "PlaybackSessions" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                        name: { kind: "Name", value: "Cursor" },
                      },
                    ],
                  },
                },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
            name: { kind: "Name", value: "PlaybackProgresses" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "PlaybackProgresses" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Page" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "CreatePlaybackSession" },
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
                  name: { kind: "Name", value: "PlaybackSession" },
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
            name: { kind: "Name", value: "UpdatePlaybackSession" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PlaybackSession" },
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
            name: { kind: "Name", value: "CreatePlaybackProgress" },
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
                  name: { kind: "Name", value: "PlaybackProgress" },
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
            name: { kind: "Name", value: "UpdatePlaybackProgress" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "PlaybackProgress" },
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
            name: { kind: "Name", value: "Shows" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "Movies" },
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "PosterUrl" },
                            },
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Status" },
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
                { kind: "Field", name: { kind: "Name", value: "Status" } },
                { kind: "Field", name: { kind: "Name", value: "Network" } },
                { kind: "Field", name: { kind: "Name", value: "Overview" } },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "PosterUrl" },
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
            name: { kind: "Name", value: "ShowChanged" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Filter" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Filter" },
                },
              },
            ],
            selectionSet: {
              kind: "SelectionSet",
              selections: [
                { kind: "Field", name: { kind: "Name", value: "Action" } },
                { kind: "Field", name: { kind: "Name", value: "Id" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Show" },
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
            name: { kind: "Name", value: "Show" },
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
                { kind: "Field", name: { kind: "Name", value: "Runtime" } },
                { kind: "Field", name: { kind: "Name", value: "Genres" } },
                { kind: "Field", name: { kind: "Name", value: "PosterUrl" } },
                { kind: "Field", name: { kind: "Name", value: "BackdropUrl" } },
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
                  name: { kind: "Name", value: "Episodes" },
                  arguments: [
                    {
                      kind: "Argument",
                      name: { kind: "Name", value: "Page" },
                      value: {
                        kind: "ObjectValue",
                        fields: [
                          {
                            kind: "ObjectField",
                            name: { kind: "Name", value: "Limit" },
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
                        name: { kind: "Name", value: "Edges" },
                        selectionSet: {
                          kind: "SelectionSet",
                          selections: [
                            {
                              kind: "Field",
                              name: { kind: "Name", value: "Node" },
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
                                    name: { kind: "Name", value: "Status" },
                                  },
                                  {
                                    kind: "Field",
                                    name: { kind: "Name", value: "MediaFile" },
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
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "PosterUrl" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "BackdropUrl" },
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
            name: { kind: "Name", value: "Sources" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasPreviousPage" },
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
            name: { kind: "Name", value: "CreateSource" },
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
            name: { kind: "Name", value: "UpdateSource" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
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
            name: { kind: "Name", value: "DeleteSource" },
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
            name: { kind: "Name", value: "Torrents" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
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
  ActiveDownloadCountQuery,
  ActiveDownloadCountQueryVariables
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
            name: { kind: "Name", value: "Torrents" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                  name: { kind: "Name", value: "PageInfo" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "TotalCount" },
                      },
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "HasNextPage" },
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
            name: { kind: "Name", value: "Torrents" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
                              name: { kind: "Name", value: "Files" },
                              arguments: [
                                {
                                  kind: "Argument",
                                  name: { kind: "Name", value: "Page" },
                                  value: {
                                    kind: "ObjectValue",
                                    fields: [
                                      {
                                        kind: "ObjectField",
                                        name: { kind: "Name", value: "Limit" },
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
                                    name: { kind: "Name", value: "Edges" },
                                    selectionSet: {
                                      kind: "SelectionSet",
                                      selections: [
                                        {
                                          kind: "Field",
                                          name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "PendingFileMatches" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Where" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Where" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "UpdateTorrent" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "Torrent" },
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
            name: { kind: "Name", value: "UsenetServers" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "OrderBy" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "OrderBy" },
                },
              },
              {
                kind: "Argument",
                name: { kind: "Name", value: "Page" },
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
                  name: { kind: "Name", value: "Edges" },
                  selectionSet: {
                    kind: "SelectionSet",
                    selections: [
                      {
                        kind: "Field",
                        name: { kind: "Name", value: "Node" },
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
            name: { kind: "Name", value: "UpdateUsenetServer" },
            arguments: [
              {
                kind: "Argument",
                name: { kind: "Name", value: "Id" },
                value: {
                  kind: "Variable",
                  name: { kind: "Name", value: "Id" },
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
                { kind: "Field", name: { kind: "Name", value: "Error" } },
                {
                  kind: "Field",
                  name: { kind: "Name", value: "UsenetServer" },
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
            name: { kind: "Name", value: "CreateUsenetServer" },
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
                  name: { kind: "Name", value: "UsenetServer" },
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
            name: { kind: "Name", value: "DeleteUsenetServer" },
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
