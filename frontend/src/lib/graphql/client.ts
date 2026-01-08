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
import { supabase } from '../supabase';

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
