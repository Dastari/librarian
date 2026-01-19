// ============================================================================
// Torrent Subscriptions
// ============================================================================

export const TORRENT_PROGRESS_SUBSCRIPTION = `
  subscription TorrentProgress {
    torrentProgress {
      id
      infoHash
      progress
      downloadSpeed
      uploadSpeed
      peers
      state
    }
  }
`;

export const TORRENT_ADDED_SUBSCRIPTION = `
  subscription TorrentAdded {
    torrentAdded {
      id
      name
      infoHash
    }
  }
`;

export const TORRENT_COMPLETED_SUBSCRIPTION = `
  subscription TorrentCompleted {
    torrentCompleted {
      id
      name
      infoHash
    }
  }
`;

export const TORRENT_REMOVED_SUBSCRIPTION = `
  subscription TorrentRemoved {
    torrentRemoved {
      id
      infoHash
    }
  }
`;

// ============================================================================
// Log Subscriptions
// ============================================================================

export const LOG_EVENTS_SUBSCRIPTION = `
  subscription LogEvents($levels: [LogLevel!]) {
    logEvents(levels: $levels) {
      timestamp
      level
      target
      message
      fields
      spanName
    }
  }
`;

export const ERROR_LOGS_SUBSCRIPTION = `
  subscription ErrorLogs {
    errorLogs {
      timestamp
      level
      target
      message
      fields
      spanName
    }
  }
`;

// ============================================================================
// Library Subscriptions
// ============================================================================

export const LIBRARY_CHANGED_SUBSCRIPTION = `
  subscription LibraryChanged {
    libraryChanged {
      changeType
      libraryId
      libraryName
      library {
        id
        name
        path
        libraryType
        icon
        color
        autoScan
        scanIntervalHours
        itemCount
        totalSizeBytes
        lastScannedAt
        scanning
      }
    }
  }
`;

// ============================================================================
// Media File Subscriptions
// ============================================================================

export const MEDIA_FILE_UPDATED_SUBSCRIPTION = `
  subscription MediaFileUpdated($libraryId: String, $episodeId: String) {
    mediaFileUpdated(libraryId: $libraryId, episodeId: $episodeId) {
      mediaFileId
      libraryId
      episodeId
      movieId
      resolution
      videoCodec
      audioCodec
      audioChannels
      isHdr
      hdrType
      duration
    }
  }
`;

// ============================================================================
// Filesystem Subscriptions
// ============================================================================

export const DIRECTORY_CONTENTS_CHANGED_SUBSCRIPTION = `
  subscription DirectoryContentsChanged($path: String) {
    directoryContentsChanged(path: $path) {
      path
      changeType
      name
      newName
      timestamp
    }
  }
`;
