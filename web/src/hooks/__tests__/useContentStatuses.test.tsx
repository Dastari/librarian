import { ApolloClient, ApolloLink, InMemoryCache, Observable, gql } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { EntityEpisodeUpdateDocument, type ContentStatus, type ContentStatusRequestInput } from "@/graphql/generated/graphql";
import { typePolicies } from "@/lib/apollo/typePolicies";
import { activeContentStatusQueries } from "@/lib/graphql/content-status";

import { useContentStatuses, useContentStatusState } from "../useContentStatuses";

const principal = vi.hoisted(() => ({ id: "user-1" }));
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ status: "authenticated", user: { id: principal.id, role: "member" } }) }));
const clients: ApolloClient[] = [];

function fixture(resolve: (inputs: ContentStatusRequestInput[], userId: string) => { status?: ContentStatus; delay?: number; error?: Error } = () => ({})) {
  const calls: ContentStatusRequestInput[][] = [];
  let wanted = true;
  const client = new ApolloClient({ cache: new InMemoryCache({ typePolicies }), link: new ApolloLink((operation) => new Observable((observer) => {
    if (operation.operationName === "EntityEpisodeUpdate") {
      wanted = false;
      observer.next({ data: { updateEpisode: { __typename: "UpdateEpisodeResult", success: true, error: null, episode: null } } });
      observer.complete();
      return;
    }
    const inputs = operation.variables.inputs as ContentStatusRequestInput[];
    calls.push(inputs);
    const reply = resolve(inputs, principal.id);
    const timer = setTimeout(() => {
      if (reply.error) observer.error(reply.error);
      else {
        observer.next({ data: { contentStatuses: inputs.map((input) => ({ __typename: "ContentStatusResult", ...input, status: reply.status ?? (wanted ? "WANTED" : "MISSING") })) } });
        observer.complete();
      }
    }, reply.delay ?? 1);
    return () => clearTimeout(timer);
  })) });
  clients.push(client);
  const wrapper = ({ children }: { children: ReactNode }) => <ApolloProvider client={client}>{children}</ApolloProvider>;
  return { client, calls, wrapper };
}

afterEach(() => {
  cleanup();
  for (const client of clients.splice(0)) client.stop();
  principal.id = "user-1";
});

describe("content status batching", () => {
  it("resolves more than 500 IDs and preserves the Map API for all media pages", async () => {
    const setup = fixture();
    const ids = Array.from({ length: 1001 }, (_, index) => `episode-${index}`);
    const { result, rerender } = renderHook(({ ids }) => useContentStatuses("EPISODE", ids), { initialProps: { ids }, wrapper: setup.wrapper });
    await waitFor(() => expect(result.current.size).toBe(1001));
    expect(result.current).toBeInstanceOf(Map);
    expect(setup.calls.map((inputs) => inputs.length)).toEqual([500, 500, 1]);
    expect(activeContentStatusQueries(setup.client)).toHaveLength(3);
    const previous = result.current;
    rerender({ ids: [...ids] });
    expect(result.current).toBe(previous);
    expect(setup.calls).toHaveLength(3);
  });

  it("observes normalized cache updates and remains active for mutation invalidation", async () => {
    const setup = fixture();
    const { result } = renderHook(() => useContentStatusState("EPISODE", ["episode-1"]), { wrapper: setup.wrapper });
    await waitFor(() => expect(result.current.statuses.get("episode-1")).toBe("WANTED"));
    act(() => {
      setup.client.cache.writeFragment({
        id: setup.client.cache.identify({ __typename: "ContentStatusResult", contentType: "EPISODE", id: "episode-1" }),
        fragment: gql`fragment UpdatedStatus on ContentStatusResult { contentType id status }`,
        data: { __typename: "ContentStatusResult", contentType: "EPISODE", id: "episode-1", status: "DOWNLOADING" },
      });
    });
    await waitFor(() => expect(result.current.statuses.get("episode-1")).toBe("DOWNLOADING"));
    await act(async () => {
      await setup.client.mutate({ mutation: EntityEpisodeUpdateDocument, variables: { id: "episode-1", input: { wanted: false } }, refetchQueries: () => activeContentStatusQueries(setup.client), awaitRefetchQueries: true });
    });
    await waitFor(() => expect(result.current.statuses.get("episode-1")).toBe("MISSING"));
    expect(setup.calls).toHaveLength(2);
  });

  it("clears old scope rows immediately and discards late batches", async () => {
    const setup = fixture((inputs) => ({ status: inputs[0]?.id === "old" ? "WANTED" : "DOWNLOADING", delay: inputs[0]?.id === "old" ? 80 : 1 }));
    const { result, rerender } = renderHook(({ id }) => useContentStatusState("TRACK", [id]), { initialProps: { id: "old" }, wrapper: setup.wrapper });
    rerender({ id: "new" });
    expect(result.current.statuses.size).toBe(0);
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.statuses.get("new")).toBe("DOWNLOADING"));
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 90)); });
    expect([...result.current.statuses.keys()]).toEqual(["new"]);
  });

  it("does not expose a previous user's cached status for the same media ID", async () => {
    const setup = fixture((_inputs, userId) => ({ status: userId === "user-1" ? "WANTED" : "MISSING", delay: 10 }));
    const { result, rerender } = renderHook(() => useContentStatusState("MOVIE", ["shared-movie"]), { wrapper: setup.wrapper });
    await waitFor(() => expect(result.current.statuses.get("shared-movie")).toBe("WANTED"));
    principal.id = "user-2";
    rerender();
    expect(result.current.statuses.size).toBe(0);
    expect(result.current.loading).toBe(true);
    await waitFor(() => expect(result.current.statuses.get("shared-movie")).toBe("MISSING"));
  });
});
