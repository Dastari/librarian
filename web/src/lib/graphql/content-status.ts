import type { ApolloClient } from "@apollo/client";

/** Computed statuses are separate cache records from the entity mutation result.
 * Refresh their active queries without refetching paged media collections.
 */
export function activeContentStatusQueries(client: ApolloClient) {
  return [...client.getObservableQueries("active").values()]
    .filter((query) => query.queryName === "ContentStatuses")
    .map((query) => ({ query: query.options.query, variables: query.variables }));
}
