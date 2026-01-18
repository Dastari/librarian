/**
 * Filesystem GraphQL operations
 *
 * These functions provide GraphQL-based filesystem operations,
 * replacing the old REST API endpoints.
 */

import { graphqlClient } from './client';
import { BROWSE_DIRECTORY_QUERY } from './queries';
import { CREATE_DIRECTORY_MUTATION } from './mutations';
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
