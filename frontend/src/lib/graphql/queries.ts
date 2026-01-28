// ============================================================================
// Auth: use generated documents from ./generated/graphql (NeedsSetupDocument, MeDocument)
// ============================================================================

// ============================================================================
// Torrent Queries
// ============================================================================

/** Legacy root-field torrents list (camelCase). Prefer DOWNLOADS_TORRENTS_QUERY for the downloads page. */
export const TORRENTS_QUERY = `
  query Torrents {
    torrents {
      id
      infoHash
      name
      state
      progress
      progressPercent
      size
      sizeFormatted
      downloaded
      uploaded
      downloadSpeed
      downloadSpeedFormatted
      uploadSpeed
      uploadSpeedFormatted
      peers
      eta
      addedAt
    }
  }
`;

/** Entity Torrents list (codegen) for downloads page. Uses Torrents(Where, Page) with PascalCase fields. */
export const DOWNLOADS_TORRENTS_QUERY = `
  query DownloadsTorrents($Where: TorrentWhereInput, $Page: PageInput) {
    Torrents(Where: $Where, Page: $Page) {
      Edges {
        Node {
          Id
          InfoHash
          Name
          State
          Progress
          TotalBytes
          DownloadedBytes
          UploadedBytes
          SavePath
          AddedAt
        }
      }
      PageInfo {
        TotalCount
        HasNextPage
      }
    }
  }
`;

/** Single Torrent by InfoHash (for TorrentInfoModal). */
export const TORRENT_BY_INFO_HASH_QUERY = `
  query TorrentByInfoHash($Where: TorrentWhereInput, $Page: PageInput) {
    Torrents(Where: $Where, Page: $Page) {
      Edges {
        Node {
          Id
          InfoHash
          Name
          State
          Progress
          TotalBytes
          DownloadedBytes
          UploadedBytes
          SavePath
          AddedAt
          Files(Page: { Limit: 500 }) {
            Edges { Node { FileIndex FilePath FileSize DownloadedBytes Progress } }
          }
        }
      }
    }
  }
`;

export const TORRENT_QUERY = `
  query Torrent($id: Int!) {
    torrent(id: $id) {
      id
      infoHash
      name
      state
      progress
      size
      sizeFormatted
    }
  }
`;

export const TORRENT_DETAILS_QUERY = `
  query TorrentDetails($id: Int!) {
    torrentDetails(id: $id) {
      id
      infoHash
      name
      state
      progress
      progressPercent
      size
      sizeFormatted
      downloaded
      downloadedFormatted
      uploaded
      uploadedFormatted
      downloadSpeed
      downloadSpeedFormatted
      uploadSpeed
      uploadSpeedFormatted
      savePath
      files {
        index
        path
        size
        progress
      }
      pieceCount
      piecesDownloaded
      averagePieceDownloadMs
      timeRemainingSecs
      timeRemainingFormatted
      peerStats {
        queued
        connecting
        live
        seen
        dead
        notNeeded
      }
      error
      finished
      ratio
    }
  }
`;

/** Query pending file matches for any source (torrent, usenet, etc.) */
export const PENDING_FILE_MATCHES_QUERY = `
  query PendingFileMatches($sourceType: String!, $sourceId: String!) {
    pendingFileMatches(sourceType: $sourceType, sourceId: $sourceId) {
      id
      sourceType
      sourceId
      sourceFileIndex
      sourcePath
      fileSize
      episodeId
      movieId
      trackId
      chapterId
      matchType
      matchConfidence
      parsedResolution
      parsedCodec
      parsedSource
      parsedAudio
      copied
      copiedAt
      copyError
      createdAt
    }
  }
`;

/**
 * Lightweight query to get the active download count
 *
 * Use this to initialize the navbar badge before subscribing to updates.
 */
export const ACTIVE_DOWNLOAD_COUNT_QUERY = `
  query ActiveDownloadCount {
    activeDownloadCount
  }
`;

// ============================================================================
// Settings Queries
// ============================================================================

export const TORRENT_SETTINGS_QUERY = `
  query TorrentSettings {
    torrentSettings {
      downloadDir
      sessionDir
      enableDht
      listenPort
      maxConcurrent
      uploadLimit
      downloadLimit
    }
  }
`;

export const UPnP_STATUS_QUERY = `
  query UpnpStatus {
    upnpStatus {
      success
      tcpForwarded
      udpForwarded
      localIp
      externalIp
      error
    }
  }
`;

export const TEST_PORT_ACCESSIBILITY_QUERY = `
  query TestPortAccessibility($port: Int) {
    testPortAccessibility(port: $port) {
      success
      portOpen
      externalIp
      error
    }
  }
`;

export const LLM_PARSER_SETTINGS_QUERY = `
  query LlmParserSettings {
    llmParserSettings {
      enabled
      ollamaUrl
      ollamaModel
      timeoutSeconds
      temperature
      maxTokens
      promptTemplate
      confidenceThreshold
      modelMovies
      modelTv
      modelMusic
      modelAudiobooks
      promptMovies
      promptTv
      promptMusic
      promptAudiobooks
    }
  }
`;





// ============================================================================
// GraphQL Libraries Query with Counts
// ============================================================================

export const LIBRARIES_WITH_COUNTS_QUERY = `
  query LibrariesWithCounts {
    Libraries {
      Edges {
        Node {
          Id
          UserId
          Name
          Path
          LibraryType
          Icon
          Color
          AutoScan
          ScanIntervalMinutes
          WatchForChanges
          AutoAddDiscovered
          AutoDownload
          AutoHunt
          Scanning
          LastScannedAt
          CreatedAt
          UpdatedAt
          Shows {
            PageInfo {
              TotalCount
            }
          }
          Movies {
            PageInfo {
              TotalCount
            }
          }
          Albums {
            PageInfo {
              TotalCount
            }
          }
          Audiobooks {
            PageInfo {
              TotalCount
            }
          }
        }
      }
      PageInfo {
        TotalCount
      }
    }
  }
`;

// ============================================================================
// Library Queries
// ============================================================================

export const LIBRARIES_QUERY = `
  query Libraries {
    libraries {
      id
      name
      path
      libraryType
      icon
      color
      autoScan
      scanIntervalMinutes
      watchForChanges
      organizeFiles
      namingPattern
      autoAddDiscovered
      itemCount
      totalSizeBytes
      showCount
      movieCount
      lastScannedAt
    }
  }
`;

export const LIBRARY_QUERY = `
  query Library($Id: String!) {
    Library(Id: $Id) {
      Id
      Name
      Path
      LibraryType
      Icon
      Color
      AutoScan
      ScanIntervalMinutes
      WatchForChanges
      AutoAddDiscovered
      AutoDownload
      AutoHunt
      Scanning
      LastScannedAt
      CreatedAt
      UpdatedAt
      UserId
    }
  }
`;

// ============================================================================
// TV Show Queries
// ============================================================================

export const ALL_TV_SHOWS_QUERY = `
  query AllTvShows {
    Shows {
      Edges {
        Node {
          Id
          LibraryId
          Name
          SortName
          Year
          Status
          PosterUrl
          Monitored
        }
      }
    }
  }
`;

export const TV_SHOWS_QUERY = `
  query TvShows($libraryId: String!) {
    Shows(Where: { LibraryId: { Eq: $libraryId } }) {
      Edges {
        Node {
          Id
          LibraryId
          Name
          SortName
          Year
          Status
          TvmazeId
          TmdbId
          TvdbId
          ImdbId
          Overview
          Network
          Runtime
          Genres
          PosterUrl
          BackdropUrl
          Monitored
          MonitorType
          Path
          EpisodeCount
          EpisodeFileCount
          SizeBytes
        }
      }
    }
  }
`;

export const TV_SHOWS_CONNECTION_QUERY = `
  query TvShowsConnection(
    $Where: ShowWhereInput
    $Page: PageInput
    $OrderBy: [ShowOrderByInput]
  ) {
    Shows(Where: $Where, Page: $Page, OrderBy: $OrderBy) {
      Edges {
        Node {
          Id
          LibraryId
          Name
          SortName
          Year
          Status
          PosterUrl
          BackdropUrl
          Monitored
          EpisodeCount
          EpisodeFileCount
        }
        Cursor
      }
      PageInfo {
        HasNextPage
        HasPreviousPage
        StartCursor
        EndCursor
        TotalCount
      }
    }
  }
`;

export const TV_SHOW_QUERY = `
  query TvShow($Id: String!) {
    Show(Id: $Id) {
      Id
      LibraryId
      Name
      SortName
      Year
      Status
      TvmazeId
      TmdbId
      TvdbId
      ImdbId
      Overview
      Network
      Runtime
      Genres
      PosterUrl
      BackdropUrl
      Monitored
      MonitorType
      Path
      EpisodeCount
      EpisodeFileCount
      SizeBytes
      CreatedAt
      UpdatedAt
      UserId
    }
  }
`;

export const SEARCH_TV_SHOWS_QUERY = `
  query SearchTvShows($query: String!) {
    searchTvShows(query: $query) {
      provider
      providerId
      name
      year
      status
      network
      overview
      posterUrl
      tvdbId
      imdbId
      score
    }
  }
`;

// ============================================================================
// Movie Queries
// ============================================================================

export const ALL_MOVIES_QUERY = `
  query AllMovies {
    Movies {
      Edges {
        Node {
          Id
          LibraryId
          Title
          SortTitle
          Year
          Status
          PosterUrl
          Monitored
          MediaFileId
        }
      }
    }
  }
`;

export const MOVIES_QUERY = `
  query Movies($libraryId: String!) {
    Movies(Where: { LibraryId: { Eq: $libraryId } }) {
      Edges {
        Node {
          Id
          LibraryId
          Title
          SortTitle
          OriginalTitle
          Year
          TmdbId
          ImdbId
          Status
          Overview
          Tagline
          Runtime
          Genres
          Director
          CastNames
          PosterUrl
          BackdropUrl
          Monitored
          MediaFileId
          CollectionId
          CollectionName
          CollectionPosterUrl
          TmdbRating
          TmdbVoteCount
          Certification
          ReleaseDate
        }
      }
    }
  }
`;

/** Movies query with pagination and filtering */
export const MOVIES_CONNECTION_QUERY = `
  query MoviesConnection(
    $Where: MovieWhereInput
    $Page: PageInput
    $OrderBy: [MovieOrderByInput]
  ) {
    Movies(Where: $Where, Page: $Page, OrderBy: $OrderBy) {
      Edges {
        Node {
          Id
          LibraryId
          Title
          SortTitle
          OriginalTitle
          Year
          TmdbId
          ImdbId
          Status
          Overview
          Runtime
          Genres
          Director
          PosterUrl
          BackdropUrl
          Monitored
          MediaFileId
          TmdbRating
          ReleaseDate
        }
        Cursor
      }
      PageInfo {
        HasNextPage
        HasPreviousPage
        StartCursor
        EndCursor
        TotalCount
      }
    }
  }
`;

export const MOVIE_QUERY = `
  query Movie($Id: String!) {
    Movie(Id: $Id) {
      Id
      LibraryId
      Title
      SortTitle
      OriginalTitle
      Year
      TmdbId
      ImdbId
      Status
      Overview
      Tagline
      Runtime
      Genres
      Director
      CastNames
      PosterUrl
      BackdropUrl
      Monitored
      MediaFileId
      CollectionId
      CollectionName
      CollectionPosterUrl
      TmdbRating
      TmdbVoteCount
      Certification
      ReleaseDate
    }
  }
`;

export const SEARCH_MOVIES_QUERY = `
  query SearchMovies($query: String!, $year: Int) {
    searchMovies(query: $query, year: $year) {
      provider
      providerId
      title
      originalTitle
      year
      overview
      posterUrl
      backdropUrl
      imdbId
      voteAverage
      popularity
    }
  }
`;

// ============================================================================
// Album/Music Queries
// ============================================================================

export const ALBUMS_QUERY = `
  query Albums($libraryId: String!) {
    albums(libraryId: $libraryId) {
      id
      artistId
      libraryId
      name
      sortName
      year
      musicbrainzId
      albumType
      genres
      label
      country
      releaseDate
      coverUrl
      trackCount
      discCount
      totalDurationSecs
      hasFiles
      sizeBytes
      path
    }
  }
`;

export const ALBUM_QUERY = `
  query Album($id: String!) {
    album(id: $id) {
      id
      artistId
      libraryId
      name
      sortName
      year
      musicbrainzId
      albumType
      genres
      label
      country
      releaseDate
      coverUrl
      trackCount
      discCount
      totalDurationSecs
      hasFiles
      sizeBytes
      path
    }
  }
`;

export const ALBUMS_CONNECTION_QUERY = `
  query AlbumsConnection($libraryId: String!, $first: Int, $after: String, $where: AlbumWhereInput, $orderBy: AlbumOrderByInput) {
    albumsConnection(libraryId: $libraryId, first: $first, after: $after, where: $where, orderBy: $orderBy) {
      edges {
        node {
          id
          artistId
          libraryId
          name
          sortName
          year
          albumType
          coverUrl
          hasFiles
          trackCount
          downloadedTrackCount
        }
        cursor
      }
      pageInfo {
        hasNextPage
        hasPreviousPage
        startCursor
        endCursor
        totalCount
      }
    }
  }
`;

export const ARTISTS_CONNECTION_QUERY = `
  query ArtistsConnection($libraryId: String!, $first: Int, $after: String, $where: ArtistWhereInput, $orderBy: ArtistOrderByInput) {
    artistsConnection(libraryId: $libraryId, first: $first, after: $after, where: $where, orderBy: $orderBy) {
      edges {
        node {
          id
          libraryId
          name
          sortName
          musicbrainzId
        }
        cursor
      }
      pageInfo {
        hasNextPage
        hasPreviousPage
        startCursor
        endCursor
        totalCount
      }
    }
  }
`;

export const ALBUM_WITH_TRACKS_QUERY = `
  query AlbumWithTracks($id: String!) {
    albumWithTracks(id: $id) {
      album {
        id
        artistId
        libraryId
        name
        sortName
        year
        musicbrainzId
        albumType
        genres
        label
        country
        releaseDate
        coverUrl
        trackCount
        discCount
        totalDurationSecs
        hasFiles
        sizeBytes
        path
      }
      artistName
      tracks {
        track {
          id
          albumId
          libraryId
          title
          trackNumber
          discNumber
          musicbrainzId
          isrc
          durationSecs
          explicit
          artistName
          artistId
          mediaFileId
          hasFile
          status
          downloadProgress
        }
        hasFile
        filePath
        fileSize
        audioCodec
        bitrate
        audioChannels
      }
      trackCount
      tracksWithFiles
      missingTracks
      completionPercent
    }
  }
`;

export const TRACKS_QUERY = `
  query Tracks($albumId: String!) {
    tracks(albumId: $albumId) {
      id
      albumId
      libraryId
      title
      trackNumber
      discNumber
      musicbrainzId
      isrc
      durationSecs
      explicit
      artistName
      artistId
      mediaFileId
      hasFile
      status
      downloadProgress
    }
  }
`;

/** Tracks query with cursor-based pagination and filtering */
export const TRACKS_CONNECTION_QUERY = `
  query TracksConnection(
    $libraryId: String!
    $first: Int
    $after: String
    $where: TrackWhereInput
    $orderBy: TrackOrderByInput
  ) {
    tracksConnection(
      libraryId: $libraryId
      first: $first
      after: $after
      where: $where
      orderBy: $orderBy
    ) {
      edges {
        node {
          id
          albumId
          libraryId
          title
          trackNumber
          discNumber
          durationSecs
          explicit
          artistName
          artistId
          mediaFileId
          hasFile
          status
          downloadProgress
        }
        cursor
      }
      pageInfo {
        hasNextPage
        hasPreviousPage
        startCursor
        endCursor
        totalCount
      }
    }
  }
`;

export const ARTISTS_QUERY = `
  query Artists($libraryId: String!) {
    artists(libraryId: $libraryId) {
      id
      libraryId
      name
      sortName
      musicbrainzId
    }
  }
`;

export const SEARCH_ALBUMS_QUERY = `
  query SearchAlbums(
    $query: String!
    $includeEps: Boolean
    $includeSingles: Boolean
    $includeCompilations: Boolean
    $includeLive: Boolean
    $includeSoundtracks: Boolean
  ) {
    searchAlbums(
      query: $query
      includeEps: $includeEps
      includeSingles: $includeSingles
      includeCompilations: $includeCompilations
      includeLive: $includeLive
      includeSoundtracks: $includeSoundtracks
    ) {
      provider
      providerId
      title
      artistName
      year
      albumType
      coverUrl
      score
    }
  }
`;

// ============================================================================
// Audiobook Queries
// ============================================================================

export const AUDIOBOOKS_QUERY = `
  query Audiobooks($libraryId: String!) {
    audiobooks(libraryId: $libraryId) {
      id
      authorId
      libraryId
      title
      sortTitle
      subtitle
      openlibraryId
      isbn
      description
      publisher
      language
      narrators
      seriesName
      durationSecs
      coverUrl
      hasFiles
      sizeBytes
      path
    }
  }
`;

export const AUDIOBOOK_QUERY = `
  query Audiobook($id: String!) {
    audiobook(id: $id) {
      id
      authorId
      libraryId
      title
      sortTitle
      subtitle
      openlibraryId
      isbn
      description
      publisher
      language
      narrators
      seriesName
      durationSecs
      coverUrl
      hasFiles
      sizeBytes
      path
    }
  }
`;

export const AUDIOBOOK_WITH_CHAPTERS_QUERY = `
  query AudiobookWithChapters($id: String!) {
    audiobookWithChapters(id: $id) {
      audiobook {
        id
        authorId
        libraryId
        title
        sortTitle
        subtitle
        openlibraryId
        isbn
        description
        publisher
        language
        narrators
        seriesName
        durationSecs
        coverUrl
        hasFiles
        sizeBytes
        path
      }
      author {
        id
        libraryId
        name
        sortName
        openlibraryId
      }
      chapters {
        id
        audiobookId
        chapterNumber
        title
        startSecs
        endSecs
        durationSecs
        mediaFileId
        status
        downloadProgress
      }
      chapterCount
      chaptersWithFiles
      missingChapters
      completionPercent
    }
  }
`;

export const AUDIOBOOK_CHAPTERS_QUERY = `
  query AudiobookChapters($audiobookId: String!) {
    audiobookChapters(audiobookId: $audiobookId) {
      id
      audiobookId
      chapterNumber
      title
      startSecs
      endSecs
      durationSecs
      mediaFileId
      status
      downloadProgress
    }
  }
`;

export const AUDIOBOOK_AUTHORS_QUERY = `
  query AudiobookAuthors($libraryId: String!) {
    audiobookAuthors(libraryId: $libraryId) {
      id
      libraryId
      name
      sortName
      openlibraryId
    }
  }
`;

export const SEARCH_AUDIOBOOKS_QUERY = `
  query SearchAudiobooks($query: String!) {
    searchAudiobooks(query: $query) {
      provider
      providerId
      title
      authorName
      year
      coverUrl
      isbn
      description
    }
  }
`;

export const AUDIOBOOKS_CONNECTION_QUERY = `
  query AudiobooksConnection($libraryId: String!, $first: Int, $after: String, $where: AudiobookWhereInput, $orderBy: AudiobookOrderByInput) {
    audiobooksConnection(libraryId: $libraryId, first: $first, after: $after, where: $where, orderBy: $orderBy) {
      edges {
        node {
          id
          authorId
          libraryId
          title
          sortTitle
          subtitle
          coverUrl
          hasFiles
          seriesName
          chapterCount
          downloadedChapterCount
        }
        cursor
      }
      pageInfo {
        hasNextPage
        hasPreviousPage
        startCursor
        endCursor
        totalCount
      }
    }
  }
`;

export const AUDIOBOOK_AUTHORS_CONNECTION_QUERY = `
  query AudiobookAuthorsConnection($libraryId: String!, $first: Int, $after: String, $where: AudiobookAuthorWhereInput, $orderBy: AudiobookAuthorOrderByInput) {
    audiobookAuthorsConnection(libraryId: $libraryId, first: $first, after: $after, where: $where, orderBy: $orderBy) {
      edges {
        node {
          id
          libraryId
          name
          sortName
          openlibraryId
        }
        cursor
      }
      pageInfo {
        hasNextPage
        hasPreviousPage
        startCursor
        endCursor
        totalCount
      }
    }
  }
`;

// ============================================================================
// Episode Queries
// ============================================================================

export const EPISODES_QUERY = `
  query Episodes($tvShowId: String!) {
    episodes(tvShowId: $tvShowId) {
      id
      tvShowId
      season
      episode
      absoluteNumber
      title
      overview
      airDate
      runtime
      tvmazeId
      tmdbId
      tvdbId
      mediaFileId
      resolution
      videoCodec
      audioCodec
      audioChannels
      isHdr
      hdrType
      videoBitrate
      fileSizeBytes
      fileSizeFormatted
      watchProgress
      watchPosition
      isWatched
      downloadProgress
    }
  }
`;

export const WANTED_EPISODES_QUERY = `
  query WantedEpisodes($libraryId: String) {
    wantedEpisodes(libraryId: $libraryId) {
      id
      tvShowId
      season
      episode
      title
      airDate
      mediaFileId
    }
  }
`;

// ============================================================================
// Quality Profile Queries
// ============================================================================

// ============================================================================
// Naming Pattern Queries
// ============================================================================

export const NAMING_PATTERNS_QUERY = `
  query NamingPatterns {
    NamingPatterns {
      Edges {
        Node {
          Id
          Name
          Pattern
          Description
          IsDefault
          IsSystem
          LibraryType
        }
      }
    }
  }
`;

// ============================================================================
// RSS Feed Queries
// ============================================================================

export const RSS_FEEDS_QUERY = `
  query RssFeeds($libraryId: String) {
    rssFeeds(libraryId: $libraryId) {
      id
      libraryId
      name
      url
      enabled
      pollIntervalMinutes
      lastPolledAt
      lastSuccessfulAt
      lastError
      consecutiveFailures
    }
  }
`;

// ============================================================================
// Parse and Identify Queries
// ============================================================================

export const PARSE_AND_IDENTIFY_QUERY = `
  query ParseAndIdentifyMedia($title: String!) {
    parseAndIdentifyMedia(title: $title) {
      parsed {
        originalTitle
        showName
        season
        episode
        year
        date
        resolution
        source
        codec
        hdr
        audio
        releaseGroup
        isProper
        isRepack
      }
      matches {
        provider
        providerId
        name
        year
        status
        network
        overview
        posterUrl
        tvdbId
        imdbId
        score
      }
    }
  }
`;

// ============================================================================
// Log Queries
// ============================================================================

export const LOGS_QUERY = `
  query Logs($filter: LogFilterInput, $orderBy: LogOrderByInput, $limit: Int!, $offset: Int!) {
    logs(filter: $filter, orderBy: $orderBy, limit: $limit, offset: $offset) {
      logs {
        id
        timestamp
        level
        target
        message
        fields
        spanName
      }
      totalCount
      hasMore
      nextCursor
    }
  }
`;

export const LOG_TARGETS_QUERY = `
  query LogTargets($limit: Int) {
    logTargets(limit: $limit)
  }
`;

export const LOG_STATS_QUERY = `
  query LogStats {
    logStats {
      traceCount
      debugCount
      infoCount
      warnCount
      errorCount
      totalCount
    }
  }
`;

// ============================================================================
// Upcoming Episode Queries (for home page)
// ============================================================================

export const UPCOMING_EPISODES_QUERY = `
  query UpcomingEpisodes($days: Int, $country: String) {
    upcomingEpisodes(days: $days, country: $country) {
      tvmazeId
      name
      season
      episode
      airDate
      airTime
      airStamp
      runtime
      summary
      episodeImageUrl
      show {
        tvmazeId
        name
        network
        posterUrl
        genres
      }
    }
  }
`;

export const LIBRARY_UPCOMING_EPISODES_QUERY = `
  query LibraryUpcomingEpisodes($days: Int) {
    libraryUpcomingEpisodes(days: $days) {
      id
      tvmazeId
      name
      season
      episode
      airDate
      status
      show {
        id
        name
        year
        network
        posterUrl
        libraryId
      }
    }
  }
`;

// ============================================================================
// Media File Queries
// ============================================================================

export const UNMATCHED_FILES_QUERY = `
  query UnmatchedFiles($libraryId: String!) {
    unmatchedFiles(libraryId: $libraryId) {
      id
      libraryId
      path
      relativePath
      originalName
      sizeBytes
      sizeFormatted
      container
      videoCodec
      audioCodec
      resolution
      isHdr
      hdrType
      width
      height
      duration
      bitrate
      episodeId
      organized
      addedAt
    }
  }
`;

export const UNMATCHED_FILES_COUNT_QUERY = `
  query UnmatchedFilesCount($libraryId: String!) {
    unmatchedFilesCount(libraryId: $libraryId)
  }
`;

export const MEDIA_FILE_BY_PATH_QUERY = `
  query MediaFileByPath($path: String!) {
    mediaFileByPath(path: $path) {
      id
      libraryId
      path
      sizeBytes
      sizeFormatted
      container
      videoCodec
      audioCodec
      resolution
      isHdr
      hdrType
      duration
      bitrate
      organized
      addedAt
    }
  }
`;

export const MOVIE_MEDIA_FILE_QUERY = `
  query MovieMediaFile($movieId: String!) {
    movieMediaFile(movieId: $movieId) {
      id
      libraryId
      path
      sizeBytes
      sizeFormatted
      container
      videoCodec
      audioCodec
      resolution
      isHdr
      hdrType
      duration
      bitrate
      organized
      addedAt
    }
  }
`;

export const MEDIA_FILE_DETAILS_QUERY = `
  query MediaFileDetails($mediaFileId: String!) {
    mediaFileDetails(mediaFileId: $mediaFileId) {
      id
      file {
        id
        libraryId
        path
        relativePath
        originalName
        sizeBytes
        sizeFormatted
        container
        videoCodec
        audioCodec
        resolution
        isHdr
        hdrType
        width
        height
        duration
        bitrate
        episodeId
        organized
        addedAt
      }
      videoStreams {
        id
        streamIndex
        codec
        codecLongName
        width
        height
        aspectRatio
        frameRate
        bitrate
        pixelFormat
        hdrType
        bitDepth
        language
        title
        isDefault
      }
      audioStreams {
        id
        streamIndex
        codec
        codecLongName
        channels
        channelLayout
        sampleRate
        bitrate
        bitDepth
        language
        title
        isDefault
        isCommentary
      }
      subtitles {
        id
        streamIndex
        sourceType
        codec
        codecLongName
        language
        title
        isDefault
        isForced
        isHearingImpaired
        filePath
      }
      chapters {
        id
        chapterIndex
        startSecs
        endSecs
        title
      }
      embeddedMetadata {
        artist
        album
        title
        trackNumber
        discNumber
        year
        genre
        showName
        season
        episode
        extracted
        coverArtBase64
        coverArtMime
        lyrics
      }
    }
  }
`;

// ============================================================================
// ============================================================================
// Cast Queries
// ============================================================================

export const CAST_DEVICES_QUERY = `
  query CastDevices {
    CastDevices {
      id
      name
      address
      port
      model
      deviceType
      isFavorite
      isManual
      isConnected
      lastSeenAt
    }
  }
`;

export const CAST_DEVICE_QUERY = `
  query CastDevice($id: ID!) {
    CastDevice(id: $id) {
      id
      name
      address
      port
      model
      deviceType
      isFavorite
      isManual
      isConnected
      lastSeenAt
    }
  }
`;

export const CAST_SESSIONS_QUERY = `
  query CastSessions {
    CastSessions {
      id
      deviceId
      deviceName
      mediaFileId
      episodeId
      streamUrl
      playerState
      currentTime
      duration
      volume
      isMuted
      startedAt
    }
  }
`;

export const CAST_SESSION_QUERY = `
  query CastSession($id: ID!) {
    CastSession(id: $id) {
      id
      deviceId
      deviceName
      mediaFileId
      episodeId
      streamUrl
      playerState
      currentTime
      duration
      volume
      isMuted
      startedAt
    }
  }
`;

export const CAST_SETTINGS_QUERY = `
  query CastSettings {
    CastSettings {
      autoDiscoveryEnabled
      discoveryIntervalSeconds
      defaultVolume
      transcodeIncompatible
      preferredQuality
    }
  }
`;

// ============================================================================
// Playback Session Queries
// ============================================================================

export const PLAYBACK_SESSION_QUERY = `
  query PlaybackSession {
    PlaybackSession {
      id
      userId
      contentType
      contentId
      mediaFileId
      episodeId
      movieId
      trackId
      audiobookId
      tvShowId
      albumId
      currentPosition
      duration
      volume
      isMuted
      isPlaying
      startedAt
      lastUpdatedAt
    }
  }
`;

export const PLAYBACK_SETTINGS_QUERY = `
  query PlaybackSettings {
    PlaybackSettings {
      syncIntervalSeconds
    }
  }
`;

// ============================================================================
// Filesystem Queries (BrowseDirectory uses codegen: see documents/filesystem.graphql)
// ============================================================================

export const QUICK_PATHS_QUERY = `
  query QuickPaths {
    quickPaths {
      name
      path
    }
  }
`;

export const VALIDATE_PATH_QUERY = `
  query ValidatePath($path: String!) {
    validatePath(path: $path) {
      isValid
      isLibraryPath
      libraryId
      libraryName
      error
    }
  }
`;

// ============================================================================
// Indexer Search
// ============================================================================

export const SEARCH_INDEXERS_QUERY = `
  query SearchIndexers($input: IndexerSearchInput!) {
    searchIndexers(input: $input) {
      indexers {
        indexerId
        indexerName
        releases {
          title
          guid
          link
          magnetUri
          infoHash
          details
          publishDate
          categories
          size
          sizeFormatted
          seeders
          leechers
          peers
          grabs
          isFreeleech
          imdbId
          poster
          description
          indexerId
          indexerName
        }
        elapsedMs
        fromCache
        error
      }
      totalReleases
      totalElapsedMs
    }
  }
`;

export const INDEXER_CONFIGS_QUERY = `
  query IndexerConfigs {
    indexerConfigs {
      id
      name
      indexerType
      enabled
      priority
      supportsSearch
      supportsRss
      apiUrl
      createdAt
      updatedAt
    }
  }
`;

// ============================================================================
// Notification Queries
// ============================================================================

export const NOTIFICATIONS_QUERY = `
  query Notifications($filter: NotificationFilterInput, $limit: Int, $offset: Int) {
    notifications(filter: $filter, limit: $limit, offset: $offset) {
      notifications {
        id
        title
        message
        notificationType
        category
        libraryId
        torrentId
        mediaFileId
        pendingMatchId
        actionType
        actionData
        readAt
        resolvedAt
        resolution
        createdAt
      }
      totalCount
      hasMore
    }
  }
`;

export const RECENT_NOTIFICATIONS_QUERY = `
  query RecentNotifications($limit: Int, $unreadOnly: Boolean) {
    recentNotifications(limit: $limit, unreadOnly: $unreadOnly) {
      id
      title
      message
      notificationType
      category
      libraryId
      torrentId
      mediaFileId
      pendingMatchId
      actionType
      actionData
      readAt
      resolvedAt
      resolution
      createdAt
    }
  }
`;

export const NOTIFICATION_COUNTS_QUERY = `
  query NotificationCounts {
    notificationCounts {
      unreadCount
      actionRequiredCount
    }
  }
`;

export const UNREAD_NOTIFICATION_COUNT_QUERY = `
  query UnreadNotificationCount {
    unreadNotificationCount
  }
`;
