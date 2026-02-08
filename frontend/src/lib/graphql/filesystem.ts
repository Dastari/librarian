/**
 * Filesystem GraphQL operations
 *
 * Uses codegen-generated documents and PascalCase types from the backend schema.
 */

import {
  BrowseDirectoryDocument,
  type BrowseDirectoryQuery,
} from './generated/graphql';
import {
  CREATE_DIRECTORY_MUTATION,
  DELETE_FILES_MUTATION,
  COPY_FILES_MUTATION,
  MOVE_FILES_MUTATION,
  RENAME_FILE_MUTATION,
} from './mutations';
import type {
  FileOperationResult,
  FileOperationPayloadPascal,
} from './types';

function fromPascal(p: FileOperationPayloadPascal): FileOperationResult {
  return {
    success: p.Success,
    error: p.Error,
    affectedCount: p.AffectedCount,
    messages: p.Messages ?? [],
    path: p.Path ?? null,
  };
}

/** Result shape from BrowseDirectory query (PascalCase). */
export type BrowseDirectoryResult = NonNullable<
  BrowseDirectoryQuery['BrowseDirectory']
>;
export type RuntimeFilesystemInfo = {
  Platform: string;
  SupportsUncCredentials: boolean;
  SupportsSambaMount: boolean;
  DefaultLinuxMountBase?: string | null;
};
export type LibraryPathAvailabilityStatus = {
  Path: string;
  Reachable: boolean;
  Exists: boolean;
  IsDirectory: boolean;
  NeedsReconnect: boolean;
  ReconnectAttempted: boolean;
  ReconnectSucceeded: boolean;
  Message?: string | null;
};

const FILESYSTEM_RUNTIME_INFO_QUERY = `
  query FilesystemRuntimeInfo {
    FilesystemRuntimeInfo {
      Platform
      SupportsUncCredentials
      SupportsSambaMount
      DefaultLinuxMountBase
    }
  }
`;

const LIBRARY_PATH_AVAILABILITY_QUERY = `
  query LibraryPathAvailability($Input: LibraryPathAvailabilityInput!) {
    LibraryPathAvailability(Input: $Input) {
      Path
      Reachable
      Exists
      IsDirectory
      NeedsReconnect
      ReconnectAttempted
      ReconnectSucceeded
      Message
    }
  }
`;

const CONFIGURE_NETWORK_PATH_MUTATION = `
  mutation ConfigureNetworkPath($Input: ConfigureNetworkPathInput!) {
    ConfigureNetworkPath(Input: $Input) {
      Success
      Error
      ResolvedPath
      Connected
      Stored
      Message
    }
  }
`;

const RECONNECT_LIBRARY_PATH_MUTATION = `
  mutation ReconnectLibraryPath($Path: String!) {
    ReconnectLibraryPath(Path: $Path) {
      Success
      Error
      ResolvedPath
      Connected
      Stored
      Message
    }
  }
`;

/**
 * Browse a directory on the server filesystem
 *
 * @param path - Path to browse (defaults to root)
 * @param dirsOnly - Only show directories (default: true)
 * @returns Browse result with CurrentPath, Entries, QuickPaths (PascalCase)
 */
export async function browseDirectory(
  path?: string,
  dirsOnly = true
): Promise<BrowseDirectoryResult> {
  const result = await queryPromise(BrowseDirectoryDocument, {
      Input: {
        Path: path ?? null,
        DirsOnly: dirsOnly,
        ShowHidden: false,
      },
    })
    ;

  if (result.error) {
    throw new Error(result.error.message);
  }

  const data = result.data?.BrowseDirectory;
  if (!data) {
    throw new Error('Failed to browse directory');
  }

  return data;
}

export async function getFilesystemRuntimeInfo(): Promise<RuntimeFilesystemInfo> {
  const result = await queryPromise<{ FilesystemRuntimeInfo: RuntimeFilesystemInfo }>(
    FILESYSTEM_RUNTIME_INFO_QUERY
  );

  if (result.error) {
    throw new Error(result.error.message);
  }

  const data = result.data?.FilesystemRuntimeInfo;
  if (!data) {
    throw new Error('Failed to load filesystem runtime info');
  }

  return data;
}

export async function getLibraryPathAvailability(
  paths: string[],
  attemptReconnect = false
): Promise<LibraryPathAvailabilityStatus[]> {
  const result = await queryPromise<{ LibraryPathAvailability: LibraryPathAvailabilityStatus[] }>(
    LIBRARY_PATH_AVAILABILITY_QUERY,
    {
    Input: {
      Paths: paths,
      AttemptReconnect: attemptReconnect,
    },
    }
  );

  if (result.error) {
    throw new Error(result.error.message);
  }

  return result.data?.LibraryPathAvailability ?? [];
}

export async function configureNetworkPath(input: {
  path: string;
  username?: string;
  password?: string;
  mountPoint?: string;
  persist?: boolean;
  attemptConnect?: boolean;
}): Promise<{
  success: boolean;
  error?: string;
  resolvedPath: string;
  connected: boolean;
  stored: boolean;
  message?: string;
}> {
  const result = await mutationPromise<{
    ConfigureNetworkPath: {
      Success: boolean;
      Error?: string | null;
      ResolvedPath: string;
      Connected: boolean;
      Stored: boolean;
      Message?: string | null;
    };
  }>(CONFIGURE_NETWORK_PATH_MUTATION, {
    Input: {
      Path: input.path,
      Username: input.username ?? null,
      Password: input.password ?? null,
      MountPoint: input.mountPoint ?? null,
      Persist: input.persist ?? true,
      AttemptConnect: input.attemptConnect ?? true,
    },
  });

  if (result.error || !result.data?.ConfigureNetworkPath) {
    return {
      success: false,
      error: result.error?.message ?? 'Failed to configure network path',
      resolvedPath: input.path,
      connected: false,
      stored: false,
    };
  }

  const payload = result.data.ConfigureNetworkPath;
  return {
    success: payload.Success,
    error: payload.Error ?? undefined,
    resolvedPath: payload.ResolvedPath,
    connected: payload.Connected,
    stored: payload.Stored,
    message: payload.Message ?? undefined,
  };
}

export async function reconnectLibraryPath(path: string): Promise<{
  success: boolean;
  error?: string;
}> {
  const result = await mutationPromise<{
    ReconnectLibraryPath: {
      Success: boolean;
      Error?: string | null;
    };
  }>(RECONNECT_LIBRARY_PATH_MUTATION, { Path: path });

  if (result.error || !result.data?.ReconnectLibraryPath) {
    return {
      success: false,
      error: result.error?.message ?? 'Failed to reconnect library path',
    };
  }

  return {
    success: result.data.ReconnectLibraryPath.Success,
    error: result.data.ReconnectLibraryPath.Error ?? undefined,
  };
}

/**
 * Create a directory on the server filesystem
 *
 * @param path - Full path of the directory to create
 * @returns Result with success status and created path
 */
export async function createDirectory(
  path: string
): Promise<{ success: boolean; path?: string; error?: string }> {
  const result = await mutationPromise<{ CreateDirectory: FileOperationPayloadPascal }>(CREATE_DIRECTORY_MUTATION, {
      Input: { Path: path },
    })
    ;

  if (result.error) {
    return {
      success: false,
      error: result.error.message,
    };
  }

  if (!result.data?.CreateDirectory) {
    return {
      success: false,
      error: 'Failed to create directory',
    };
  }

  const data = result.data.CreateDirectory;
  return {
    success: data.Success,
    path: data.Path ?? undefined,
    error: data.Error ?? undefined,
  };
}

/**
 * Delete files or directories
 *
 * @param paths - Array of paths to delete
 * @param recursive - Whether to recursively delete directories (default: true)
 * @returns Result with success status and affected count
 */
export async function deleteFiles(
  paths: string[],
  recursive = true
): Promise<FileOperationResult> {
  const result = await mutationPromise<{ DeleteFiles: FileOperationPayloadPascal }>(DELETE_FILES_MUTATION, {
      Input: { Paths: paths, Recursive: recursive },
    })
    ;

  if (result.error) {
    return {
      success: false,
      error: result.error.message,
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  if (!result.data?.DeleteFiles) {
    return {
      success: false,
      error: 'Failed to delete files',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPascal(result.data.DeleteFiles);
}

/**
 * Copy files or directories to a destination
 *
 * @param sources - Array of source paths to copy
 * @param destination - Destination directory path
 * @param overwrite - Whether to overwrite existing files (default: false)
 * @returns Result with success status and affected count
 */
export async function copyFiles(
  sources: string[],
  destination: string,
  overwrite = false
): Promise<FileOperationResult> {
  const result = await mutationPromise<{ CopyFiles: FileOperationPayloadPascal }>(COPY_FILES_MUTATION, {
      Input: { Sources: sources, Destination: destination, Overwrite: overwrite },
    })
    ;

  if (result.error) {
    return {
      success: false,
      error: result.error.message,
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  if (!result.data?.CopyFiles) {
    return {
      success: false,
      error: 'Failed to copy files',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPascal(result.data.CopyFiles);
}

/**
 * Move files or directories to a destination
 *
 * @param sources - Array of source paths to move
 * @param destination - Destination directory path
 * @param overwrite - Whether to overwrite existing files (default: false)
 * @returns Result with success status and affected count
 */
export async function moveFiles(
  sources: string[],
  destination: string,
  overwrite = false
): Promise<FileOperationResult> {
  const result = await mutationPromise<{ MoveFiles: FileOperationPayloadPascal }>(MOVE_FILES_MUTATION, {
      Input: { Sources: sources, Destination: destination, Overwrite: overwrite },
    })
    ;

  if (result.error) {
    return {
      success: false,
      error: result.error.message,
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  if (!result.data?.MoveFiles) {
    return {
      success: false,
      error: 'Failed to move files',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPascal(result.data.MoveFiles);
}

/**
 * Rename a file or directory
 *
 * @param path - Path to the file or directory to rename
 * @param newName - New name (not full path, just the name)
 * @returns Result with success status and new path
 */
export async function renameFile(
  path: string,
  newName: string
): Promise<FileOperationResult> {
  const result = await mutationPromise<{ RenameFile: FileOperationPayloadPascal }>(RENAME_FILE_MUTATION, {
      Input: { Path: path, NewName: newName },
    })
    ;

  if (result.error) {
    return {
      success: false,
      error: result.error.message,
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  if (!result.data?.RenameFile) {
    return {
      success: false,
      error: 'Failed to rename file',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPascal(result.data.RenameFile);
}
