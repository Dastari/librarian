/**
 * Apollo Client wiring: HTTP for queries/mutations, graphql-ws for subscriptions, a retry
 * link for flaky networks and an error link that renews the session once on UNAUTHORIZED
 * before replaying the operation.
 */
import { ApolloClient, ApolloLink, HttpLink, InMemoryCache, type Operation } from "@apollo/client";
import { ErrorLink } from "@apollo/client/link/error";
import { RetryLink } from "@apollo/client/link/retry";
import { GraphQLWsLink } from "@apollo/client/link/subscriptions";
import { getMainDefinition } from "@apollo/client/utilities";
import { Kind, OperationTypeNode } from "graphql";
import { createClient } from "graphql-ws";
import { from as observableFrom, switchMap, throwError } from "rxjs";

import { liveTransport } from "@/lib/graphql/live/transport";
import { session } from "@/lib/auth/session";
import { isUnauthorizedError } from "@/lib/graphql/errors";

import { typePolicies } from "./typePolicies";

const AUTH_OPERATIONS = new Set(["Login", "Register", "RefreshSession", "Logout", "NeedsSetup"]);

function websocketUrl(): string {
  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  return `${protocol}://${window.location.host}/graphql/ws`;
}

export const wsClient = createClient({
  url: websocketUrl,
  lazy: true,
  lazyCloseTimeout: 10_000,
  retryAttempts: Infinity,
  shouldRetry: () => true,
  keepAlive: 20_000,
  on: { connected: () => liveTransport.emit(true), closed: () => liveTransport.emit(false), error: () => liveTransport.emit(false) },
});

const httpLink = new HttpLink({ uri: "/graphql", credentials: "include" });
const wsLink = new GraphQLWsLink(wsClient);

export function isReadOperation(operation: Pick<Operation, "query">): boolean {
  const definition = getMainDefinition(operation.query);
  return definition.kind === Kind.OPERATION_DEFINITION && definition.operation === OperationTypeNode.QUERY;
}

// An uncertain response does not establish that a command failed to execute.
// Commands require explicit retry unless their own contract proves idempotency.
export function shouldRetryNetworkError(error: unknown, operation: Pick<Operation, "query">): boolean {
  return isReadOperation(operation) && Boolean(error) && !isUnauthorizedError(error as never);
}

const retryLink = new RetryLink({
  delay: { initial: 400, max: 6_000, jitter: true },
  attempts: {
    max: 4,
    retryIf: shouldRetryNetworkError,
  },
});

/** Exported for tests: retries an operation once after a successful session renewal. */
export const sessionLink = new ErrorLink(({ error, operation, forward }) => {
  if (!isUnauthorizedError(error) || AUTH_OPERATIONS.has(operation.operationName ?? "")) return;
  if (!isReadOperation(operation)) return;
  if (operation.getContext().sessionRetried) return;
  operation.setContext({ sessionRetried: true });
  return observableFrom(session.refresh()).pipe(
    switchMap((renewed) => (renewed ? forward(operation) : throwError(() => error))),
  );
});

const transportLink = ApolloLink.split(
  ({ query }) => {
    const definition = getMainDefinition(query);
    return definition.kind === Kind.OPERATION_DEFINITION && definition.operation === OperationTypeNode.SUBSCRIPTION;
  },
  wsLink,
  ApolloLink.from([retryLink, httpLink]),
);

export const apolloClient = new ApolloClient({
  link: ApolloLink.from([sessionLink, transportLink]),
  cache: new InMemoryCache({ typePolicies }),
  defaultOptions: {
    watchQuery: { fetchPolicy: "cache-and-network", nextFetchPolicy: "cache-first" },
  },
  devtools: { enabled: import.meta.env.DEV },
});

session.attach(apolloClient);
let credential = session.getSnapshot().expiresAt;
session.subscribe(() => {
  const next = session.getSnapshot().expiresAt;
  if (next === credential) return;
  credential = next;
  // Renewal rotates cookies too; an old authenticated socket must not outlive
  // its credential. Reconnection resnapshots active authorized views.
  wsClient.terminate();
});
session.onChange(() => wsClient.terminate());
