/**
 * Apollo Client wiring: HTTP for queries/mutations, graphql-ws for subscriptions, a retry
 * link for flaky networks and an error link that renews the session once on UNAUTHORIZED
 * before replaying the operation.
 */
import { ApolloClient, ApolloLink, HttpLink, InMemoryCache } from "@apollo/client";
import { ErrorLink } from "@apollo/client/link/error";
import { RetryLink } from "@apollo/client/link/retry";
import { GraphQLWsLink } from "@apollo/client/link/subscriptions";
import { getMainDefinition } from "@apollo/client/utilities";
import { Kind, OperationTypeNode } from "graphql";
import { createClient } from "graphql-ws";
import { from as observableFrom, switchMap, throwError } from "rxjs";

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
});

const httpLink = new HttpLink({ uri: "/graphql", credentials: "include" });
const wsLink = new GraphQLWsLink(wsClient);

const retryLink = new RetryLink({
  delay: { initial: 400, max: 6_000, jitter: true },
  attempts: {
    max: 4,
    retryIf: (error) => Boolean(error) && !isUnauthorizedError(error as never),
  },
});

/** Exported for tests: retries an operation once after a successful session renewal. */
export const sessionLink = new ErrorLink(({ error, operation, forward }) => {
  if (!isUnauthorizedError(error) || AUTH_OPERATIONS.has(operation.operationName ?? "")) return;
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
session.onChange(() => {
  // Force the socket to reconnect so the new (or absent) cookies are used.
  wsClient.terminate();
});
