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
