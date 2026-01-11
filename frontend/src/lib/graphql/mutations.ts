// ============================================================================
// Torrent Mutations
// ============================================================================

export const ADD_TORRENT_MUTATION = `
  mutation AddTorrent($input: AddTorrentInput!) {
    addTorrent(input: $input) {
      success
      torrent {
        id
        infoHash
        name
        state
        progress
        progressPercent
        size
        sizeFormatted
      }
      error
    }
  }
`;

export const PAUSE_TORRENT_MUTATION = `
  mutation PauseTorrent($id: Int!) {
    pauseTorrent(id: $id) {
      success
      error
    }
  }
`;

export const RESUME_TORRENT_MUTATION = `
  mutation ResumeTorrent($id: Int!) {
    resumeTorrent(id: $id) {
      success
      error
    }
  }
`;

export const REMOVE_TORRENT_MUTATION = `
  mutation RemoveTorrent($id: Int!, $deleteFiles: Boolean!) {
    removeTorrent(id: $id, deleteFiles: $deleteFiles) {
      success
      error
    }
  }
`;

export const ORGANIZE_TORRENT_MUTATION = `
  mutation OrganizeTorrent($id: Int!, $libraryId: String) {
    organizeTorrent(id: $id, libraryId: $libraryId) {
      success
      organizedCount
      failedCount
      messages
    }
  }
`;

// ============================================================================
// Settings Mutations
// ============================================================================

export const UPDATE_TORRENT_SETTINGS_MUTATION = `
  mutation UpdateTorrentSettings($input: UpdateTorrentSettingsInput!) {
    updateTorrentSettings(input: $input) {
      success
      error
    }
  }
`;

// ============================================================================
// Library Mutations
// ============================================================================

export const CREATE_LIBRARY_MUTATION = `
  mutation CreateLibrary($input: CreateLibraryFullInput!) {
    createLibrary(input: $input) {
      success
      library {
        id
        name
        path
        libraryType
        icon
        color
      }
      error
    }
  }
`;

export const UPDATE_LIBRARY_MUTATION = `
  mutation UpdateLibrary($id: String!, $input: UpdateLibraryFullInput!) {
    updateLibrary(id: $id, input: $input) {
      success
      library {
        id
        name
        path
      }
      error
    }
  }
`;

export const DELETE_LIBRARY_MUTATION = `
  mutation DeleteLibrary($id: String!) {
    deleteLibrary(id: $id) {
      success
      error
    }
  }
`;

export const SCAN_LIBRARY_MUTATION = `
  mutation ScanLibrary($id: String!) {
    scanLibrary(id: $id) {
      libraryId
      status
      message
    }
  }
`;

// ============================================================================
// TV Show Mutations
// ============================================================================

export const ADD_TV_SHOW_MUTATION = `
  mutation AddTvShow($libraryId: String!, $input: AddTvShowInput!) {
    addTvShow(libraryId: $libraryId, input: $input) {
      success
      tvShow {
        id
        name
        posterUrl
      }
      error
    }
  }
`;

export const DELETE_TV_SHOW_MUTATION = `
  mutation DeleteTvShow($id: String!) {
    deleteTvShow(id: $id) {
      success
      error
    }
  }
`;

export const REFRESH_TV_SHOW_MUTATION = `
  mutation RefreshTvShow($id: String!) {
    refreshTvShow(id: $id) {
      success
      tvShow {
        id
        episodeCount
      }
      error
    }
  }
`;

// ============================================================================
// RSS Feed Mutations
// ============================================================================

export const CREATE_RSS_FEED_MUTATION = `
  mutation CreateRssFeed($input: CreateRssFeedInput!) {
    createRssFeed(input: $input) {
      success
      rssFeed {
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
      error
    }
  }
`;

export const UPDATE_RSS_FEED_MUTATION = `
  mutation UpdateRssFeed($id: String!, $input: UpdateRssFeedInput!) {
    updateRssFeed(id: $id, input: $input) {
      success
      rssFeed {
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
      error
    }
  }
`;

export const DELETE_RSS_FEED_MUTATION = `
  mutation DeleteRssFeed($id: String!) {
    deleteRssFeed(id: $id) {
      success
      error
    }
  }
`;

export const TEST_RSS_FEED_MUTATION = `
  mutation TestRssFeed($url: String!) {
    testRssFeed(url: $url) {
      success
      itemCount
      sampleItems {
        title
        link
        pubDate
        description
        parsedShowName
        parsedSeason
        parsedEpisode
        parsedResolution
        parsedCodec
      }
      error
    }
  }
`;

export const POLL_RSS_FEED_MUTATION = `
  mutation PollRssFeed($id: String!) {
    pollRssFeed(id: $id) {
      success
      rssFeed {
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
      error
    }
  }
`;

// ============================================================================
// TV Show Mutations
// ============================================================================

export const UPDATE_TV_SHOW_MUTATION = `
  mutation UpdateTvShow($id: String!, $input: UpdateTvShowInput!) {
    updateTvShow(id: $id, input: $input) {
      success
      tvShow {
        id
        libraryId
        name
        sortName
        year
        status
        monitored
        monitorType
        qualityProfileId
        path
        autoDownloadOverride
        backfillExisting
        organizeFilesOverride
        renameStyleOverride
        episodeCount
        episodeFileCount
        sizeBytes
      }
      error
    }
  }
`;

// ============================================================================
// Episode Mutations
// ============================================================================

export const DOWNLOAD_EPISODE_MUTATION = `
  mutation DownloadEpisode($episodeId: String!) {
    downloadEpisode(episodeId: $episodeId) {
      success
      episode {
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
        torrentLink
        torrentLinkAddedAt
      }
      error
    }
  }
`;

// ============================================================================
// Log Mutations
// ============================================================================

export const CLEAR_ALL_LOGS_MUTATION = `
  mutation ClearAllLogs {
    clearAllLogs {
      success
      deletedCount
      error
    }
  }
`;

export const CLEAR_OLD_LOGS_MUTATION = `
  mutation ClearOldLogs($days: Int!) {
    clearOldLogs(days: $days) {
      success
      deletedCount
      error
    }
  }
`;
