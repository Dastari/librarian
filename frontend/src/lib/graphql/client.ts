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
import { onError } from '@apollo/client/link/error';
import { setContext } from '@apollo/client/link/context';
import { GraphQLWsLink } from '@apollo/client/link/subscriptions';
import { getMainDefinition } from '@apollo/client/utilities';
import { createClient as createWSClient } from 'graphql-ws';
import { supabase } from '../supabase';

// Error event emitter for components to subscribe to
type GraphQLErrorHandler = (error: { message: string; isNetworkError: boolean }) => void;
const errorHandlers: Set<GraphQLErrorHandler> = new Set();

/** Subscribe to GraphQL errors for displaying toasts/alerts */
export function onGraphQLError(handler: GraphQLErrorHandler): () => void {
  errorHandlers.add(handler);
  return () => errorHandlers.delete(handler);
}

function notifyError(message: string, isNetworkError: boolean) {
  errorHandlers.forEach(handler => handler({ message, isNetworkError }));
}

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

// Error link to handle GraphQL and network errors gracefully
// Apollo Client v4 uses a unified error interface
import { CombinedGraphQLErrors } from '@apollo/client/errors';

const errorLink = onError(({ error, operation }) => {
  const operationName = operation.operationName || 'Unknown operation';
  
  // Check if it's a GraphQL error (has errors array)
  if (CombinedGraphQLErrors.is(error)) {
    error.errors.forEach((err) => {
      const message = err.message;
      
      console.error(
        `[GraphQL error]: Message: ${message}, Operation: ${operationName}`
      );
      
      // Don't notify for auth errors (handled separately)
      const isAuthError = message.toLowerCase().includes('not authenticated') || 
                         message.toLowerCase().includes('unauthorized');
      
      // Notify subscribers about the error
      if (!isAuthError) {
        notifyError(message, false);
      }
    });
  } else if (error) {
    // Network or other error
    console.error(`[Network error]: ${error.message}, Operation: ${operationName}`);
    
    // Notify subscribers about network error
    const errorMessage = error.message.includes('Failed to fetch')
      ? 'Unable to connect to server. Please check your connection.'
      : error.message;
    
    notifyError(errorMessage, true);
  }
});

// Combine error, auth and http links
const authedHttpLink = from([errorLink, authLink, httpLink]);

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
