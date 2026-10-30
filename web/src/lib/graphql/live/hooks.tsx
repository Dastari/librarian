import type { ApolloClient, DocumentNode, OperationVariables, TypedDocumentNode } from "@apollo/client";
import { skipToken, useApolloClient, useQuery } from "@apollo/client/react";
import { useRouter } from "@tanstack/react-router";
import { createContext, useContext, useEffect, useRef } from "react";

import { operationKey, queryDependencies } from "./documents";
import { stableKey, type LiveDependency, type LiveRegistry } from "./registry";

export const LiveContext = createContext<{ registry: LiveRegistry; epoch: string } | null>(null);

/** Snapshot owners include this in their identity; old principals/clients never
 * reuse local arrays, cached completeness flags or in-flight generation tokens. */
export function useLiveEpoch(): string {
  const context = useContext(LiveContext);
  const client = useApolloClient();
  return `${context?.epoch ?? "isolated"}:${clientEpochKey(client)}`;
}

export function useLiveInvalidation(
  document: DocumentNode | undefined,
  variables: OperationVariables | undefined,
  refresh: () => unknown | Promise<unknown>,
  options?: { enabled?: boolean; key?: string; shared?: boolean; dependencies?: readonly LiveDependency[] },
): void {
  const context = useContext(LiveContext);
  const callback = useRef(refresh);
  callback.current = refresh;
  const variablesKey = stableKey(variables ?? {});
  const dependencies = options?.dependencies ?? (document ? queryDependencies(document, variables) : []);
  const dependencyKey = stableKey(dependencies);
  const key = options?.key ?? `${document ? operationKey(document) : "disabled"}:${variablesKey}`;
  useEffect(() => {
    if (!context || !document || options?.enabled === false) return;
    return context.registry.register({ key, shared: options?.shared, dependencies, refresh: () => callback.current() });
  }, [context?.registry, context?.epoch, key, dependencyKey, options?.enabled, options?.shared]);
}

// Preserve Apollo's complete overload/inference contract. The implementation
// delegates to Apollo unchanged; the added effect registers its authorized read.
const liveQuery = (document: DocumentNode | TypedDocumentNode<unknown, OperationVariables>, options?: useQuery.Options<unknown, OperationVariables> | typeof skipToken) => {
  const result = useQuery(document, options);
  const config = typeof options === "symbol" ? undefined : options;
  useLiveInvalidation(document, config?.variables, () => result.refetch(), { enabled: options !== skipToken && !config?.skip && config?.context?.liveManaged !== true, shared: config?.fetchPolicy !== "no-cache" });
  return result;
};
export const useLiveQuery = liveQuery as typeof useQuery;

/** Stable client identity for imperative cache scopes without a provider. */
export function clientEpochKey(client: ApolloClient): string {
  let id = clients.get(client);
  if (!id) { id = `client-${++clientSequence}`; clients.set(client, id); }
  return id;
}
const clients = new WeakMap<ApolloClient, string>();
let clientSequence = 0;

/** Route loader snapshots (layout props/crumbs) need their owning router callback,
 * rather than assuming an Apollo cache write replaces loader data. */
export function useLiveRoute(document: DocumentNode, variables: OperationVariables): void {
  const client = useApolloClient();
  const router = useRouter();
  useLiveInvalidation(document, variables, async () => {
    await client.query({ query: document, variables, fetchPolicy: "network-only", context: { queryDeduplication: false } });
    await router.invalidate();
  });
}
