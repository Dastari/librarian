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
} from "@apollo/client";
import { onError } from "@apollo/client/link/error";
import { setContext } from "@apollo/client/link/context";
import { GraphQLWsLink } from "@apollo/client/link/subscriptions";
import { getMainDefinition } from "@apollo/client/utilities";
import { createClient as createWSClient } from "graphql-ws";
import { getAuthHeader, getAuthHeaderSync } from "../auth";

// Error event emitter for components to subscribe to
type GraphQLErrorHandler = (error: {
  message: string;
  isNetworkError: boolean;
}) => void;
const errorHandlers: Set<GraphQLErrorHandler> = new Set();

/** Subscribe to GraphQL errors for displaying toasts/alerts */
export function onGraphQLError(handler: GraphQLErrorHandler): () => void {
  errorHandlers.add(handler);
  return () => errorHandlers.delete(handler);
}

function notifyError(message: string, isNetworkError: boolean) {
  errorHandlers.forEach((handler) => handler({ message, isNetworkError }));
}

const API_URL = import.meta.env.VITE_API_URL || "http://localhost:3001";
const WS_URL = API_URL.replace(/^http/, "ws");

// Helper to get auth token from localStorage
async function getAuthTokenAsync(): Promise<string> {
  // This is already synchronous, but kept async for API compatibility
  return getAuthHeader();
}

// Synchronous version for WebSocket connection params
function getAuthTokenSync(): string {
  return getAuthHeaderSync();
}

// HTTP link for queries and mutations
const httpLink = new HttpLink({
  uri: `${API_URL}/graphql`,
});

// Auth context link - adds Authorization header to every request
const authLink = setContext(async (operation, { headers }) => {
  const token = await getAuthTokenAsync();
  
  // Debug logging in development - use warn for no token so it's visible
  if (import.meta.env.DEV) {
    const opName = operation.operationName || 'unknown';
    if (!token) {
      console.warn(`[GraphQL] ⚠️ No auth token for operation: ${opName} - request will fail if auth required`);
    } else {
      // Log that we're sending a token (truncated for security)
      console.debug(`[GraphQL] Sending auth token for ${opName}: ${token.substring(0, 20)}...`);
    }
  }
  
  return {
    headers: {
      ...headers,
      ...(token ? { authorization: token } : {}),
    },
  };
});

// WebSocket client for subscriptions - lazy mode so it reconnects with fresh auth
const wsClient = createWSClient({
  url: `${WS_URL}/graphql/ws`,
  connectionParams: () => ({
    Authorization: getAuthTokenSync(),
  }),
  lazy: true, // Only connect when needed
  retryAttempts: 5,
  shouldRetry: () => true,
});

const wsLink = new GraphQLWsLink(wsClient);

// Function to restart WebSocket connection (called after auth changes)
export function restartWebSocket(): void {
  // Terminate existing connection so it reconnects with new auth
  wsClient.terminate();
}

// Error link to handle GraphQL and network errors gracefully
// Apollo Client v4 uses a unified error interface
import { CombinedGraphQLErrors } from "@apollo/client/errors";

const errorLink = onError(({ error, operation }) => {
  const operationName = operation.operationName || "Unknown operation";

  // Check if it's a GraphQL error (has errors array)
  if (CombinedGraphQLErrors.is(error)) {
    error.errors.forEach((err) => {
      const message = err.message;

      // Check if it's an auth error (expected when not logged in)
      const isAuthError =
        message.toLowerCase().includes("not authenticated") ||
        message.toLowerCase().includes("unauthorized") ||
        message.toLowerCase().includes("authentication required");

      // Only log non-auth errors as errors, auth errors are expected when not logged in
      if (isAuthError) {
        // Silently ignore auth errors - they're expected when not logged in
        if (import.meta.env.DEV) {
          console.debug(
            `[GraphQL] Auth required for ${operationName} (user not logged in)`,
          );
        }
      } else {
        console.error(
          `[GraphQL error]: Message: ${message}, Operation: ${operationName}`,
        );
        // Notify subscribers about the error
        notifyError(message, false);
      }
    });
  } else if (error) {
    // Network or other error
    console.error(
      `[Network error]: ${error.message}, Operation: ${operationName}`,
    );

    // Notify subscribers about network error
    const errorMessage = error.message.includes("Failed to fetch")
      ? "Unable to connect to server. Please check your connection."
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
      definition.kind === "OperationDefinition" &&
      definition.operation === "subscription"
    );
  },
  wsLink,
  authedHttpLink,
);

// Create Apollo Client
export const apolloClient = new ApolloClient({
  link: splitLink,
  cache: new InMemoryCache(),
  defaultOptions: {
    watchQuery: {
      fetchPolicy: "cache-and-network",
    },
    query: {
      fetchPolicy: "network-only",
    },
  },
});

// Reset Apollo cache after login/logout to clear any stale auth state
export function resetApolloCache(): void {
  apolloClient.resetStore().catch((err) => {
    console.warn("[Apollo] Cache reset failed:", err);
  });
}

// Legacy wrapper for compatibility with existing code that uses urql-style API
export const graphqlClient = {
  query: <T = unknown>(
    query: string | DocumentNode,
    variables?: OperationVariables,
  ) => ({
    toPromise: async (): Promise<{ data?: T; error?: Error }> => {
      try {
        const doc = typeof query === "string" ? gql(query) : query;
        const result = await apolloClient.query<T>({
          query: doc as TypedDocumentNode<T>,
          variables,
          fetchPolicy: "network-only",
        });
        return { data: result.data };
      } catch (error) {
        return { error: error as Error };
      }
    },
  }),

  mutation: <T = unknown>(
    mutation: string | DocumentNode,
    variables?: OperationVariables,
  ) => ({
    toPromise: async (): Promise<{ data?: T; error?: Error }> => {
      try {
        const doc = typeof mutation === "string" ? gql(mutation) : mutation;
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

  subscription: <T = unknown>(
    subscription: string | DocumentNode,
    variables?: OperationVariables,
  ) => {
    const doc =
      typeof subscription === "string" ? gql(subscription) : subscription;
    return apolloClient.subscribe<T>({
      query: doc as TypedDocumentNode<T>,
      variables,
    });
  },
};
