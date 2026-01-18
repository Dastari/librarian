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
      defaultQualityProfileId
      autoAddDiscovered
      itemCount
      totalSizeBytes
      showCount
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
      defaultQualityProfileId
      autoAddDiscovered
      autoDownload
      autoHunt
      scanning
      itemCount
      totalSizeBytes
      showCount
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
      qualityProfileId
      path
      episodeCount
      episodeFileCount
      sizeBytes
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
      qualityProfileId
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

export const QUALITY_PROFILES_QUERY = `
  query QualityProfiles {
    qualityProfiles {
      id
      name
      preferredResolution
      minResolution
      preferredCodec
      preferredAudio
      requireHdr
      hdrTypes
      preferredLanguage
      maxSizeGb
      minSeeders
      releaseGroupWhitelist
      releaseGroupBlacklist
      upgradeUntil
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
  query Logs($filter: LogFilterInput, $limit: Int!, $offset: Int!) {
    logs(filter: $filter, limit: $limit, offset: $offset) {
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
