// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
const mocks = vi.hoisted(() => ({
  mutate: vi.fn(),
  refetchQueries: vi.fn(),
  toast: vi.fn(),
  connections: new Set<(state: { status: string }) => void>(),
}));
vi.mock("../../src/lib/graphql/client", () => ({
  onWebSocketConnectionState: (
    listener: (state: { status: string }) => void,
  ) => {
    mocks.connections.add(listener);
    listener({ status: "connected" });
    return () => mocks.connections.delete(listener);
  },
  apolloClient: { mutate: mocks.mutate, refetchQueries: mocks.refetchQueries },
}));
vi.mock("../../src/hooks/useAuth", () => ({
  useAuth: () => ({ user: { id: "owner-a" } }),
}));
vi.mock("@heroui/toast", () => ({ addToast: mocks.toast }));
import {
  markAllNotificationsRead,
  useMarkAllNotificationsRead,
  useNotificationRefresh,
  useNotificationOwner,
} from "../../src/hooks/useNotificationFeed";
import { MarkAllNotificationsReadDocument } from "../../src/lib/graphql/generated/graphql";

beforeEach(() => {
  vi.clearAllMocks();
  mocks.refetchQueries.mockResolvedValue([]);
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});
it("acknowledges all pages of both feed types with one request and one refresh pass", async () => {
  mocks.mutate.mockResolvedValue({
    data: {
      markAllNotificationsRead: { notificationCount: 404, scanIssueCount: 307 },
    },
  });
  const first = markAllNotificationsRead();
  const second = markAllNotificationsRead();
  expect(first).toBe(second);
  expect(await first).toBe(711);
  expect(mocks.mutate).toHaveBeenCalledTimes(1);
  expect(mocks.mutate).toHaveBeenCalledWith({
    mutation: MarkAllNotificationsReadDocument,
  });
  expect(mocks.refetchQueries).toHaveBeenCalledTimes(1);
});
it("keeps the button busy until acknowledged and reports failure without claiming success", async () => {
  let reject!: (error: Error) => void;
  mocks.mutate.mockImplementationOnce(
    () =>
      new Promise((_, no) => {
        reject = no;
      }),
  );
  function Harness() {
    const { handleMarkAllRead, markingAllRead } = useMarkAllNotificationsRead();
    return (
      <button disabled={markingAllRead} onClick={handleMarkAllRead}>
        Mark all read
      </button>
    );
  }
  render(<Harness />);
  fireEvent.click(screen.getByRole("button"));
  expect((screen.getByRole("button") as HTMLButtonElement).disabled).toBe(true);
  await act(async () => reject(new Error("Server unavailable")));
  await waitFor(() =>
    expect((screen.getByRole("button") as HTMLButtonElement).disabled).toBe(
      false,
    ),
  );
  expect(mocks.refetchQueries).not.toHaveBeenCalled();
  expect(mocks.toast).toHaveBeenCalledWith(
    expect.objectContaining({ color: "danger" }),
  );
  mocks.mutate.mockResolvedValue({
    data: {
      markAllNotificationsRead: { notificationCount: 1, scanIssueCount: 0 },
    },
  });
  fireEvent.click(screen.getByRole("button"));
  await waitFor(() =>
    expect(mocks.toast).toHaveBeenCalledWith(
      expect.objectContaining({ color: "success" }),
    ),
  );
});
it("coalesces hundreds of subscription events and does not refresh on rerender", async () => {
  vi.useFakeTimers();
  const refresh = vi.fn().mockResolvedValue(undefined);
  const { result, rerender, unmount } = renderHook(() =>
    useNotificationRefresh(() => refresh()),
  );
  act(() => {
    for (let i = 0; i < 711; i++) result.current();
  });
  await act(async () => vi.advanceTimersByTimeAsync(300));
  expect(refresh).toHaveBeenCalledTimes(1);
  rerender();
  await act(async () => vi.advanceTimersByTimeAsync(5000));
  expect(refresh).toHaveBeenCalledTimes(1);
  act(() => result.current());
  unmount();
  await vi.advanceTimersByTimeAsync(1000);
  expect(refresh).toHaveBeenCalledTimes(1);
});
it("queues at most one follow-up refresh while a previous request is in flight", async () => {
  vi.useFakeTimers();
  let resolve!: () => void;
  const refresh = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<void>((yes) => {
          resolve = yes;
        }),
    )
    .mockResolvedValue(undefined);
  const { result } = renderHook(() => useNotificationRefresh(refresh));
  act(() => result.current());
  await act(async () => vi.advanceTimersByTimeAsync(300));
  act(() => {
    for (let i = 0; i < 500; i++) result.current();
  });
  await act(async () => vi.advanceTimersByTimeAsync(1000));
  expect(refresh).toHaveBeenCalledTimes(1);
  await act(async () => resolve());
  await act(async () => vi.advanceTimersByTimeAsync(300));
  expect(refresh).toHaveBeenCalledTimes(2);
});
it("keeps the feed scoped to the current account even for administrators", () => {
  const { result } = renderHook(() => useNotificationOwner());
  expect(result.current).toEqual({ userId: { eq: "owner-a" } });
});

it("refreshes after reconnect to recover notifications missed during token renewal", async () => {
  vi.useFakeTimers();
  const refresh = vi.fn().mockResolvedValue(undefined);
  renderHook(() => useNotificationRefresh(refresh));
  act(() => {
    for (const status of ["idle", "connecting", "connected", "connected"]) {
      mocks.connections.forEach((listener) => listener({ status }));
    }
  });
  await act(async () => vi.advanceTimersByTimeAsync(300));
  expect(refresh).toHaveBeenCalledTimes(1);
});
