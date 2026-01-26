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

/**
 * Lightweight subscription for navbar badge
 *
 * Only emits when the count of active downloads changes.
 * Use this instead of TORRENT_PROGRESS_SUBSCRIPTION for the navbar.
 */
export const ACTIVE_DOWNLOAD_COUNT_SUBSCRIPTION = `
  subscription ActiveDownloadCount {
    activeDownloadCount {
      count
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
    LibraryChanged {
      Action
      Id
      Library {
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

// ============================================================================
// Notification Subscriptions
// ============================================================================

/**
 * Subscribe to notification events (new, read, resolved)
 * Use this to update the notification popover in real-time
 */
export const NOTIFICATION_RECEIVED_SUBSCRIPTION = `
  subscription NotificationReceived {
    notificationReceived {
      notification {
        id
        title
        message
        notificationType
        category
        libraryId
        actionType
        actionData
        readAt
        resolvedAt
        resolution
        createdAt
      }
      eventType
    }
  }
`;

/**
 * Subscribe to notification count updates
 * Use this for the navbar badge
 */
export const NOTIFICATION_COUNTS_SUBSCRIPTION = `
  subscription NotificationCounts {
    notificationCounts {
      unreadCount
      actionRequiredCount
    }
  }
`;

// ============================================================================
// Content Download Progress Subscriptions
// ============================================================================

/**
 * Subscribe to content download progress updates
 * Use this to show real-time download progress on content detail pages
 */
export const CONTENT_DOWNLOAD_PROGRESS_SUBSCRIPTION = `
  subscription ContentDownloadProgress($libraryId: String, $parentId: String) {
    contentDownloadProgress(libraryId: $libraryId, parentId: $parentId) {
      contentType
      contentId
      libraryId
      progress
      downloadSpeed
      contentName
      parentId
    }
  }
`;
