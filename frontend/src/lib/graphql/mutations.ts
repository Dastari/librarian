// ============================================================================
// Auth: use generated documents from ./generated/graphql (LoginDocument, etc.)
// ============================================================================

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
  mutation OrganizeTorrent($id: Int!, $libraryId: String, $albumId: String) {
    organizeTorrent(id: $id, libraryId: $libraryId, albumId: $albumId) {
      success
      organizedCount
      failedCount
      messages
    }
  }
`;

// ============================================================================
// File Match Mutations (Source-Agnostic)
// ============================================================================

/** Re-match all files from a source (torrent, usenet, etc.) */
export const REMATCH_SOURCE_MUTATION = `
  mutation RematchSource($sourceType: String!, $sourceId: String!, $libraryId: String) {
    rematchSource(sourceType: $sourceType, sourceId: $sourceId, libraryId: $libraryId) {
      success
      matchCount
      error
    }
  }
`;

/** Process all pending matches for a source (copy files to library) */
export const PROCESS_SOURCE_MUTATION = `
  mutation ProcessSource($sourceType: String!, $sourceId: String!) {
    processSource(sourceType: $sourceType, sourceId: $sourceId) {
      success
      filesProcessed
      filesFailed
      messages
      error
    }
  }
`;

/** Manually set a match target for a pending file match */
export const SET_MATCH_MUTATION = `
  mutation SetMatch($matchId: String!, $targetType: String!, $targetId: String!) {
    setMatch(matchId: $matchId, targetType: $targetType, targetId: $targetId) {
      success
      error
    }
  }
`;

/** Remove a specific pending file match */
export const REMOVE_MATCH_MUTATION = `
  mutation RemoveMatch($matchId: String!) {
    removeMatch(matchId: $matchId) {
      success
      error
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

export const ATTEMPT_UPNP_PORT_FORWARDING_MUTATION = `
  mutation AttemptUpnpPortForwarding {
    attemptUpnpPortForwarding {
      success
      tcpForwarded
      udpForwarded
      localIp
      externalIp
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
  mutation UpdateLibrary($Id: String!, $Input: UpdateLibraryInput!) {
    UpdateLibrary(Id: $Id, Input: $Input) {
      Success
      Library {
        Id
        Name
        Path
      }
      Error
    }
  }
`;

export const DELETE_LIBRARY_MUTATION = `
  mutation DeleteLibrary($Id: String!) {
    DeleteLibrary(Id: $Id) {
      Success
      Error
    }
  }
`;

export const SCAN_LIBRARY_MUTATION = `
  mutation ScanLibrary($Id: String!) {
    ScanLibrary(Id: $Id) {
      LibraryId
      Status
      Message
    }
  }
`;

export const CONSOLIDATE_LIBRARY_MUTATION = `
  mutation ConsolidateLibrary($id: String!) {
    consolidateLibrary(id: $id) {
      success
      foldersRemoved
      filesMoved
      messages
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

export const REFRESH_MOVIE_MUTATION = `
  mutation RefreshMovie($id: String!) {
    refreshMovie(id: $id) {
      success
      movie {
        id
        posterUrl
        backdropUrl
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
        path
        autoDownloadOverride
        autoHuntOverride
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
// Movie Mutations
// ============================================================================

export const ADD_MOVIE_MUTATION = `
  mutation AddMovie($libraryId: String!, $input: AddMovieInput!) {
    addMovie(libraryId: $libraryId, input: $input) {
      success
      movie {
        id
        libraryId
        title
        year
        tmdbId
        imdbId
        status
        overview
        posterUrl
        backdropUrl
        monitored
        mediaFileId
      }
      error
    }
  }
`;

export const UPDATE_MOVIE_MUTATION = `
  mutation UpdateMovie($id: String!, $input: UpdateMovieInput!) {
    updateMovie(id: $id, input: $input) {
      success
      movie {
        id
        libraryId
        title
        year
        status
        monitored
        mediaFileId
      }
      error
    }
  }
`;

export const DELETE_MOVIE_MUTATION = `
  mutation DeleteMovie($id: String!) {
    deleteMovie(id: $id) {
      success
      error
    }
  }
`;

// ============================================================================
// Album Mutations
// ============================================================================

export const ADD_ALBUM_MUTATION = `
  mutation AddAlbum($input: AddAlbumInput!) {
    addAlbum(input: $input) {
      success
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
        coverUrl
        hasFiles
      }
      error
    }
  }
`;

export const DELETE_ALBUM_MUTATION = `
  mutation DeleteAlbum($id: String!) {
    deleteAlbum(id: $id) {
      success
      error
    }
  }
`;

// ============================================================================
// Audiobook Mutations
// ============================================================================

export const ADD_AUDIOBOOK_MUTATION = `
  mutation AddAudiobook($input: AddAudiobookInput!) {
    addAudiobook(input: $input) {
      success
      audiobook {
        id
        authorId
        libraryId
        title
        sortTitle
        coverUrl
        hasFiles
      }
      error
    }
  }
`;

export const DELETE_AUDIOBOOK_MUTATION = `
  mutation DeleteAudiobook($id: String!) {
    deleteAudiobook(id: $id) {
      success
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
        tvmazeId
        tmdbId
        tvdbId
        mediaFileId
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
// Playback Session Mutations
// ============================================================================

export const START_PLAYBACK_MUTATION = `
  mutation StartPlayback($input: StartPlaybackInput!) {
    startPlayback(input: $input) {
      success
      session {
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
      error
    }
  }
`;

export const UPDATE_PLAYBACK_MUTATION = `
  mutation UpdatePlayback($input: UpdatePlaybackInput!) {
    updatePlayback(input: $input) {
      success
      session {
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
      error
    }
  }
`;

export const STOP_PLAYBACK_MUTATION = `
  mutation StopPlayback {
    stopPlayback {
      success
      session {
        id
      }
      error
    }
  }
`;

export const UPDATE_PLAYBACK_SETTINGS_MUTATION = `
  mutation UpdatePlaybackSettings($input: UpdatePlaybackSettingsInput!) {
    updatePlaybackSettings(input: $input) {
      syncIntervalSeconds
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

// ============================================================================
// Naming Pattern Mutations
// ============================================================================

export const CREATE_NAMING_PATTERN_MUTATION = `
  mutation CreateNamingPattern($Input: CreateNamingPatternInput!) {
    CreateNamingPattern(Input: $Input) {
      Success
      NamingPattern {
        Id
        Name
        Pattern
        Description
        IsDefault
        IsSystem
        LibraryType
      }
      Error
    }
  }
`;

export const UPDATE_NAMING_PATTERN_MUTATION = `
  mutation UpdateNamingPattern($Id: String!, $Input: UpdateNamingPatternInput!) {
    UpdateNamingPattern(Id: $Id, Input: $Input) {
      Success
      NamingPattern {
        Id
        Name
        Pattern
        Description
        IsDefault
        IsSystem
        LibraryType
      }
      Error
    }
  }
`;

export const DELETE_NAMING_PATTERN_MUTATION = `
  mutation DeleteNamingPattern($Id: String!) {
    DeleteNamingPattern(Id: $Id) {
      Success
      Error
    }
  }
`;

export const SET_DEFAULT_NAMING_PATTERN_MUTATION = `
  mutation SetDefaultNamingPattern($Id: String!) {
    SetDefaultNamingPattern(Id: $Id) {
      Success
      Error
    }
  }
`;

// ============================================================================
// Auto-Hunt Mutations
// ============================================================================

export const TRIGGER_AUTO_HUNT_MUTATION = `
  mutation TriggerAutoHunt($libraryId: String!) {
    triggerAutoHunt(libraryId: $libraryId) {
      success
      error
      searched
      matched
      downloaded
      skipped
      failed
    }
  }
`;

// ============================================================================
// Notification Mutations
// ============================================================================

export const MARK_NOTIFICATION_READ_MUTATION = `
  mutation MarkNotificationRead($id: String!) {
    markNotificationRead(id: $id) {
      success
      error
      notification {
        id
        readAt
      }
    }
  }
`;

export const MARK_ALL_NOTIFICATIONS_READ_MUTATION = `
  mutation MarkAllNotificationsRead {
    markAllNotificationsRead {
      success
      count
      error
    }
  }
`;

export const RESOLVE_NOTIFICATION_MUTATION = `
  mutation ResolveNotification($input: ResolveNotificationInput!) {
    resolveNotification(input: $input) {
      success
      error
      notification {
        id
        resolvedAt
        resolution
      }
    }
  }
`;

export const RESOLVE_NOTIFICATION_WITH_ACTION_MUTATION = `
  mutation ResolveNotificationWithAction($id: String!, $resolution: NotificationResolution!, $actionPerformed: String, $actionResult: String) {
    resolveNotificationWithAction(id: $id, resolution: $resolution, actionPerformed: $actionPerformed, actionResult: $actionResult) {
      success
      error
      notification {
        id
        resolvedAt
        resolution
      }
    }
  }
`;

export const DELETE_NOTIFICATION_MUTATION = `
  mutation DeleteNotification($id: String!) {
    deleteNotification(id: $id) {
      success
      error
    }
  }
`;

// ============================================================================
// Manual Match Mutations
// ============================================================================

export const MANUAL_MATCH_MUTATION = `
  mutation ManualMatch(
    $mediaFileId: String!
    $episodeId: String
    $movieId: String
    $trackId: String
    $albumId: String
    $audiobookId: String
    $chapterId: String
  ) {
    manualMatch(
      mediaFileId: $mediaFileId
      episodeId: $episodeId
      movieId: $movieId
      trackId: $trackId
      albumId: $albumId
      audiobookId: $audiobookId
      chapterId: $chapterId
    ) {
      success
      error
      mediaFile {
        id
        matchType
        isManualMatch
        contentType
        episodeId
        movieId
        trackId
        albumId
        audiobookId
        chapterId
        matchedAt
      }
    }
  }
`;

export const UNMATCH_MEDIA_FILE_MUTATION = `
  mutation UnmatchMediaFile($mediaFileId: String!) {
    unmatchMediaFile(mediaFileId: $mediaFileId) {
      success
      error
      mediaFile {
        id
        matchType
        isManualMatch
        contentType
        episodeId
        movieId
        trackId
        albumId
        audiobookId
        chapterId
        matchedAt
      }
    }
  }
`;
