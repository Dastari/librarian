// ============================================================================
// Torrent Queries
// ============================================================================

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
      postDownloadAction
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
  query Library($id: String!) {
    library(id: $id) {
      id
      name
      path
      libraryType
      icon
      color
      autoScan
      scanIntervalMinutes
      watchForChanges
      postDownloadAction
      organizeFiles
      renameStyle
      namingPattern
      autoAddDiscovered
      autoDownload
      autoHunt
      scanning
      itemCount
      totalSizeBytes
      showCount
      movieCount
      lastScannedAt
      allowedResolutions
      allowedVideoCodecs
      allowedAudioFormats
      requireHdr
      allowedHdrTypes
      allowedSources
      releaseGroupBlacklist
      releaseGroupWhitelist
    }
  }
`;

// ============================================================================
// TV Show Queries
// ============================================================================

export const ALL_TV_SHOWS_QUERY = `
  query AllTvShows {
    allTvShows {
      id
      libraryId
      name
      sortName
      year
      status
      posterUrl
      monitored
    }
  }
`;

export const TV_SHOWS_QUERY = `
  query TvShows($libraryId: String!) {
    tvShows(libraryId: $libraryId) {
      id
      libraryId
      name
      sortName
      year
      status
      tvmazeId
      tmdbId
      tvdbId
      imdbId
      overview
      network
      runtime
      genres
      posterUrl
      backdropUrl
      monitored
      monitorType
      path
      episodeCount
      episodeFileCount
      sizeBytes
    }
  }
`;

export const TV_SHOWS_CONNECTION_QUERY = `
  query TvShowsConnection($libraryId: String!, $first: Int, $after: String, $where: TvShowWhereInput, $orderBy: TvShowOrderByInput) {
    tvShowsConnection(libraryId: $libraryId, first: $first, after: $after, where: $where, orderBy: $orderBy) {
      edges {
        node {
          id
          libraryId
          name
          sortName
          year
          status
          posterUrl
          backdropUrl
          monitored
          episodeCount
          episodeFileCount
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

export const TV_SHOW_QUERY = `
  query TvShow($id: String!) {
    tvShow(id: $id) {
      id
      libraryId
      name
      sortName
      year
      status
      tvmazeId
      tmdbId
      tvdbId
      imdbId
      overview
      network
      runtime
      genres
      posterUrl
      backdropUrl
      monitored
      monitorType
      path
      autoDownloadOverride
      backfillExisting
      organizeFilesOverride
      renameStyleOverride
      autoHuntOverride
      episodeCount
      episodeFileCount
      sizeBytes
      allowedResolutionsOverride
      allowedVideoCodecsOverride
      allowedAudioFormatsOverride
      requireHdrOverride
      allowedHdrTypesOverride
      allowedSourcesOverride
      releaseGroupBlacklistOverride
      releaseGroupWhitelistOverride
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
    allMovies {
      id
      libraryId
      title
      sortTitle
      year
      status
      posterUrl
      monitored
      hasFile
    }
  }
`;

export const MOVIES_QUERY = `
  query Movies($libraryId: String!) {
    movies(libraryId: $libraryId) {
      id
      libraryId
      title
      sortTitle
      originalTitle
      year
      tmdbId
      imdbId
      status
      overview
      tagline
      runtime
      genres
      director
      castNames
      posterUrl
      backdropUrl
      monitored
      hasFile
      sizeBytes
      path
      collectionId
      collectionName
      collectionPosterUrl
      tmdbRating
      tmdbVoteCount
      certification
      releaseDate
      allowedResolutionsOverride
      allowedVideoCodecsOverride
      allowedAudioFormatsOverride
      requireHdrOverride
      allowedHdrTypesOverride
      allowedSourcesOverride
      releaseGroupBlacklistOverride
      releaseGroupWhitelistOverride
    }
  }
`;

/** Movies query with cursor-based pagination and filtering */
export const MOVIES_CONNECTION_QUERY = `
  query MoviesConnection(
    $libraryId: String!
    $first: Int
    $after: String
    $where: MovieWhereInput
    $orderBy: MovieOrderByInput
  ) {
    moviesConnection(
      libraryId: $libraryId
      first: $first
      after: $after
      where: $where
      orderBy: $orderBy
    ) {
      edges {
        node {
          id
          libraryId
          title
          sortTitle
          originalTitle
          year
          tmdbId
          imdbId
          status
          overview
          runtime
          genres
          director
          posterUrl
          backdropUrl
          monitored
          hasFile
          sizeBytes
          path
          tmdbRating
          releaseDate
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

export const MOVIE_QUERY = `
  query Movie($id: String!) {
    movie(id: $id) {
      id
      libraryId
      title
      sortTitle
      originalTitle
      year
      tmdbId
      imdbId
      status
      overview
      tagline
      runtime
      genres
      director
      castNames
      posterUrl
      backdropUrl
      monitored
      hasFile
      sizeBytes
      path
      collectionId
      collectionName
      collectionPosterUrl
      tmdbRating
      tmdbVoteCount
      certification
      releaseDate
      allowedResolutionsOverride
      allowedVideoCodecsOverride
      allowedAudioFormatsOverride
      requireHdrOverride
      allowedHdrTypesOverride
      allowedSourcesOverride
      releaseGroupBlacklistOverride
      releaseGroupWhitelistOverride
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
        }
        hasFile
        filePath
        fileSize
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
  query SearchAlbums($query: String!) {
    searchAlbums(query: $query) {
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
      status
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
      status
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
    namingPatterns {
      id
      name
      pattern
      description
      isDefault
      isSystem
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
    }
  }
`;

// ============================================================================
// Security Settings Queries
// ============================================================================

export const SECURITY_SETTINGS_QUERY = `
  query SecuritySettings {
    securitySettings {
      encryptionKeySet
      encryptionKeyPreview
      encryptionKeyLastModified
    }
  }
`;

// ============================================================================
// Cast Queries
// ============================================================================

export const CAST_DEVICES_QUERY = `
  query CastDevices {
    castDevices {
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
    castDevice(id: $id) {
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
    castSessions {
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
    castSession(id: $id) {
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
    castSettings {
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
    playbackSession {
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
    playbackSettings {
      syncIntervalSeconds
    }
  }
`;

// ============================================================================
// Filesystem Queries
// ============================================================================

export const BROWSE_DIRECTORY_QUERY = `
  query BrowseDirectory($input: BrowseDirectoryInput) {
    browseDirectory(input: $input) {
      currentPath
      parentPath
      entries {
        name
        path
        isDir
        size
        sizeFormatted
        readable
        writable
        mimeType
        modifiedAt
      }
      quickPaths {
        name
        path
      }
      isLibraryPath
      libraryId
    }
  }
`;

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
