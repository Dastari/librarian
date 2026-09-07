/**
 * Filesystem GraphQL operations
 *
 * Uses codegen-generated documents from the backend schema.
 */

import {
  BrowseDirectoryDocument,
  ConfigureNetworkPathDocument,
  CopyFilesDocument,
  CreateDirectoryDocument,
  DeleteFilesDocument,
  FilesystemRuntimeInfoDocument,
  LibraryPathAvailabilityDocument,
  MoveFilesDocument,
  ReconnectLibraryPathDocument,
  RenameFileDocument,
  type BrowseDirectoryQuery,
  type CopyFilesMutation,
  type CreateDirectoryMutation,
  type DeleteFilesMutation,
  type MoveFilesMutation,
  type RenameFileMutation,
} from './generated/graphql';
import { mutationPromise, queryPromise } from './client';
import type { FileOperationResult } from './types';

type FileOperationPayload =
  | CreateDirectoryMutation['createDirectory']
  | DeleteFilesMutation['deleteFiles']
  | CopyFilesMutation['copyFiles']
  | MoveFilesMutation['moveFiles']
  | RenameFileMutation['renameFile'];

function fromPayload(p: FileOperationPayload): FileOperationResult {
  return {
    success: p.success,
    error: p.error,
    affectedCount: p.affectedCount,
    messages: p.messages ?? [],
    path: p.path ?? null,
  };
}

/** Result shape from BrowseDirectory query. */
export type BrowseDirectoryResult = NonNullable<
  BrowseDirectoryQuery['browseDirectory']
>;
export type RuntimeFilesystemInfo = {
  platform: string;
  supportsUncCredentials: boolean;
  supportsSambaMount: boolean;
  defaultLinuxMountBase?: string | null;
};
export type LibraryPathAvailabilityStatus = {
  path: string;
  reachable: boolean;
  exists: boolean;
  isDirectory: boolean;
  needsReconnect: boolean;
  reconnectAttempted: boolean;
  reconnectSucceeded: boolean;
  message?: string | null;
};

/**
 * Browse a directory on the server filesystem
 *
 * @param path - Path to browse (defaults to root)
 * @param dirsOnly - Only show directories (default: true)
 * @returns Browse result with currentPath, entries, quickPaths
 */
export async function browseDirectory(
  path?: string,
  dirsOnly = true
): Promise<BrowseDirectoryResult> {
  const result = await queryPromise(BrowseDirectoryDocument, {
      input: {
        path: path ?? null,
        dirsOnly: dirsOnly,
        showHidden: false,
      },
    })
    ;

  if (result.error) {
    throw new Error(result.error.message);
  }

  const data = result.data?.browseDirectory;
  if (!data) {
    throw new Error('Failed to browse directory');
  }

  return data;
}

export async function getFilesystemRuntimeInfo(): Promise<RuntimeFilesystemInfo> {
  const result = await queryPromise<{ filesystemRuntimeInfo: RuntimeFilesystemInfo }>(
    FilesystemRuntimeInfoDocument
  );

  if (result.error) {
    throw new Error(result.error.message);
  }

  const data = result.data?.filesystemRuntimeInfo;
  if (!data) {
    throw new Error('Failed to load filesystem runtime info');
  }

  return data;
}

export async function getLibraryPathAvailability(
  paths: string[],
  attemptReconnect = false
): Promise<LibraryPathAvailabilityStatus[]> {
  const result = await queryPromise<{ libraryPathAvailability: LibraryPathAvailabilityStatus[] }>(
    LibraryPathAvailabilityDocument,
    {
    input: {
      paths: paths,
      attemptReconnect: attemptReconnect,
    },
    }
  );

  if (result.error) {
    throw new Error(result.error.message);
  }

  return result.data?.libraryPathAvailability ?? [];
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
    configureNetworkPath: {
      success: boolean;
      error?: string | null;
      resolvedPath: string;
      connected: boolean;
      stored: boolean;
      message?: string | null;
    };
  }>(ConfigureNetworkPathDocument, {
    input: {
      path: input.path,
      username: input.username ?? null,
      password: input.password ?? null,
      mountPoint: input.mountPoint ?? null,
      persist: input.persist ?? true,
      attemptConnect: input.attemptConnect ?? true,
    },
  });

  if (result.error || !result.data?.configureNetworkPath) {
    return {
      success: false,
      error: result.error?.message ?? 'Failed to configure network path',
      resolvedPath: input.path,
      connected: false,
      stored: false,
    };
  }

  const payload = result.data.configureNetworkPath;
  return {
    success: payload.success,
    error: payload.error ?? undefined,
    resolvedPath: payload.resolvedPath,
    connected: payload.connected,
    stored: payload.stored,
    message: payload.message ?? undefined,
  };
}

export async function reconnectLibraryPath(path: string): Promise<{
  success: boolean;
  error?: string;
}> {
  const result = await mutationPromise<{
    reconnectLibraryPath: {
      success: boolean;
      error?: string | null;
    };
  }>(ReconnectLibraryPathDocument, { path: path });

  if (result.error || !result.data?.reconnectLibraryPath) {
    return {
      success: false,
      error: result.error?.message ?? 'Failed to reconnect library path',
    };
  }

  return {
    success: result.data.reconnectLibraryPath.success,
    error: result.data.reconnectLibraryPath.error ?? undefined,
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
  const result = await mutationPromise<CreateDirectoryMutation>(CreateDirectoryDocument, {
      input: { path: path },
    })
    ;

  if (result.error) {
    return {
      success: false,
      error: result.error.message,
    };
  }

  if (!result.data?.createDirectory) {
    return {
      success: false,
      error: 'Failed to create directory',
    };
  }

  const data = result.data.createDirectory;
  return {
    success: data.success,
    path: data.path ?? undefined,
    error: data.error ?? undefined,
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
  const result = await mutationPromise<DeleteFilesMutation>(DeleteFilesDocument, {
      input: { paths: paths, recursive: recursive },
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

  if (!result.data?.deleteFiles) {
    return {
      success: false,
      error: 'Failed to delete files',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPayload(result.data.deleteFiles);
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
  const result = await mutationPromise<CopyFilesMutation>(CopyFilesDocument, {
      input: { sources: sources, destination: destination, overwrite: overwrite },
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

  if (!result.data?.copyFiles) {
    return {
      success: false,
      error: 'Failed to copy files',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPayload(result.data.copyFiles);
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
  const result = await mutationPromise<MoveFilesMutation>(MoveFilesDocument, {
      input: { sources: sources, destination: destination, overwrite: overwrite },
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

  if (!result.data?.moveFiles) {
    return {
      success: false,
      error: 'Failed to move files',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPayload(result.data.moveFiles);
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
  const result = await mutationPromise<RenameFileMutation>(RenameFileDocument, {
      input: { path: path, newName: newName },
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

  if (!result.data?.renameFile) {
    return {
      success: false,
      error: 'Failed to rename file',
      affectedCount: 0,
      messages: [],
      path: null,
    };
  }

  return fromPayload(result.data.renameFile);
}
