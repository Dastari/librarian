import { ApolloClient, ApolloLink, InMemoryCache, Observable, gql } from "@apollo/client";
import { describe, expect, it } from "vitest";

import { ContentStatusesDocument, EntityEpisodeUpdateDocument } from "@/graphql/generated/graphql";
import { activeContentStatusQueries } from "../content-status";

describe("computed status refresh after media mutations", () => {
  it("refreshes active backend-computed statuses while leaving the complete media query intact", async () => {
    let wanted = true;
    const calls: string[] = [];
    const client = new ApolloClient({ cache: new InMemoryCache(), link: new ApolloLink((operation) => new Observable((observer) => {
      calls.push(operation.operationName!);
      queueMicrotask(() => {
        if (operation.operationName === "EntityEpisodeUpdate") {
          wanted = false;
          observer.next({ data: { updateEpisode: { __typename: "UpdateEpisodeResult", success: true, error: null, episode: null } } });
        } else if (operation.operationName === "ContentStatuses") {
          observer.next({ data: { contentStatuses: [{ __typename: "ContentStatusResult", contentType: "EPISODE", id: "ep-1", status: wanted ? "WANTED" : "MISSING" }] } });
        } else observer.next({ data: { episodes: { total: 201 } } });
        observer.complete();
      });
    })) });
    const variables = { inputs: [{ contentType: "EPISODE" as const, id: "ep-1" }] };
    const statuses = client.watchQuery({ query: ContentStatusesDocument, variables });
    const collection = client.watchQuery({ query: gql`query CompleteMedia { episodes { total } }` });
    let statusSubscription: { unsubscribe(): void };
    let collectionSubscription: { unsubscribe(): void };
    await Promise.all([
      new Promise<void>((resolve) => { statusSubscription = statuses.subscribe((result) => { if (!result.loading) resolve(); }); }),
      new Promise<void>((resolve) => { collectionSubscription = collection.subscribe((result) => { if (!result.loading) resolve(); }); }),
    ]);
    await client.mutate({ mutation: EntityEpisodeUpdateDocument, variables: { id: "ep-1", input: { wanted: false } }, refetchQueries: () => activeContentStatusQueries(client), awaitRefetchQueries: true });
    expect(client.readQuery({ query: ContentStatusesDocument, variables })?.contentStatuses[0]?.status).toBe("MISSING");
    expect(calls.filter((name) => name === "ContentStatuses")).toHaveLength(2);
    expect(calls.filter((name) => name === "CompleteMedia")).toHaveLength(1);
    statusSubscription!.unsubscribe();
    expect(activeContentStatusQueries(client)).toEqual([]);
    collectionSubscription!.unsubscribe();
    client.stop();
  });
});
