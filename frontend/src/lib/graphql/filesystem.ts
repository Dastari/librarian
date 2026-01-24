/**
 * Filesystem GraphQL operations
 *
 * These functions provide GraphQL-based filesystem operations,
 * replacing the old REST API endpoints.
 */

import { graphqlClient } from './client';
import { BROWSE_DIRECTORY_QUERY } from './queries';
import {
  CREATE_DIRECTORY_MUTATION,
  DELETE_FILES_MUTATION,
  COPY_FILES_MUTATION,
  MOVE_FILES_MUTATION,
  RENAME_FILE_MUTATION,
} from './mutations';
import type { BrowseResponse, BrowseDirectoryResult, FileOperationResult } from './types';

/**
 * Browse a directory on the server filesystem
 *
 * @param path - Path to browse (defaults to root)
 * @param dirsOnly - Only show directories (default: true)
 * @returns Browse response with entries and quick paths
 */
export async function browseDirectory(
  path?: string,
  dirsOnly = true
): Promise<BrowseResponse> {
  const result = await graphqlClient
    .query<{ browseDirectory: BrowseDirectoryResult }>(BROWSE_DIRECTORY_QUERY, {
      input: {
        path: path || null,
        dirsOnly,
        showHidden: false,
      },
    })
    .toPromise();

  if (result.error) {
    throw new Error(result.error.message);
  }

  if (!result.data?.browseDirectory) {
    throw new Error('Failed to browse directory');
  }

  const data = result.data.browseDirectory;

  // Convert to BrowseResponse for backward compatibility
  return {
    currentPath: data.currentPath,
    parentPath: data.parentPath,
    entries: data.entries.map((e) => ({
      name: e.name,
      path: e.path,
      isDir: e.isDir,
      size: e.size,
      sizeFormatted: e.sizeFormatted,
      readable: e.readable,
      writable: e.writable,
      mimeType: e.mimeType,
      modifiedAt: e.modifiedAt,
    })),
    quickPaths: data.quickPaths,
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
  const result = await graphqlClient
    .mutation<{ createDirectory: FileOperationResult }>(CREATE_DIRECTORY_MUTATION, {
      input: { path },
    })
    .toPromise();

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
    path: data.path || undefined,
    error: data.error || undefined,
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
  const result = await graphqlClient
    .mutation<{ deleteFiles: FileOperationResult }>(DELETE_FILES_MUTATION, {
      input: { paths, recursive },
    })
    .toPromise();

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

  return result.data.deleteFiles;
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
  const result = await graphqlClient
    .mutation<{ copyFiles: FileOperationResult }>(COPY_FILES_MUTATION, {
      input: { sources, destination, overwrite },
    })
    .toPromise();

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

  return result.data.copyFiles;
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
  const result = await graphqlClient
    .mutation<{ moveFiles: FileOperationResult }>(MOVE_FILES_MUTATION, {
      input: { sources, destination, overwrite },
    })
    .toPromise();

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

  return result.data.moveFiles;
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
  const result = await graphqlClient
    .mutation<{ renameFile: FileOperationResult }>(RENAME_FILE_MUTATION, {
      input: { path, newName },
    })
    .toPromise();

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

  return result.data.renameFile;
}
