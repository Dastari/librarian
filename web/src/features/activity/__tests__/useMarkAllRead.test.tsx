import { ApolloClient, ApolloLink, InMemoryCache, Observable } from "@apollo/client";
import { ApolloProvider } from "@apollo/client/react";
import { act, cleanup, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useMarkAllRead } from "../useMarkAllRead";

const toast = vi.hoisted(() => ({ success: vi.fn(), danger: vi.fn() }));
vi.mock("glass-ux/toast", () => ({ toast }));

type Observer = { next: (value: ApolloLink.Result) => void; error: (error: Error) => void; complete: () => void };

function fixture(reply: (observer: Observer) => void) {
  const operations: string[] = [];
  const client = new ApolloClient({ cache: new InMemoryCache(), link: new ApolloLink((operation) => new Observable((observer) => { operations.push(String(operation.operationName)); reply(observer); })) });
  const onChanged = vi.fn();
  const wrapper = ({ children }: { children: ReactNode }) => <ApolloProvider client={client}>{children}</ApolloProvider>;
  return { ...renderHook(() => useMarkAllRead(onChanged), { wrapper }), operations, onChanged, client };
}

afterEach(() => { cleanup(); vi.clearAllMocks(); });

describe("mark all read", () => {
  it("uses the feed-wide mutation so scan issues are acknowledged too, then reports both counts", async () => {
    const { result, operations, onChanged, client } = fixture((observer) => {
      observer.next({ data: { markAllNotificationsRead: { __typename: "MarkAllNotificationsReadResult", notificationCount: 2, scanIssueCount: 3 } } });
      observer.complete();
    });
    await act(() => result.current.markAllRead());
    expect(operations).toEqual(["MarkAllNotificationsRead"]);
    expect(toast.success).toHaveBeenCalledWith("Marked 5 items read");
    expect(onChanged).toHaveBeenCalledOnce();
    client.stop();
  });

  it("says so when nothing was unread", async () => {
    const { result } = fixture((observer) => {
      observer.next({ data: { markAllNotificationsRead: { __typename: "MarkAllNotificationsReadResult", notificationCount: 0, scanIssueCount: 0 } } });
      observer.complete();
    });
    await act(() => result.current.markAllRead());
    expect(toast.success).toHaveBeenCalledWith("Everything was already read");
  });

  it("reports a failure and still refreshes the list", async () => {
    const { result, onChanged } = fixture((observer) => observer.error(new Error("offline")));
    await act(() => result.current.markAllRead());
    expect(toast.danger).toHaveBeenCalledWith("offline");
    expect(toast.success).not.toHaveBeenCalled();
    expect(onChanged).toHaveBeenCalledOnce();
  });
});
