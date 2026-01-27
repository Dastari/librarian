/**
 * Filesystem GraphQL operations
 *
 * Uses codegen-generated documents and PascalCase types from the backend schema.
 */

import { graphqlClient } from './client';
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
  const result = await graphqlClient
    .query(BrowseDirectoryDocument, {
      Input: {
        Path: path ?? null,
        DirsOnly: dirsOnly,
        ShowHidden: false,
      },
    })
    .toPromise();

  if (result.error) {
    throw new Error(result.error.message);
  }

  const data = result.data?.BrowseDirectory;
  if (!data) {
    throw new Error('Failed to browse directory');
  }

  return data;
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
    .mutation<{ CreateDirectory: FileOperationPayloadPascal }>(CREATE_DIRECTORY_MUTATION, {
      Input: { Path: path },
    })
    .toPromise();

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
  const result = await graphqlClient
    .mutation<{ DeleteFiles: FileOperationPayloadPascal }>(DELETE_FILES_MUTATION, {
      Input: { Paths: paths, Recursive: recursive },
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
  const result = await graphqlClient
    .mutation<{ CopyFiles: FileOperationPayloadPascal }>(COPY_FILES_MUTATION, {
      Input: { Sources: sources, Destination: destination, Overwrite: overwrite },
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
  const result = await graphqlClient
    .mutation<{ MoveFiles: FileOperationPayloadPascal }>(MOVE_FILES_MUTATION, {
      Input: { Sources: sources, Destination: destination, Overwrite: overwrite },
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
  const result = await graphqlClient
    .mutation<{ RenameFile: FileOperationPayloadPascal }>(RENAME_FILE_MUTATION, {
      Input: { Path: path, NewName: newName },
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
