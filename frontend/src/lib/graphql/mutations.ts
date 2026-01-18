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
  mutation CreateLibrary($input: CreateLibraryInput!) {
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
  mutation UpdateLibrary($id: String!, $input: UpdateLibraryInput!) {
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
        allowedResolutionsOverride
        allowedVideoCodecsOverride
        allowedAudioFormatsOverride
        requireHdrOverride
        allowedHdrTypesOverride
        allowedSourcesOverride
        releaseGroupBlacklistOverride
        releaseGroupWhitelistOverride
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

// ============================================================================
// Security Settings Mutations
// ============================================================================

export const INITIALIZE_ENCRYPTION_KEY_MUTATION = `
  mutation InitializeEncryptionKey {
    initializeEncryptionKey {
      success
      error
      settings {
        encryptionKeySet
        encryptionKeyPreview
        encryptionKeyLastModified
      }
    }
  }
`;

export const REGENERATE_ENCRYPTION_KEY_MUTATION = `
  mutation RegenerateEncryptionKey($input: GenerateEncryptionKeyInput!) {
    regenerateEncryptionKey(input: $input) {
      success
      error
      settings {
        encryptionKeySet
        encryptionKeyPreview
        encryptionKeyLastModified
      }
    }
  }
`;

// ============================================================================
// Cast Mutations
// ============================================================================

export const DISCOVER_CAST_DEVICES_MUTATION = `
  mutation DiscoverCastDevices {
    discoverCastDevices {
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

export const ADD_CAST_DEVICE_MUTATION = `
  mutation AddCastDevice($input: AddCastDeviceInput!) {
    addCastDevice(input: $input) {
      success
      device {
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
      error
    }
  }
`;

export const UPDATE_CAST_DEVICE_MUTATION = `
  mutation UpdateCastDevice($id: ID!, $input: UpdateCastDeviceInput!) {
    updateCastDevice(id: $id, input: $input) {
      success
      device {
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
      error
    }
  }
`;

export const REMOVE_CAST_DEVICE_MUTATION = `
  mutation RemoveCastDevice($id: ID!) {
    removeCastDevice(id: $id) {
      success
      error
    }
  }
`;

export const CAST_MEDIA_MUTATION = `
  mutation CastMedia($input: CastMediaInput!) {
    castMedia(input: $input) {
      success
      session {
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
      error
    }
  }
`;

export const CAST_PLAY_MUTATION = `
  mutation CastPlay($sessionId: ID!) {
    castPlay(sessionId: $sessionId) {
      success
      session {
        id
        playerState
        currentTime
      }
      error
    }
  }
`;

export const CAST_PAUSE_MUTATION = `
  mutation CastPause($sessionId: ID!) {
    castPause(sessionId: $sessionId) {
      success
      session {
        id
        playerState
        currentTime
      }
      error
    }
  }
`;

export const CAST_STOP_MUTATION = `
  mutation CastStop($sessionId: ID!) {
    castStop(sessionId: $sessionId) {
      success
      error
    }
  }
`;

export const CAST_SEEK_MUTATION = `
  mutation CastSeek($sessionId: ID!, $position: Float!) {
    castSeek(sessionId: $sessionId, position: $position) {
      success
      session {
        id
        playerState
        currentTime
      }
      error
    }
  }
`;

export const CAST_SET_VOLUME_MUTATION = `
  mutation CastSetVolume($sessionId: ID!, $volume: Float!) {
    castSetVolume(sessionId: $sessionId, volume: $volume) {
      success
      session {
        id
        volume
        isMuted
      }
      error
    }
  }
`;

export const CAST_SET_MUTED_MUTATION = `
  mutation CastSetMuted($sessionId: ID!, $muted: Boolean!) {
    castSetMuted(sessionId: $sessionId, muted: $muted) {
      success
      session {
        id
        volume
        isMuted
      }
      error
    }
  }
`;

export const UPDATE_CAST_SETTINGS_MUTATION = `
  mutation UpdateCastSettings($input: UpdateCastSettingsInput!) {
    updateCastSettings(input: $input) {
      success
      settings {
        autoDiscoveryEnabled
        discoveryIntervalSeconds
        defaultVolume
        transcodeIncompatible
        preferredQuality
      }
      error
    }
  }
`;

// ============================================================================
// Filesystem Mutations
// ============================================================================

export const CREATE_DIRECTORY_MUTATION = `
  mutation CreateDirectory($input: CreateDirectoryInput!) {
    createDirectory(input: $input) {
      success
      error
      affectedCount
      messages
      path
    }
  }
`;

export const DELETE_FILES_MUTATION = `
  mutation DeleteFiles($input: DeleteFilesInput!) {
    deleteFiles(input: $input) {
      success
      error
      affectedCount
      messages
      path
    }
  }
`;

export const COPY_FILES_MUTATION = `
  mutation CopyFiles($input: CopyFilesInput!) {
    copyFiles(input: $input) {
      success
      error
      affectedCount
      messages
      path
    }
  }
`;

export const MOVE_FILES_MUTATION = `
  mutation MoveFiles($input: MoveFilesInput!) {
    moveFiles(input: $input) {
      success
      error
      affectedCount
      messages
      path
    }
  }
`;

export const RENAME_FILE_MUTATION = `
  mutation RenameFile($input: RenameFileInput!) {
    renameFile(input: $input) {
      success
      error
      affectedCount
      messages
      path
    }
  }
`;
