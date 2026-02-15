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

export type WebSocketConnectionStatus =
  | "idle"
  | "connecting"
  | "connected"
  | "disconnected";

export interface WebSocketConnectionState {
  status: WebSocketConnectionStatus;
  retryCount: number;
  lastCloseCode: number | null;
  lastCloseReason: string | null;
  isAuthIssue: boolean;
}

type WebSocketConnectionHandler = (state: WebSocketConnectionState) => void;
const wsConnectionHandlers: Set<WebSocketConnectionHandler> = new Set();

let wsConnectionState: WebSocketConnectionState = {
  status: "idle",
  retryCount: 0,
  lastCloseCode: null,
  lastCloseReason: null,
  isAuthIssue: false,
};

function isExpectedWsClose(code: number | null): boolean {
  // 1000/1001 are normal client/server lifecycle closes (including lazy mode)
  // and should not be treated as server disconnects.
  return code === 1000 || code === 1001;
}

function notifyWebSocketConnectionState() {
  wsConnectionHandlers.forEach((handler) => handler(wsConnectionState));
}

function updateWebSocketConnectionState(
  partial: Partial<WebSocketConnectionState>,
) {
  wsConnectionState = { ...wsConnectionState, ...partial };
  notifyWebSocketConnectionState();
}

export function onWebSocketConnectionState(
  handler: WebSocketConnectionHandler,
): () => void {
  wsConnectionHandlers.add(handler);
  handler(wsConnectionState);
  return () => wsConnectionHandlers.delete(handler);
}

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
  credentials: "include",
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

// WebSocket client for subscriptions - lazy mode so it reconnects with fresh auth
const wsClient = createWSClient({
  url: `${WS_URL}/graphql/ws`,
  connectionParams: () => ({
    Authorization: getAuthTokenSync(),
  }),
  lazy: true, // Only connect when needed
  retryAttempts: Infinity,
  retryWait: async (retries) => {
    // Exponential backoff with cap to keep trying forever.
    const delayMs = Math.min(1000 * 2 ** retries, 10000);
    await new Promise((resolve) => setTimeout(resolve, delayMs));
  },
  shouldRetry: () => true,
  on: {
    connecting: (isRetry) => {
      updateWebSocketConnectionState({
        status: "connecting",
        retryCount: isRetry ? wsConnectionState.retryCount + 1 : 0,
        isAuthIssue: false,
      });
    },
    connected: () => {
      updateWebSocketConnectionState({
        status: "connected",
        retryCount: 0,
        lastCloseCode: null,
        lastCloseReason: null,
        isAuthIssue: false,
      });
    },
    closed: (event) => {
      const closeCode =
        typeof event === "object" &&
        event != null &&
        "code" in event &&
        typeof (event as { code?: unknown }).code === "number"
          ? (event as { code: number }).code
          : null;
      const closeReason =
        typeof event === "object" &&
        event != null &&
        "reason" in event &&
        typeof (event as { reason?: unknown }).reason === "string"
          ? (event as { reason: string }).reason
          : null;
      const isAuthIssue = closeCode === 4401 || closeCode === 4403;
      const isExpectedClose = isExpectedWsClose(closeCode);
      updateWebSocketConnectionState({
        status: isAuthIssue || isExpectedClose ? "idle" : "disconnected",
        lastCloseCode: closeCode,
        lastCloseReason: closeReason,
        isAuthIssue,
      });
    },
    error: (err) => {
      // Some subscription operation errors can flow through this callback even
      // while the underlying WebSocket transport remains healthy. Let `closed`
      // drive transport state transitions to avoid false "disconnected" overlays.
      console.warn("[GraphQL WS] socket error callback:", err);
    },
  },
});

const wsLink = new GraphQLWsLink(wsClient);

// Keep auth lifecycle operations on HTTP because they rely on HTTP cookie headers.
const HTTP_ONLY_OPERATION_NAMES = new Set([
  "Login",
  "Register",
  "RefreshToken",
  "Logout",
]);

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
      } else {
        console.error(
          `[GraphQL error]: Message: ${message}, Operation: ${operationName}`
        );
        // Notify subscribers about the error
        notifyError(message, false);
      }
    });
  } else if (error) {
    // Network or other error
    console.error(
      `[Network error]: ${error.message}, Operation: ${operationName}`
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

// Split link: auth lifecycle ops go to HTTP; everything else goes to WebSocket.
// This enables WS-first query/mutation/subscription transport while preserving
// cookie-based auth flows that require HTTP response headers.
const splitLink = split(
  (operation) => {
    if (typeof window === "undefined") {
      // SSR/build-time execution should not attempt a WebSocket transport.
      return true;
    }
    const { query, operationName } = operation;
    const definition = getMainDefinition(query);
    if (definition.kind !== "OperationDefinition") {
      return true;
    }
    const resolvedName = operationName || definition.name?.value;
    if (!resolvedName) {
      // Keep anonymous operations on HTTP for predictable behavior.
      return true;
    }
    return HTTP_ONLY_OPERATION_NAMES.has(resolvedName);
  },
  authedHttpLink,
  wsLink,
);

// Create Apollo Client
export const apolloClient = new ApolloClient({
  link: splitLink,
  cache: new InMemoryCache({
    typePolicies: {
      MediaFile: {
        keyFields: ["Id"],
      },
      Query: {
        fields: {
          MediaFile: {
            // Merge partial payloads (e.g. { Id, Metadata }) into existing
            // MediaFile objects instead of replacing and dropping cached fields.
            merge: true,
          },
        },
      },
    },
  }),
  defaultOptions: {
    watchQuery: {
      fetchPolicy: "cache-and-network",
    },
    query: {
      fetchPolicy: "network-only",
    },
  },
});

// Re-export Apollo hooks for convenience
export {
  useQuery,
  useMutation,
  useSubscription,
  useLazyQuery,
} from "@apollo/client/react";
export { gql };

// Reset Apollo cache after login/logout to clear stale auth state.
// Use clearStore (no active query refetch) and serialize calls to avoid
// "Store reset while query was in flight" invariant races.
let cacheResetInFlight = false;
let cacheResetPending = false;

async function runCacheReset(): Promise<void> {
  if (cacheResetInFlight) {
    cacheResetPending = true;
    return;
  }
  cacheResetInFlight = true;

  try {
    await apolloClient.clearStore();
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    // Apollo may throw this during auth transitions with active requests.
    // It's safe to ignore because we immediately move to new auth state.
    if (!message.includes("Store reset while query was in flight")) {
      console.warn("[Apollo] Cache reset failed:", err);
    }
  } finally {
    cacheResetInFlight = false;
    if (cacheResetPending) {
      cacheResetPending = false;
      queueMicrotask(() => {
        void runCacheReset();
      });
    }
  }
}

export function resetApolloCache(): void {
  queueMicrotask(() => {
    void runCacheReset();
  });
}

// Promise helpers for route/components being migrated off urql-style chaining
export async function queryPromise<
  TData = unknown,
  TVariables = OperationVariables,
>(
  query: TypedDocumentNode<TData, TVariables> | string | DocumentNode,
  variables?: TVariables
): Promise<{ data?: TData; error?: Error }> {
  try {
    const doc = typeof query === "string" ? gql(query) : query;
    const result = await apolloClient.query<TData>({
      query: doc as TypedDocumentNode<TData, TVariables>,
      variables: variables as OperationVariables,
      fetchPolicy: "network-only",
    });
    return { data: result.data };
  } catch (error) {
    return { error: error as Error };
  }
}

export async function mutationPromise<
  TData = unknown,
  TVariables = OperationVariables,
>(
  mutation: TypedDocumentNode<TData, TVariables> | string | DocumentNode,
  variables?: TVariables
): Promise<{ data?: TData; error?: Error }> {
  try {
    const doc = typeof mutation === "string" ? gql(mutation) : mutation;
    const result = await apolloClient.mutate<TData>({
      mutation: doc as TypedDocumentNode<TData, TVariables>,
      variables: variables as OperationVariables,
    });
    return { data: result.data ?? undefined };
  } catch (error) {
    return { error: error as Error };
  }
}

export function subscriptionStream<
  TData = unknown,
  TVariables = OperationVariables,
>(
  subscription: TypedDocumentNode<TData, TVariables> | string | DocumentNode,
  variables?: TVariables
) {
  const doc =
    typeof subscription === "string" ? gql(subscription) : subscription;
  return apolloClient.subscribe<TData>({
    query: doc as TypedDocumentNode<TData, TVariables>,
    variables: variables as OperationVariables,
  });
}

// Legacy wrapper for compatibility with existing code that uses urql-style API
export const graphqlClient = {
  query: <TData = unknown, TVariables = OperationVariables>(
    query: TypedDocumentNode<TData, TVariables> | string | DocumentNode,
    variables?: TVariables
  ) => ({
    toPromise: async (): Promise<{ data?: TData; error?: Error }> => {
      try {
        const doc = typeof query === "string" ? gql(query) : query;
        const result = await apolloClient.query<TData>({
          query: doc as TypedDocumentNode<TData, TVariables>,
          variables: variables as OperationVariables,
          fetchPolicy: "network-only",
        });
        return { data: result.data };
      } catch (error) {
        return { error: error as Error };
      }
    },
  }),

  mutation: <TData = unknown, TVariables = OperationVariables>(
    mutation: TypedDocumentNode<TData, TVariables> | string | DocumentNode,
    variables?: TVariables
  ) => ({
    toPromise: async (): Promise<{ data?: TData; error?: Error }> => {
      try {
        const doc = typeof mutation === "string" ? gql(mutation) : mutation;
        const result = await apolloClient.mutate<TData>({
          mutation: doc as TypedDocumentNode<TData, TVariables>,
          variables: variables as OperationVariables,
        });
        return { data: result.data ?? undefined };
      } catch (error) {
        return { error: error as Error };
      }
    },
  }),

  subscription: <TData = unknown, TVariables = OperationVariables>(
    subscription: TypedDocumentNode<TData, TVariables> | string | DocumentNode,
    variables?: TVariables
  ) => {
    const doc =
      typeof subscription === "string" ? gql(subscription) : subscription;
    return apolloClient.subscribe<TData>({
      query: doc as TypedDocumentNode<TData, TVariables>,
      variables: variables as OperationVariables,
    });
  },
};

// Transitional global exposure for files being migrated off `graphqlClient...toPromise()`.
// This keeps runtime stable while imports are standardized incrementally.
if (import.meta.env.DEV && typeof globalThis !== "undefined") {
  (globalThis as any).graphqlClient = graphqlClient;
  (globalThis as any).queryPromise = queryPromise;
  (globalThis as any).mutationPromise = mutationPromise;
  (globalThis as any).subscriptionStream = subscriptionStream;
}
