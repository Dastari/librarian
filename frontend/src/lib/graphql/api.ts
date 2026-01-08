import type { BrowseResponse, RawBrowseResponse } from './types';

const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';

// ============================================================================
// Filesystem API (REST)
// ============================================================================

export async function browseDirectory(path?: string, dirsOnly = true): Promise<BrowseResponse> {
  const params = new URLSearchParams();
  if (path) params.set('path', path);
  if (dirsOnly) params.set('dirs_only', 'true');
  
  const response = await fetch(`${API_URL}/api/filesystem/browse?${params}`);
  if (!response.ok) {
    // Try to get the actual error message from the response body
    const errorText = await response.text().catch(() => response.statusText);
    throw new Error(`Failed to browse: ${errorText}`);
  }
  
  // Transform snake_case response to camelCase
  const raw: RawBrowseResponse = await response.json();
  return {
    currentPath: raw.current_path,
    parentPath: raw.parent_path,
    entries: raw.entries.map((e) => ({
      name: e.name,
      path: e.path,
      isDir: e.is_dir,
      size: e.size,
      readable: e.readable,
      writable: e.writable,
    })),
    quickPaths: raw.quick_paths,
  };
}

export async function createDirectory(path: string): Promise<{ success: boolean; error?: string }> {
  const response = await fetch(`${API_URL}/api/filesystem/mkdir`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path }),
  });
  return response.json();
}
