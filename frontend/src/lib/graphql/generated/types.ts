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

/** Album Entity */
export type Album = {
  AlbumType?: Maybe<Scalars["String"]["output"]>;
  ArtistId: Scalars["String"]["output"];
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
  /** Tracks in this album */
  Tracks: Array<Track>;
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

export type AlbumWhereInput = {
  AlbumType?: InputMaybe<StringFilter>;
  /** Logical AND of conditions */
  And?: InputMaybe<Array<AlbumWhereInput>>;
  ArtistId?: InputMaybe<StringFilter>;
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
  /** Get related #graphql_name with optional filtering, sorting, and pagination */
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
  OrderBy?: InputMaybe<Array<AlbumOrderByInput>>;
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
  ChapterCount?: Maybe<Scalars["Int"]["output"]>;
  /** Get related #graphql_name with optional filtering, sorting, and pagination */
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
  OrderBy?: InputMaybe<Array<ChapterOrderByInput>>;
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

export type AudiobookWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<AudiobookWhereInput>>;
  Asin?: InputMaybe<StringFilter>;
  AudibleId?: InputMaybe<StringFilter>;
  AuthorName?: InputMaybe<StringFilter>;
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
  Title?: Maybe<Scalars["String"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
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
};

export type CopyFilesInput = {
  Destination: Scalars["String"]["input"];
  Overwrite?: InputMaybe<Scalars["Boolean"]["input"]>;
  Sources: Array<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateAlbumInput = {
  AlbumType?: InputMaybe<Scalars["String"]["input"]>;
  ArtistId: Scalars["String"]["input"];
  Country?: InputMaybe<Scalars["String"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateAppLogInput = {
  CreatedAt: Scalars["String"]["input"];
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
  CreatedAt: Scalars["String"]["input"];
  Description?: InputMaybe<Scalars["String"]["input"]>;
  Key: Scalars["String"]["input"];
  UpdatedAt: Scalars["String"]["input"];
  Value: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateArtistInput = {
  AlbumCount?: InputMaybe<Scalars["Int"]["input"]>;
  Bio?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
  Disambiguation?: InputMaybe<Scalars["String"]["input"]>;
  ImageUrl?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  TotalDurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  TrackCount?: InputMaybe<Scalars["Int"]["input"]>;
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateArtworkCacheInput = {
  ArtworkType: Scalars["String"]["input"];
  ContentHash: Scalars["String"]["input"];
  CreatedAt: Scalars["String"]["input"];
  EntityId: Scalars["String"]["input"];
  EntityType: Scalars["String"]["input"];
  Height?: InputMaybe<Scalars["Int"]["input"]>;
  MimeType: Scalars["String"]["input"];
  SizeBytes: Scalars["Int"]["input"];
  SourceUrl?: InputMaybe<Scalars["String"]["input"]>;
  UpdatedAt: Scalars["String"]["input"];
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
  CreatedAt: Scalars["String"]["input"];
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
  ChapterCount?: InputMaybe<Scalars["Int"]["input"]>;
  CoverUrl?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateCastDeviceInput = {
  Address: Scalars["String"]["input"];
  CreatedAt: Scalars["String"]["input"];
  DeviceType: Scalars["String"]["input"];
  IsFavorite: Scalars["Boolean"]["input"];
  IsManual: Scalars["Boolean"]["input"];
  LastSeenAt?: InputMaybe<Scalars["String"]["input"]>;
  Model?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  Port: Scalars["Int"]["input"];
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateCastSessionInput = {
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  Volume: Scalars["Float"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateCastSettingInput = {
  AutoDiscoveryEnabled: Scalars["Boolean"]["input"];
  CreatedAt: Scalars["String"]["input"];
  DefaultVolume: Scalars["Float"]["input"];
  DiscoveryIntervalSeconds: Scalars["Int"]["input"];
  PreferredQuality?: InputMaybe<Scalars["String"]["input"]>;
  TranscodeIncompatible: Scalars["Boolean"]["input"];
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateChapterInput = {
  AudiobookId: Scalars["String"]["input"];
  ChapterNumber: Scalars["Int"]["input"];
  CreatedAt: Scalars["String"]["input"];
  DurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  EndTimeSecs?: InputMaybe<Scalars["Float"]["input"]>;
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  StartTimeSecs: Scalars["Float"]["input"];
  Title?: InputMaybe<Scalars["String"]["input"]>;
  UpdatedAt: Scalars["String"]["input"];
};

export type CreateDirectoryInput = {
  Path: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateEpisodeInput = {
  AbsoluteNumber?: InputMaybe<Scalars["Int"]["input"]>;
  AirDate?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateIndexerConfigInput = {
  Capabilities?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
  CredentialNonce: Scalars["String"]["input"];
  CredentialType: Scalars["String"]["input"];
  CredentialValue: Scalars["String"]["input"];
  DefinitionId?: InputMaybe<Scalars["String"]["input"]>;
  Enabled: Scalars["Boolean"]["input"];
  ErrorCount: Scalars["Int"]["input"];
  IndexerType: Scalars["String"]["input"];
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastErrorAt?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessAt?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  PostDownloadAction?: InputMaybe<Scalars["String"]["input"]>;
  Priority: Scalars["Int"]["input"];
  SiteUrl?: InputMaybe<Scalars["String"]["input"]>;
  SupportsBookSearch: Scalars["Boolean"]["input"];
  SupportsImdbSearch: Scalars["Boolean"]["input"];
  SupportsMovieSearch: Scalars["Boolean"]["input"];
  SupportsMusicSearch: Scalars["Boolean"]["input"];
  SupportsSearch: Scalars["Boolean"]["input"];
  SupportsTvSearch: Scalars["Boolean"]["input"];
  SupportsTvdbSearch: Scalars["Boolean"]["input"];
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateIndexerSearchCacheInput = {
  CreatedAt: Scalars["String"]["input"];
  ExpiresAt: Scalars["String"]["input"];
  IndexerConfigId: Scalars["String"]["input"];
  QueryHash: Scalars["String"]["input"];
  QueryType: Scalars["String"]["input"];
  ResultCount: Scalars["Int"]["input"];
  Results: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateIndexerSettingInput = {
  CreatedAt: Scalars["String"]["input"];
  IndexerConfigId: Scalars["String"]["input"];
  SettingKey: Scalars["String"]["input"];
  SettingValue: Scalars["String"]["input"];
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateInviteTokenInput = {
  AccessLevel: Scalars["String"]["input"];
  ApplyRestrictions: Scalars["Boolean"]["input"];
  CreatedAt: Scalars["String"]["input"];
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
  AutoAddDiscovered: Scalars["Boolean"]["input"];
  AutoDownload: Scalars["Boolean"]["input"];
  AutoHunt: Scalars["Boolean"]["input"];
  AutoScan: Scalars["Boolean"]["input"];
  Color?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
  Icon?: InputMaybe<Scalars["String"]["input"]>;
  LastScannedAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Path: Scalars["String"]["input"];
  ScanIntervalMinutes: Scalars["Int"]["input"];
  Scanning: Scalars["Boolean"]["input"];
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
  WatchForChanges: Scalars["Boolean"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateMediaChapterInput = {
  ChapterIndex: Scalars["Int"]["input"];
  CreatedAt: Scalars["String"]["input"];
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
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  CastNames: Array<Scalars["String"]["input"]>;
  Certification?: InputMaybe<Scalars["String"]["input"]>;
  CollectionId?: InputMaybe<Scalars["Int"]["input"]>;
  CollectionName?: InputMaybe<Scalars["String"]["input"]>;
  CollectionPosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  ProductionCountries: Array<Scalars["String"]["input"]>;
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  SpokenLanguages: Array<Scalars["String"]["input"]>;
  Status?: InputMaybe<Scalars["String"]["input"]>;
  Tagline?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TmdbRating?: InputMaybe<Scalars["String"]["input"]>;
  TmdbVoteCount?: InputMaybe<Scalars["Int"]["input"]>;
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateNamingPatternInput = {
  CreatedAt: Scalars["String"]["input"];
  Description?: InputMaybe<Scalars["String"]["input"]>;
  IsDefault: Scalars["Boolean"]["input"];
  IsSystem: Scalars["Boolean"]["input"];
  LibraryType: Scalars["String"]["input"];
  Name: Scalars["String"]["input"];
  Pattern: Scalars["String"]["input"];
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateNotificationInput = {
  ActionData?: InputMaybe<Scalars["String"]["input"]>;
  ActionType?: InputMaybe<Scalars["String"]["input"]>;
  Category: Scalars["String"]["input"];
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreatePendingFileMatchInput = {
  ChapterId?: InputMaybe<Scalars["String"]["input"]>;
  CopiedAt?: InputMaybe<Scalars["String"]["input"]>;
  CopyAttempts: Scalars["Int"]["input"];
  CopyError?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
  VerificationReason?: InputMaybe<Scalars["String"]["input"]>;
  VerificationStatus?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreatePlaybackProgressInput = {
  CreatedAt: Scalars["String"]["input"];
  CurrentPosition: Scalars["Float"]["input"];
  Duration?: InputMaybe<Scalars["Float"]["input"]>;
  IsWatched: Scalars["Boolean"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  ProgressPercent: Scalars["Float"]["input"];
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
  WatchedAt?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreatePlaybackSessionInput = {
  AlbumId?: InputMaybe<Scalars["String"]["input"]>;
  AudiobookId?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  ContentType?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
  Volume: Scalars["Float"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateRefreshTokenInput = {
  CreatedAt: Scalars["String"]["input"];
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
  CreatedAt: Scalars["String"]["input"];
  Enabled: Scalars["Boolean"]["input"];
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastPolledAt?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessfulAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  Name: Scalars["String"]["input"];
  PollIntervalMinutes: Scalars["Int"]["input"];
  PostDownloadAction?: InputMaybe<Scalars["String"]["input"]>;
  UpdatedAt: Scalars["String"]["input"];
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
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateScheduleSyncStateInput = {
  CountryCode: Scalars["String"]["input"];
  CreatedAt: Scalars["String"]["input"];
  LastSyncDays: Scalars["Int"]["input"];
  LastSyncedAt: Scalars["String"]["input"];
  SyncError?: InputMaybe<Scalars["String"]["input"]>;
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateShowInput = {
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  ContentRating?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
  EpisodeCount?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeFileCount?: InputMaybe<Scalars["Int"]["input"]>;
  Genres: Array<Scalars["String"]["input"]>;
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MonitorType: Scalars["String"]["input"];
  Monitored: Scalars["Boolean"]["input"];
  Name: Scalars["String"]["input"];
  Network?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  Status?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeId?: InputMaybe<Scalars["Int"]["input"]>;
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
  Year?: InputMaybe<Scalars["Int"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateSourcePriorityRuleInput = {
  CreatedAt: Scalars["String"]["input"];
  Enabled: Scalars["Boolean"]["input"];
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  PriorityOrder: Array<Scalars["String"]["input"]>;
  SearchAllSources: Scalars["Boolean"]["input"];
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateSubtitleInput = {
  Codec?: InputMaybe<Scalars["String"]["input"]>;
  CodecLongName?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateTorrentFileInput = {
  CreatedAt: Scalars["String"]["input"];
  DownloadedBytes: Scalars["Int"]["input"];
  FileIndex: Scalars["Int"]["input"];
  FilePath: Scalars["String"]["input"];
  FileSize: Scalars["Int"]["input"];
  IsExcluded: Scalars["Boolean"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  Progress: Scalars["Float"]["input"];
  RelativePath: Scalars["String"]["input"];
  TorrentId: Scalars["String"]["input"];
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateTorrentInput = {
  AddedAt: Scalars["String"]["input"];
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  CreatedAt: Scalars["String"]["input"];
  DiscNumber?: InputMaybe<Scalars["Int"]["input"]>;
  DurationSecs?: InputMaybe<Scalars["Int"]["input"]>;
  Explicit: Scalars["Boolean"]["input"];
  Isrc?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId: Scalars["String"]["input"];
  MediaFileId?: InputMaybe<Scalars["String"]["input"]>;
  MusicbrainzId?: InputMaybe<Scalars["String"]["input"]>;
  Title: Scalars["String"]["input"];
  TrackNumber: Scalars["Int"]["input"];
  UpdatedAt: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateUsenetDownloadInput = {
  AlbumId?: InputMaybe<Scalars["String"]["input"]>;
  AudiobookId?: InputMaybe<Scalars["String"]["input"]>;
  CompletedAt?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  UserId: Scalars["String"]["input"];
};

/** Input for creating a new #struct_name */
export type CreateUsenetServerInput = {
  Connections: Scalars["Int"]["input"];
  CreatedAt: Scalars["String"]["input"];
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
  UpdatedAt: Scalars["String"]["input"];
  UseSsl: Scalars["Boolean"]["input"];
  UserId: Scalars["String"]["input"];
  Username?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for creating a new #struct_name */
export type CreateUserInput = {
  AvatarUrl?: InputMaybe<Scalars["String"]["input"]>;
  CreatedAt: Scalars["String"]["input"];
  DisplayName?: InputMaybe<Scalars["String"]["input"]>;
  Email?: InputMaybe<Scalars["String"]["input"]>;
  IsActive: Scalars["Boolean"]["input"];
  LastLoginAt?: InputMaybe<Scalars["String"]["input"]>;
  Role: Scalars["String"]["input"];
  UpdatedAt: Scalars["String"]["input"];
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
  CreatedAt: Scalars["String"]["input"];
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
export type DeleteIndexerConfigsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteIndexerSearchCachesResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
};

/** Result of bulk delete by Where filter */
export type DeleteIndexerSettingsResult = {
  DeletedCount: Scalars["Int"]["output"];
  error?: Maybe<Scalars["String"]["output"]>;
  success: Scalars["Boolean"]["output"];
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
  Title?: Maybe<Scalars["String"]["output"]>;
  TmdbId?: Maybe<Scalars["Int"]["output"]>;
  TvdbId?: Maybe<Scalars["Int"]["output"]>;
  TvmazeId?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
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

export type IndexerConfig = {
  Capabilities?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  CredentialNonce: Scalars["String"]["output"];
  CredentialType: Scalars["String"]["output"];
  CredentialValue: Scalars["String"]["output"];
  DefinitionId?: Maybe<Scalars["String"]["output"]>;
  Enabled: Scalars["Boolean"]["output"];
  ErrorCount: Scalars["Int"]["output"];
  Id: Scalars["String"]["output"];
  IndexerType: Scalars["String"]["output"];
  LastError?: Maybe<Scalars["String"]["output"]>;
  LastErrorAt?: Maybe<Scalars["String"]["output"]>;
  LastSuccessAt?: Maybe<Scalars["String"]["output"]>;
  Name: Scalars["String"]["output"];
  PostDownloadAction?: Maybe<Scalars["String"]["output"]>;
  Priority: Scalars["Int"]["output"];
  SiteUrl?: Maybe<Scalars["String"]["output"]>;
  SupportsBookSearch: Scalars["Boolean"]["output"];
  SupportsImdbSearch: Scalars["Boolean"]["output"];
  SupportsMovieSearch: Scalars["Boolean"]["output"];
  SupportsMusicSearch: Scalars["Boolean"]["output"];
  SupportsSearch: Scalars["Boolean"]["output"];
  SupportsTvSearch: Scalars["Boolean"]["output"];
  SupportsTvdbSearch: Scalars["Boolean"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type IndexerConfigChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  IndexerConfig?: Maybe<IndexerConfig>;
};

/** Connection containing edges and page info */
export type IndexerConfigConnection = {
  /** The edges in this connection */
  Edges: Array<IndexerConfigEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type IndexerConfigEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: IndexerConfig;
};

export type IndexerConfigOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  IndexerType?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  Priority?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type IndexerConfigResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  IndexerConfig?: Maybe<IndexerConfig>;
  Success: Scalars["Boolean"]["output"];
};

export type IndexerConfigWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<IndexerConfigWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  CredentialType?: InputMaybe<StringFilter>;
  DefinitionId?: InputMaybe<StringFilter>;
  Enabled?: InputMaybe<BoolFilter>;
  ErrorCount?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  IndexerType?: InputMaybe<StringFilter>;
  LastErrorAt?: InputMaybe<DateFilter>;
  LastSuccessAt?: InputMaybe<DateFilter>;
  Name?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<IndexerConfigWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<IndexerConfigWhereInput>>;
  PostDownloadAction?: InputMaybe<StringFilter>;
  Priority?: InputMaybe<IntFilter>;
  SiteUrl?: InputMaybe<StringFilter>;
  SupportsBookSearch?: InputMaybe<BoolFilter>;
  SupportsImdbSearch?: InputMaybe<BoolFilter>;
  SupportsMovieSearch?: InputMaybe<BoolFilter>;
  SupportsMusicSearch?: InputMaybe<BoolFilter>;
  SupportsSearch?: InputMaybe<BoolFilter>;
  SupportsTvSearch?: InputMaybe<BoolFilter>;
  SupportsTvdbSearch?: InputMaybe<BoolFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
};

export type IndexerSearchCache = {
  CreatedAt: Scalars["String"]["output"];
  ExpiresAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  IndexerConfigId: Scalars["String"]["output"];
  QueryHash: Scalars["String"]["output"];
  QueryType: Scalars["String"]["output"];
  ResultCount: Scalars["Int"]["output"];
  Results: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type IndexerSearchCacheChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  IndexerSearchCache?: Maybe<IndexerSearchCache>;
};

/** Connection containing edges and page info */
export type IndexerSearchCacheConnection = {
  /** The edges in this connection */
  Edges: Array<IndexerSearchCacheEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type IndexerSearchCacheEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: IndexerSearchCache;
};

export type IndexerSearchCacheOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  ExpiresAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type IndexerSearchCacheResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  IndexerSearchCache?: Maybe<IndexerSearchCache>;
  Success: Scalars["Boolean"]["output"];
};

export type IndexerSearchCacheWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<IndexerSearchCacheWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  ExpiresAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IndexerConfigId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<IndexerSearchCacheWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<IndexerSearchCacheWhereInput>>;
  QueryHash?: InputMaybe<StringFilter>;
  QueryType?: InputMaybe<StringFilter>;
  ResultCount?: InputMaybe<IntFilter>;
};

/** IndexerSetting Entity - per-indexer settings */
export type IndexerSetting = {
  CreatedAt: Scalars["String"]["output"];
  Id: Scalars["String"]["output"];
  IndexerConfigId: Scalars["String"]["output"];
  SettingKey: Scalars["String"]["output"];
  SettingValue: Scalars["String"]["output"];
  UpdatedAt: Scalars["String"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type IndexerSettingChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  IndexerSetting?: Maybe<IndexerSetting>;
};

/** Connection containing edges and page info */
export type IndexerSettingConnection = {
  /** The edges in this connection */
  Edges: Array<IndexerSettingEdge>;
  /** Pagination information */
  PageInfo: PageInfo;
};

/** Edge containing a node and cursor */
export type IndexerSettingEdge = {
  /** A cursor for pagination */
  Cursor: Scalars["String"]["output"];
  /** The item at the end of the edge */
  Node: IndexerSetting;
};

export type IndexerSettingOrderByInput = {
  CreatedAt?: InputMaybe<SortDirection>;
  SettingKey?: InputMaybe<SortDirection>;
  UpdatedAt?: InputMaybe<SortDirection>;
};

/** Result type for #struct_name mutations */
export type IndexerSettingResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  IndexerSetting?: Maybe<IndexerSetting>;
  Success: Scalars["Boolean"]["output"];
};

export type IndexerSettingWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<IndexerSettingWhereInput>>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  IndexerConfigId?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<IndexerSettingWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<IndexerSettingWhereInput>>;
  SettingKey?: InputMaybe<StringFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
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

export type Library = {
  AutoAddDiscovered: Scalars["Boolean"]["output"];
  AutoDownload: Scalars["Boolean"]["output"];
  AutoHunt: Scalars["Boolean"]["output"];
  AutoScan: Scalars["Boolean"]["output"];
  Color?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  Icon?: Maybe<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  LastScannedAt?: Maybe<Scalars["String"]["output"]>;
  LibraryType: Scalars["String"]["output"];
  Name: Scalars["String"]["output"];
  Path: Scalars["String"]["output"];
  ScanIntervalMinutes: Scalars["Int"]["output"];
  Scanning: Scalars["Boolean"]["output"];
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  WatchForChanges: Scalars["Boolean"]["output"];
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

/** Result type for #struct_name mutations */
export type LibraryResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Library?: Maybe<Library>;
  Success: Scalars["Boolean"]["output"];
};

export type LibraryWhereInput = {
  /** Logical AND of conditions */
  And?: InputMaybe<Array<LibraryWhereInput>>;
  AutoAddDiscovered?: InputMaybe<BoolFilter>;
  AutoDownload?: InputMaybe<BoolFilter>;
  AutoHunt?: InputMaybe<BoolFilter>;
  AutoScan?: InputMaybe<BoolFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  Id?: InputMaybe<StringFilter>;
  LastScannedAt?: InputMaybe<DateFilter>;
  LibraryType?: InputMaybe<StringFilter>;
  Name?: InputMaybe<StringFilter>;
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
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  Monitored: Scalars["Boolean"]["output"];
  OriginalTitle?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  ProductionCountries: Array<Scalars["String"]["output"]>;
  ReleaseDate?: Maybe<Scalars["String"]["output"]>;
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  SortTitle?: Maybe<Scalars["String"]["output"]>;
  SpokenLanguages: Array<Scalars["String"]["output"]>;
  Status?: Maybe<Scalars["String"]["output"]>;
  Tagline?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TmdbId?: Maybe<Scalars["Int"]["output"]>;
  TmdbRating?: Maybe<Scalars["String"]["output"]>;
  TmdbVoteCount?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
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
  Status?: InputMaybe<StringFilter>;
  Title?: InputMaybe<StringFilter>;
  TmdbId?: InputMaybe<IntFilter>;
  TmdbVoteCount?: InputMaybe<IntFilter>;
  UpdatedAt?: InputMaybe<DateFilter>;
  UserId?: InputMaybe<StringFilter>;
  Year?: InputMaybe<IntFilter>;
};

export type MutationRoot = {
  /** Add a torrent from a magnet link or URL */
  AddTorrent: AddTorrentResult;
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
  CreateIndexerConfig: IndexerConfigResult;
  /** Create a new #struct_name_str */
  CreateIndexerSearchCache: IndexerSearchCacheResult;
  /** Create a new #struct_name_str */
  CreateIndexerSetting: IndexerSettingResult;
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
  DeleteIndexerConfig: IndexerConfigResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteIndexerConfigs: DeleteIndexerConfigsResult;
  /** Delete a #struct_name_str */
  DeleteIndexerSearchCache: IndexerSearchCacheResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteIndexerSearchCaches: DeleteIndexerSearchCachesResult;
  /** Delete a #struct_name_str */
  DeleteIndexerSetting: IndexerSettingResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteIndexerSettings: DeleteIndexerSettingsResult;
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
  DeleteSourcePriorityRule: SourcePriorityRuleResult;
  /** Delete multiple #plural_name matching the given Where filter */
  DeleteSourcePriorityRules: DeleteSourcePriorityRulesResult;
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
  Login: AuthPayload;
  Logout: LogoutPayload;
  MoveFiles: FileOperationPayload;
  /** Pause a torrent */
  PauseTorrent: TorrentActionResult;
  RefreshToken: AuthPayload;
  Register: AuthPayload;
  /** Remove a torrent */
  RemoveTorrent: TorrentActionResult;
  RenameFile: FileOperationPayload;
  /** Resume a paused torrent */
  ResumeTorrent: TorrentActionResult;
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
  /** Update an existing #struct_name_str */
  UpdateChapter: ChapterResult;
  /** Update an existing #struct_name_str */
  UpdateEpisode: EpisodeResult;
  /** Update an existing #struct_name_str */
  UpdateIndexerConfig: IndexerConfigResult;
  /** Update an existing #struct_name_str */
  UpdateIndexerSearchCache: IndexerSearchCacheResult;
  /** Update an existing #struct_name_str */
  UpdateIndexerSetting: IndexerSettingResult;
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

export type MutationRootAddTorrentArgs = {
  Input: AddTorrentInput;
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

export type MutationRootCreateIndexerConfigArgs = {
  Input: CreateIndexerConfigInput;
};

export type MutationRootCreateIndexerSearchCacheArgs = {
  Input: CreateIndexerSearchCacheInput;
};

export type MutationRootCreateIndexerSettingArgs = {
  Input: CreateIndexerSettingInput;
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

export type MutationRootDeleteIndexerConfigArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteIndexerConfigsArgs = {
  Where?: InputMaybe<IndexerConfigWhereInput>;
};

export type MutationRootDeleteIndexerSearchCacheArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteIndexerSearchCachesArgs = {
  Where?: InputMaybe<IndexerSearchCacheWhereInput>;
};

export type MutationRootDeleteIndexerSettingArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteIndexerSettingsArgs = {
  Where?: InputMaybe<IndexerSettingWhereInput>;
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

export type MutationRootDeleteSourcePriorityRuleArgs = {
  Id: Scalars["String"]["input"];
};

export type MutationRootDeleteSourcePriorityRulesArgs = {
  Where?: InputMaybe<SourcePriorityRuleWhereInput>;
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

export type MutationRootMoveFilesArgs = {
  Input: MoveFilesInput;
};

export type MutationRootPauseTorrentArgs = {
  Id: Scalars["Int"]["input"];
};

export type MutationRootRefreshTokenArgs = {
  Input: RefreshTokenInput;
};

export type MutationRootRegisterArgs = {
  Input: RegisterUserInput;
};

export type MutationRootRemoveTorrentArgs = {
  DeleteFiles?: Scalars["Boolean"]["input"];
  Id: Scalars["Int"]["input"];
};

export type MutationRootRenameFileArgs = {
  Input: RenameFileInput;
};

export type MutationRootResumeTorrentArgs = {
  Id: Scalars["Int"]["input"];
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

export type MutationRootUpdateChapterArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateChapterInput;
};

export type MutationRootUpdateEpisodeArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateEpisodeInput;
};

export type MutationRootUpdateIndexerConfigArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateIndexerConfigInput;
};

export type MutationRootUpdateIndexerSearchCacheArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateIndexerSearchCacheInput;
};

export type MutationRootUpdateIndexerSettingArgs = {
  Id: Scalars["String"]["input"];
  Input: UpdateIndexerSettingInput;
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
  /** Get a single #struct_name_str by ID */
  IndexerConfig?: Maybe<IndexerConfig>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  IndexerConfigs: IndexerConfigConnection;
  /** Get a single #struct_name_str by ID */
  IndexerSearchCache?: Maybe<IndexerSearchCache>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  IndexerSearchCaches: IndexerSearchCacheConnection;
  /** Get a single #struct_name_str by ID */
  IndexerSetting?: Maybe<IndexerSetting>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  IndexerSettings: IndexerSettingConnection;
  /** Get a single #struct_name_str by ID */
  InviteToken?: Maybe<InviteToken>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  InviteTokens: InviteTokenConnection;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Libraries: LibraryConnection;
  /** Get a single #struct_name_str by ID */
  Library?: Maybe<Library>;
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
  /** Get a single #struct_name_str by ID */
  Show?: Maybe<Show>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  Shows: ShowConnection;
  /** Get a single #struct_name_str by ID */
  SourcePriorityRule?: Maybe<SourcePriorityRule>;
  /** Get a list of #plural_name with optional filtering, sorting, and pagination */
  SourcePriorityRules: SourcePriorityRuleConnection;
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

export type QueryRootIndexerConfigArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootIndexerConfigsArgs = {
  OrderBy?: InputMaybe<Array<IndexerConfigOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<IndexerConfigWhereInput>;
};

export type QueryRootIndexerSearchCacheArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootIndexerSearchCachesArgs = {
  OrderBy?: InputMaybe<Array<IndexerSearchCacheOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<IndexerSearchCacheWhereInput>;
};

export type QueryRootIndexerSettingArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootIndexerSettingsArgs = {
  OrderBy?: InputMaybe<Array<IndexerSettingOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<IndexerSettingWhereInput>;
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

export type QueryRootShowArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootShowsArgs = {
  OrderBy?: InputMaybe<Array<ShowOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<ShowWhereInput>;
};

export type QueryRootSourcePriorityRuleArgs = {
  Id: Scalars["String"]["input"];
};

export type QueryRootSourcePriorityRulesArgs = {
  OrderBy?: InputMaybe<Array<SourcePriorityRuleOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<SourcePriorityRuleWhereInput>;
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

export type Show = {
  BackdropUrl?: Maybe<Scalars["String"]["output"]>;
  ContentRating?: Maybe<Scalars["String"]["output"]>;
  CreatedAt: Scalars["String"]["output"];
  EpisodeCount?: Maybe<Scalars["Int"]["output"]>;
  EpisodeFileCount?: Maybe<Scalars["Int"]["output"]>;
  /** Episodes in this show */
  Episodes: Array<Episode>;
  Genres: Array<Scalars["String"]["output"]>;
  Id: Scalars["String"]["output"];
  ImdbId?: Maybe<Scalars["String"]["output"]>;
  LibraryId: Scalars["String"]["output"];
  MonitorType: Scalars["String"]["output"];
  Monitored: Scalars["Boolean"]["output"];
  Name: Scalars["String"]["output"];
  Network?: Maybe<Scalars["String"]["output"]>;
  Overview?: Maybe<Scalars["String"]["output"]>;
  Path?: Maybe<Scalars["String"]["output"]>;
  PosterUrl?: Maybe<Scalars["String"]["output"]>;
  Runtime?: Maybe<Scalars["Int"]["output"]>;
  SizeBytes?: Maybe<Scalars["Int"]["output"]>;
  SortName?: Maybe<Scalars["String"]["output"]>;
  Status?: Maybe<Scalars["String"]["output"]>;
  TmdbId?: Maybe<Scalars["Int"]["output"]>;
  TvdbId?: Maybe<Scalars["Int"]["output"]>;
  TvmazeId?: Maybe<Scalars["Int"]["output"]>;
  UpdatedAt: Scalars["String"]["output"];
  UserId: Scalars["String"]["output"];
  Year?: Maybe<Scalars["Int"]["output"]>;
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
  EpisodeCount?: InputMaybe<SortDirection>;
  Name?: InputMaybe<SortDirection>;
  SizeBytes?: InputMaybe<SortDirection>;
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
  ContentRating?: InputMaybe<StringFilter>;
  CreatedAt?: InputMaybe<DateFilter>;
  EpisodeCount?: InputMaybe<IntFilter>;
  EpisodeFileCount?: InputMaybe<IntFilter>;
  Id?: InputMaybe<StringFilter>;
  ImdbId?: InputMaybe<StringFilter>;
  LibraryId?: InputMaybe<StringFilter>;
  MonitorType?: InputMaybe<StringFilter>;
  Monitored?: InputMaybe<BoolFilter>;
  Name?: InputMaybe<StringFilter>;
  Network?: InputMaybe<StringFilter>;
  /** Logical NOT of condition */
  Not?: InputMaybe<ShowWhereInput>;
  /** Logical OR of conditions */
  Or?: InputMaybe<Array<ShowWhereInput>>;
  Runtime?: InputMaybe<IntFilter>;
  SizeBytes?: InputMaybe<IntFilter>;
  Status?: InputMaybe<StringFilter>;
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
  IndexerConfigChanged: IndexerConfigChangedEvent;
  /** Subscribe to #struct_name_str changes */
  IndexerSearchCacheChanged: IndexerSearchCacheChangedEvent;
  /** Subscribe to #struct_name_str changes */
  IndexerSettingChanged: IndexerSettingChangedEvent;
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
  SourcePriorityRuleChanged: SourcePriorityRuleChangedEvent;
  /** Subscribe to #struct_name_str changes */
  SubtitleChanged: SubtitleChangedEvent;
  /** Subscribe to #struct_name_str changes */
  TorrentChanged: TorrentChangedEvent;
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

export type SubscriptionRootIndexerConfigChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootIndexerSearchCacheChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootIndexerSettingChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
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

export type SubscriptionRootSourcePriorityRuleChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootSubtitleChangedArgs = {
  Filter?: InputMaybe<SubscriptionFilterInput>;
};

export type SubscriptionRootTorrentChangedArgs = {
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
  /** Get related #graphql_name with optional filtering, sorting, and pagination */
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
  UploadedBytes: Scalars["Int"]["output"];
  UserId: Scalars["String"]["output"];
};

export type TorrentFilesArgs = {
  OrderBy?: InputMaybe<Array<TorrentFileOrderByInput>>;
  Page?: InputMaybe<PageInput>;
  Where?: InputMaybe<TorrentFileWhereInput>;
};

/** Result of pause/resume/remove */
export type TorrentActionResult = {
  Error?: Maybe<Scalars["String"]["output"]>;
  Success: Scalars["Boolean"]["output"];
};

/** Event for #struct_name changes (subscriptions) */
export type TorrentChangedEvent = {
  Action: ChangeAction;
  Id: Scalars["String"]["output"];
  Torrent?: Maybe<Torrent>;
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
  /** Get related #graphql_name */
  MediaFile?: Maybe<MediaFile>;
  MediaFileId?: Maybe<Scalars["String"]["output"]>;
  MusicbrainzId?: Maybe<Scalars["String"]["output"]>;
  Title: Scalars["String"]["output"];
  TrackNumber: Scalars["Int"]["output"];
  UpdatedAt: Scalars["String"]["output"];
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
};

/** Input for updating an existing #struct_name */
export type UpdateAlbumInput = {
  AlbumType?: InputMaybe<Scalars["String"]["input"]>;
  ArtistId?: InputMaybe<Scalars["String"]["input"]>;
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
};

/** Input for updating an existing #struct_name */
export type UpdateIndexerConfigInput = {
  Capabilities?: InputMaybe<Scalars["String"]["input"]>;
  CredentialNonce?: InputMaybe<Scalars["String"]["input"]>;
  CredentialType?: InputMaybe<Scalars["String"]["input"]>;
  CredentialValue?: InputMaybe<Scalars["String"]["input"]>;
  DefinitionId?: InputMaybe<Scalars["String"]["input"]>;
  Enabled?: InputMaybe<Scalars["Boolean"]["input"]>;
  ErrorCount?: InputMaybe<Scalars["Int"]["input"]>;
  IndexerType?: InputMaybe<Scalars["String"]["input"]>;
  LastError?: InputMaybe<Scalars["String"]["input"]>;
  LastErrorAt?: InputMaybe<Scalars["String"]["input"]>;
  LastSuccessAt?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  PostDownloadAction?: InputMaybe<Scalars["String"]["input"]>;
  Priority?: InputMaybe<Scalars["Int"]["input"]>;
  SiteUrl?: InputMaybe<Scalars["String"]["input"]>;
  SupportsBookSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsImdbSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsMovieSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsMusicSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsTvSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  SupportsTvdbSearch?: InputMaybe<Scalars["Boolean"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
export type UpdateIndexerSearchCacheInput = {
  ExpiresAt?: InputMaybe<Scalars["String"]["input"]>;
  IndexerConfigId?: InputMaybe<Scalars["String"]["input"]>;
  QueryHash?: InputMaybe<Scalars["String"]["input"]>;
  QueryType?: InputMaybe<Scalars["String"]["input"]>;
  ResultCount?: InputMaybe<Scalars["Int"]["input"]>;
  Results?: InputMaybe<Scalars["String"]["input"]>;
};

/** Input for updating an existing #struct_name */
export type UpdateIndexerSettingInput = {
  IndexerConfigId?: InputMaybe<Scalars["String"]["input"]>;
  SettingKey?: InputMaybe<Scalars["String"]["input"]>;
  SettingValue?: InputMaybe<Scalars["String"]["input"]>;
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
  AutoAddDiscovered?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoDownload?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoHunt?: InputMaybe<Scalars["Boolean"]["input"]>;
  AutoScan?: InputMaybe<Scalars["Boolean"]["input"]>;
  Color?: InputMaybe<Scalars["String"]["input"]>;
  Icon?: InputMaybe<Scalars["String"]["input"]>;
  LastScannedAt?: InputMaybe<Scalars["String"]["input"]>;
  LibraryType?: InputMaybe<Scalars["String"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
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
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
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
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  ProductionCountries?: InputMaybe<Array<Scalars["String"]["input"]>>;
  ReleaseDate?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SortTitle?: InputMaybe<Scalars["String"]["input"]>;
  SpokenLanguages?: InputMaybe<Array<Scalars["String"]["input"]>>;
  Status?: InputMaybe<Scalars["String"]["input"]>;
  Tagline?: InputMaybe<Scalars["String"]["input"]>;
  Title?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TmdbRating?: InputMaybe<Scalars["String"]["input"]>;
  TmdbVoteCount?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
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
  BackdropUrl?: InputMaybe<Scalars["String"]["input"]>;
  ContentRating?: InputMaybe<Scalars["String"]["input"]>;
  EpisodeCount?: InputMaybe<Scalars["Int"]["input"]>;
  EpisodeFileCount?: InputMaybe<Scalars["Int"]["input"]>;
  Genres?: InputMaybe<Array<Scalars["String"]["input"]>>;
  ImdbId?: InputMaybe<Scalars["String"]["input"]>;
  LibraryId?: InputMaybe<Scalars["String"]["input"]>;
  MonitorType?: InputMaybe<Scalars["String"]["input"]>;
  Monitored?: InputMaybe<Scalars["Boolean"]["input"]>;
  Name?: InputMaybe<Scalars["String"]["input"]>;
  Network?: InputMaybe<Scalars["String"]["input"]>;
  Overview?: InputMaybe<Scalars["String"]["input"]>;
  Path?: InputMaybe<Scalars["String"]["input"]>;
  PosterUrl?: InputMaybe<Scalars["String"]["input"]>;
  Runtime?: InputMaybe<Scalars["Int"]["input"]>;
  SizeBytes?: InputMaybe<Scalars["Int"]["input"]>;
  SortName?: InputMaybe<Scalars["String"]["input"]>;
  Status?: InputMaybe<Scalars["String"]["input"]>;
  TmdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvdbId?: InputMaybe<Scalars["Int"]["input"]>;
  TvmazeId?: InputMaybe<Scalars["Int"]["input"]>;
  UserId?: InputMaybe<Scalars["String"]["input"]>;
  Year?: InputMaybe<Scalars["Int"]["input"]>;
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

export type DashboardShowsQueryVariables = Exact<{
  Where?: InputMaybe<ShowWhereInput>;
  Page?: InputMaybe<PageInput>;
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
        Status?: string | null;
        TvmazeId?: number | null;
        TmdbId?: number | null;
        TvdbId?: number | null;
        ImdbId?: string | null;
        Overview?: string | null;
        Network?: string | null;
        Runtime?: number | null;
        PosterUrl?: string | null;
        BackdropUrl?: string | null;
        Monitored: boolean;
        MonitorType: string;
        Path?: string | null;
        EpisodeCount?: number | null;
        EpisodeFileCount?: number | null;
        SizeBytes?: number | null;
        Genres: Array<string>;
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
        Name: string;
        Path: string;
        LibraryType: string;
        Icon?: string | null;
        Color?: string | null;
        AutoScan: boolean;
        ScanIntervalMinutes: number;
        WatchForChanges: boolean;
        AutoAddDiscovered: boolean;
        AutoDownload: boolean;
        AutoHunt: boolean;
        Scanning: boolean;
        LastScannedAt?: string | null;
        CreatedAt: string;
        UpdatedAt: string;
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
      AutoAddDiscovered: boolean;
      AutoDownload: boolean;
      AutoHunt: boolean;
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

export type ActiveDownloadCountQueryVariables = Exact<{
  Where?: InputMaybe<TorrentWhereInput>;
  Page?: InputMaybe<PageInput>;
}>;

export type ActiveDownloadCountQuery = {
  Torrents: { PageInfo: { TotalCount?: number | null } };
};

export type TorrentChangedSubscriptionVariables = Exact<{
  Filter?: InputMaybe<SubscriptionFilterInput>;
}>;

export type TorrentChangedSubscription = {
  TorrentChanged: { Action: ChangeAction; Id: string };
};
