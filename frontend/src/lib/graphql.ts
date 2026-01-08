import {
  ApolloClient,
  InMemoryCache,
  HttpLink,
  split,
  gql,
  from,
  type DocumentNode,
  type OperationVariables,
  type TypedDocumentNode,
} from '@apollo/client';
import { setContext } from '@apollo/client/link/context';
import { GraphQLWsLink } from '@apollo/client/link/subscriptions';
import { getMainDefinition } from '@apollo/client/utilities';
import { createClient as createWSClient } from 'graphql-ws';
import { supabase } from './supabase';

const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';
const WS_URL = API_URL.replace(/^http/, 'ws');

// Helper to get auth token from Supabase session
async function getAuthTokenAsync(): Promise<string> {
  try {
    const { data: { session } } = await supabase.auth.getSession();
    if (session?.access_token) {
      return `Bearer ${session.access_token}`;
    }
  } catch (e) {
    console.error('[GraphQL] Error getting auth token:', e);
  }
  return '';
}

// Synchronous version that reads from localStorage (for WebSocket connection params)
function getAuthTokenSync(): string {
  try {
    const supabaseUrl = import.meta.env.VITE_SUPABASE_URL || 'http://localhost:54321';
    // Try multiple storage key patterns
    const patterns = [
      `sb-${new URL(supabaseUrl).hostname.split('.')[0]}-auth-token`,
      `sb-127-auth-token`,
      `sb-localhost-auth-token`,
    ];
    
    for (const key of patterns) {
      const stored = localStorage.getItem(key);
      if (stored) {
        const session = JSON.parse(stored);
        if (session?.access_token) {
          return `Bearer ${session.access_token}`;
        }
      }
    }
    
    // Also try to find any Supabase auth token key
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith('sb-') && key?.endsWith('-auth-token')) {
        const stored = localStorage.getItem(key);
        if (stored) {
          const session = JSON.parse(stored);
          if (session?.access_token) {
            return `Bearer ${session.access_token}`;
          }
        }
      }
    }
  } catch { /* ignore */ }
  return '';
}

// HTTP link for queries and mutations
const httpLink = new HttpLink({
  uri: `${API_URL}/graphql`,
});

// Auth context link - adds Authorization header to every request
const authLink = setContext(async (_, { headers }) => {
  const token = await getAuthTokenAsync();
  return {
    headers: {
      ...headers,
      ...(token ? { authorization: token } : {}),
    },
  };
});

// WebSocket link for subscriptions
const wsLink = new GraphQLWsLink(
  createWSClient({
    url: `${WS_URL}/graphql/ws`,
    connectionParams: () => ({
      Authorization: getAuthTokenSync(),
    }),
  })
);

// Combine auth link with http link
const authedHttpLink = from([authLink, httpLink]);

// Split link: subscriptions go to WebSocket, everything else to HTTP
const splitLink = split(
  ({ query }) => {
    const definition = getMainDefinition(query);
    return (
      definition.kind === 'OperationDefinition' &&
      definition.operation === 'subscription'
    );
  },
  wsLink,
  authedHttpLink
);

// Create Apollo Client
export const apolloClient = new ApolloClient({
  link: splitLink,
  cache: new InMemoryCache(),
  defaultOptions: {
    watchQuery: {
      fetchPolicy: 'cache-and-network',
    },
    query: {
      fetchPolicy: 'network-only',
    },
  },
});

// Legacy wrapper for compatibility with existing code that uses urql-style API
export const graphqlClient = {
  query: <T = unknown>(query: string | DocumentNode, variables?: OperationVariables) => ({
    toPromise: async (): Promise<{ data?: T; error?: Error }> => {
      try {
        const doc = typeof query === 'string' ? gql(query) : query;
        const result = await apolloClient.query<T>({
          query: doc as TypedDocumentNode<T>,
          variables,
          fetchPolicy: 'network-only',
        });
        return { data: result.data };
      } catch (error) {
        return { error: error as Error };
      }
    },
  }),

  mutation: <T = unknown>(mutation: string | DocumentNode, variables?: OperationVariables) => ({
    toPromise: async (): Promise<{ data?: T; error?: Error }> => {
      try {
        const doc = typeof mutation === 'string' ? gql(mutation) : mutation;
        const result = await apolloClient.mutate<T>({
          mutation: doc as TypedDocumentNode<T>,
          variables,
        });
        return { data: result.data ?? undefined };
      } catch (error) {
        return { error: error as Error };
      }
    },
  }),

  subscription: <T = unknown>(subscription: string | DocumentNode, variables?: OperationVariables) => {
    const doc = typeof subscription === 'string' ? gql(subscription) : subscription;
    return apolloClient.subscribe<T>({
      query: doc as TypedDocumentNode<T>,
      variables,
    });
  },
};

// Torrent types
export interface Torrent {
  id: number;
  infoHash: string;
  name: string;
  state: TorrentState;
  progress: number;
  progressPercent: number;
  size: number;
  sizeFormatted: string;
  downloaded: number;
  uploaded: number;
  downloadSpeed: number;
  downloadSpeedFormatted: string;
  uploadSpeed: number;
  uploadSpeedFormatted: string;
  peers: number;
  eta: number | null;
}

export type TorrentState = 'QUEUED' | 'CHECKING' | 'DOWNLOADING' | 'SEEDING' | 'PAUSED' | 'ERROR';

export interface TorrentProgress {
  id: number;
  infoHash: string;
  progress: number;
  downloadSpeed: number;
  uploadSpeed: number;
  peers: number;
  state: TorrentState;
}

export interface AddTorrentResult {
  success: boolean;
  torrent: Torrent | null;
  error: string | null;
}

export interface TorrentActionResult {
  success: boolean;
  error: string | null;
}

// GraphQL Queries
export const TORRENTS_QUERY = `
  query Torrents {
    torrents {
      id
      infoHash
      name
      state
      progress
      progressPercent
      size
      sizeFormatted
      downloaded
      uploaded
      downloadSpeed
      downloadSpeedFormatted
      uploadSpeed
      uploadSpeedFormatted
      peers
      eta
    }
  }
`;

export const TORRENT_QUERY = `
  query Torrent($id: Int!) {
    torrent(id: $id) {
      id
      infoHash
      name
      state
      progress
      size
      sizeFormatted
    }
  }
`;

// GraphQL Mutations
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

// GraphQL Subscriptions
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
// Settings Types
// ============================================================================

export interface TorrentSettings {
  downloadDir: string;
  sessionDir: string;
  enableDht: boolean;
  listenPort: number;
  maxConcurrent: number;
  uploadLimit: number;
  downloadLimit: number;
}

export interface SettingsResult {
  success: boolean;
  error: string | null;
}

// Settings Queries
export const TORRENT_SETTINGS_QUERY = `
  query TorrentSettings {
    torrentSettings {
      downloadDir
      sessionDir
      enableDht
      listenPort
      maxConcurrent
      uploadLimit
      downloadLimit
    }
  }
`;

// Settings Mutations
export const UPDATE_TORRENT_SETTINGS_MUTATION = `
  mutation UpdateTorrentSettings($input: UpdateTorrentSettingsInput!) {
    updateTorrentSettings(input: $input) {
      success
      error
    }
  }
`;

// ============================================================================
// Filesystem Types
// ============================================================================

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  readable: boolean;
  writable: boolean;
}

export interface QuickPath {
  name: string;
  path: string;
}

export interface BrowseResponse {
  currentPath: string;
  parentPath: string | null;
  entries: FileEntry[];
  quickPaths: QuickPath[];
}

// Raw response types from the backend (snake_case)
interface RawFileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
  readable: boolean;
  writable: boolean;
}

interface RawBrowseResponse {
  current_path: string;
  parent_path: string | null;
  entries: RawFileEntry[];
  quick_paths: QuickPath[];
}

// Filesystem API (REST)
export async function browseDirectory(path?: string, dirsOnly = true): Promise<BrowseResponse> {
  const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';
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
  const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';
  const response = await fetch(`${API_URL}/api/filesystem/mkdir`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ path }),
  });
  return response.json();
}
