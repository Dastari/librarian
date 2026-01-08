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
      autoRename
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
      autoRename
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
      episodeCount
      episodeFileCount
      sizeBytes
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
