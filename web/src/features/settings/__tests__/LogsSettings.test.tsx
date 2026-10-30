import { ApolloClient, ApolloLink, InMemoryCache, Observable } from "@apollo/client";
import { createMemoryHistory, createRootRoute, createRouter, RouterProvider } from "@tanstack/react-router";
import { ApolloProvider } from "@apollo/client/react";
import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Toaster, toast } from "glass-ux/toast";
import { TooltipProvider } from "glass-ux/tooltip";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { apolloLiveSource } from "@/lib/graphql/live/documents";
import { LiveContext } from "@/lib/graphql/live/hooks";
import { LiveRegistry } from "@/lib/graphql/live/registry";
import type { AppLogFieldsFragment } from "@/graphql/generated/graphql";

import { LogsSettings } from "../LogsSettings";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "log-admin", role: "admin" } }) }));

const log: AppLogFieldsFragment = { id: "entry-1", timestamp: "2026-10-23T12:00:00Z", level: "ERROR", target: "librarian::scanner", message: "Could not scan Movie.mkv", fields: '{"path":"/media/Movie.mkv"}', spanName: "scan", spanId: "span-1", createdAt: "2026-10-23T12:00:00Z" };

function mount(options: { deleteSuccess?: boolean; more?: boolean; search?: Record<string, unknown> } = {}) {
  const queries = vi.fn();
  const deletes = vi.fn();
  let emitChange: (() => void) | undefined;
  let rows = [log];
  let failQueries = false;
  let holdNextAppend = false;
  let deferredAppend: (() => void) | undefined;
  const client = new ApolloClient({
    cache: new InMemoryCache(),
    link: new ApolloLink((operation) => new Observable((observer) => {
      if (operation.operationName === "EntityAppLogChanged") {
        emitChange = () => observer.next({ data: { appLogChanged: { action: "CREATED", changeKind: "DIRECT", id: "new-entry", sourceEntity: null, sourceId: null, path: [] } } });
        return () => { emitChange = undefined; };
      }
      const timeout = setTimeout(() => {
        if (operation.operationName === "EntityAppLogDeleteMany") {
          deletes(operation.variables);
          if (options.deleteSuccess !== false) rows = [];
          observer.next({ data: { deleteAppLogs: { success: options.deleteSuccess !== false, error: options.deleteSuccess === false ? "Log store unavailable" : null } } });
        } else {
          queries(operation.variables);
          if (failQueries) { observer.error(new Error("Refresh unavailable")); return; }
          const keyset = operation.operationName === "EntityAppLogKeyset";
          const offset = keyset ? Number(Boolean(operation.variables.page?.after)) : Number(operation.variables.page?.offset ?? 0);
          const batch = offset && rows.length ? [{ ...log, id: "older-entry", message: "Older log entry" }] : rows;
          const respond = () => {
            observer.next({ data: { [keyset ? "appLogsKeyset" : "appLogs"]: { edges: batch.map((node) => ({ __typename: "AppLogEdge", cursor: node.id, node: { __typename: "AppLog", ...node } })), pageInfo: { hasNextPage: Boolean(options.more && !offset && rows.length), hasPreviousPage: offset > 0, endCursor: offset ? "tail" : "head", startCursor: "start", totalCount: rows.length ? rows.length + Number(Boolean(options.more)) : 0 } } } });
          };
          if (holdNextAppend && offset) {
            holdNextAppend = false;
            deferredAppend = () => { respond(); observer.complete(); };
            return;
          }
          respond();
        }
        observer.complete();
      }, 0);
      return () => clearTimeout(timeout);
    })),
  });
  const registry = new LiveRegistry(apolloLiveSource(client, true));
  registry.reset(true);
  const tree = <ApolloProvider client={client}><LiveContext.Provider value={{ registry, epoch: "log-admin" }}><TooltipProvider><LogsSettings /><Toaster /></TooltipProvider></LiveContext.Provider></ApolloProvider>;
  const router = options.search ? createRouter({
    routeTree: createRootRoute({ component: () => tree }),
    history: createMemoryHistory({ initialEntries: [`/?${new URLSearchParams(Object.entries(options.search).map(([key, value]) => [key, typeof value === "string" ? value : JSON.stringify(value)]))}`] }),
    defaultPendingMinMs: 0,
  }) : undefined;
  const view = render(router ? <RouterProvider router={router} /> : tree);
  return {
    ...view, queries, deletes, router,
    holdNextAppend: () => { holdNextAppend = true; },
    releaseAppend: () => {
      if (!deferredAppend) throw new Error("No pending append to release");
      const finish = deferredAppend;
      deferredAppend = undefined;
      finish();
    },
    setQueryFailure: (fail: boolean) => { failQueries = fail; },
    addLog: () => { rows = [{ ...log, id: "entry-2", message: "New live entry" }, ...rows]; },
    emitChange: () => { emitChange?.(); },
  };
}

// Supply layout measurements; keep the real table, virtualizer and all interactions.
const intersections = new Set<() => void>();
function revealEnd() { act(() => { [...intersections].forEach((notify) => notify()); }); }
beforeEach(() => {
  intersections.clear();
  vi.stubGlobal("IntersectionObserver", class {
    notify: () => void;
    constructor(callback: IntersectionObserverCallback) { this.notify = () => callback([{ isIntersecting: true } as IntersectionObserverEntry], this as unknown as IntersectionObserver); }
    observe() { intersections.add(this.notify); }
    disconnect() { intersections.delete(this.notify); }
    unobserve() {}
  });
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ x: 0, y: 0, top: 0, left: 0, bottom: 800, right: 1440, width: 1440, height: 800, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : 1440; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 800; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1440);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
});
afterEach(() => { toast.dismiss(); vi.restoreAllMocks(); vi.unstubAllGlobals(); });

describe("LogsSettings Glass table pilot", () => {
  it("hydrates a shared query and sends only allowlisted camelCase server inputs", async () => {
    const view = mount({ search: {
      q: "unrelated", tab: "details",
      "table.settings.logs": {
        sorting: [{ id: "level", desc: false }, { id: "privateField", desc: true }], search: "Movie",
        filters: [{ id: "level", value: { op: "in", values: ["WARN"] } }, { id: "target", value: { op: "contains", value: "scanner" } }, { id: "id", value: { op: "eq", value: "injected" } }],
        pagination: { pageIndex: 99, pageSize: 999 }, selection: { secret: true },
      },
    } });
    await screen.findByText(log.message);
    expect(view.queries).toHaveBeenLastCalledWith({
      where: { and: [{ level: { inList: ["WARN"] } }, { target: { contains: "scanner" } }, { or: [{ message: { contains: "Movie" } }, { target: { contains: "Movie" } }] }] },
      orderBy: [{ level: "ASC" }], page: { limit: 100, offset: 0 },
    });
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "Time" }));
    await vi.waitFor(() => expect(view.router?.state.location.search).toMatchObject({ q: "unrelated", tab: "details" }));
  });

  it("does not load additional network batches during print", async () => {
    const view = mount({ more: true });
    await screen.findByText(log.message);
    await vi.waitFor(() => expect(intersections.size).toBeGreaterThan(0));
    act(() => window.dispatchEvent(new Event("beforeprint")));
    const before = view.queries.mock.calls.length;
    revealEnd();
    expect(view.queries).toHaveBeenCalledTimes(before);
    expect(screen.queryByText("Older log entry")).not.toBeInTheDocument();
    act(() => window.dispatchEvent(new Event("afterprint")));
    await vi.waitFor(() => expect(intersections.size).toBeGreaterThan(0));
    revealEnd();
    expect(await screen.findByText("Older log entry")).toBeInTheDocument();
  });

  it("requests generated camelCase variables and opens full details from the real table with Enter", async () => {
    const user = userEvent.setup();
    const { queries, container } = mount();
    await vi.waitFor(() => expect(queries).toHaveBeenCalled());
    const message = await screen.findByText(log.message);
    expect(queries).toHaveBeenCalledWith({ where: {}, page: { limit: 100, after: null, includeTotalCount: true } });
    expect(container.querySelector('[data-height="fill"]')).toBeInTheDocument();
    const row = message.closest('[role="row"]')! as HTMLElement;
    row.focus();
    await user.keyboard("{Enter}");
    const dialog = await screen.findByRole("dialog", { name: "Log entry" });
    expect(within(dialog).getByText(log.fields!)).toBeInTheDocument();
    expect(within(dialog).getByText("Span: scan (span-1)")).toBeInTheDocument();
    await user.keyboard("{Escape}");
    await vi.waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await vi.waitFor(() => expect(row).toHaveFocus());
  });

  it("appends server batches without page controls and restarts after sort/search", async () => {
    const user = userEvent.setup();
    const view = mount({ more: true });
    await screen.findByText(log.message);
    expect(screen.queryByRole("button", { name: "Next page" })).not.toBeInTheDocument();
    expect(screen.queryByRole("combobox", { name: "Rows per page" })).not.toBeInTheDocument();
    await vi.waitFor(() => expect(intersections.size).toBeGreaterThan(0));
    revealEnd();
    await vi.waitFor(() => expect(view.queries).toHaveBeenLastCalledWith({ where: {}, page: { limit: 100, after: "head", includeTotalCount: true } }));
    expect(await screen.findByText("Older log entry")).toBeInTheDocument();
    expect(screen.getByText(log.message)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Time" }));
    await vi.waitFor(() => expect(view.queries).toHaveBeenLastCalledWith({ where: {}, orderBy: [{ timestamp: "ASC" }], page: { limit: 100, offset: 0 } }));
    await vi.waitFor(() => expect(screen.queryByText("Older log entry")).not.toBeInTheDocument());
    if (!screen.queryByRole("searchbox", { name: "Search entries" })) await user.click(screen.getByRole("button", { name: "Search entries" }));
    await user.type(await screen.findByRole("searchbox", { name: "Search entries" }), "Movie");
    await vi.waitFor(() => expect(view.queries).toHaveBeenLastCalledWith({ where: { or: [{ message: { contains: "Movie" } }, { target: { contains: "Movie" } }] }, orderBy: [{ timestamp: "ASC" }], page: { limit: 100, offset: 0 } }));
  });

  it.each(["manual", "live"] as const)("rearms an invalidated append after a %s equal-count refresh without resetting scroll", async (refresh) => {
    const user = userEvent.setup();
    const view = mount({ more: true });
    await screen.findByText(log.message);
    const scroller = view.container.querySelector<HTMLElement>(".fa-dt-scroll")!;
    scroller.scrollTop = 400;
    await vi.waitFor(() => expect(intersections.size).toBeGreaterThan(0));

    // Consume the loader's current count/key, but leave that batch in flight.
    // Refreshing the one-row window invalidates it without changing the count.
    view.holdNextAppend();
    revealEnd();
    await vi.waitFor(() => expect(view.queries).toHaveBeenCalledTimes(2));
    expect(view.queries).toHaveBeenLastCalledWith({ where: {}, page: { limit: 100, after: "head", includeTotalCount: true } });
    if (refresh === "manual") await user.click(screen.getByRole("button", { name: "Refresh system log" }));
    else await act(async () => { view.emitChange(); });
    await vi.waitFor(() => {
      expect(view.queries).toHaveBeenCalledTimes(3);
      expect(view.queries).toHaveBeenLastCalledWith({ where: {}, page: { limit: 100, after: null, includeTotalCount: true } });
      expect(view.container.querySelector(".fa-dt")).not.toHaveAttribute("data-refetching");
    });
    expect(screen.getByText(log.message)).toBeInTheDocument();
    expect(screen.queryByText("Older log entry")).not.toBeInTheDocument();
    expect(scroller.scrollTop).toBe(400);

    // The stale response must stay discarded. Its finally path releases the
    // loader, which must accept the same row count again in the new revision.
    await act(async () => { view.releaseAppend(); });
    expect(screen.queryByText("Older log entry")).not.toBeInTheDocument();
    await vi.waitFor(() => expect(intersections.size).toBeGreaterThan(0));
    revealEnd();
    expect(await screen.findByText("Older log entry")).toBeInTheDocument();
    expect(view.queries).toHaveBeenCalledTimes(4);
    expect(view.queries).toHaveBeenLastCalledWith({ where: {}, page: { limit: 100, after: "head", includeTotalCount: true } });
    expect(view.container.querySelector(".fa-dt-scroll")).toBe(scroller);
    expect(scroller.scrollTop).toBe(400);
  });

  it("applies the library level filter to the server", async () => {
    const user = userEvent.setup();
    const view = mount();
    await screen.findByText(log.message);
    await user.click(screen.getByRole("button", { name: "Filters" }));
    await user.click(await screen.findByRole("menuitem", { name: "Level" }));
    const editor = await screen.findByRole("dialog", { name: "Level" });
    await user.click(within(editor).getByRole("checkbox", { name: "WARN" }));
    await vi.waitFor(() => expect(view.queries).toHaveBeenLastCalledWith({ where: { level: { inList: ["WARN"] } }, page: { limit: 100, after: null, includeTotalCount: true } }));
  });

  it("uses the existing batch-size preference without exposing page controls", async () => {
    window.localStorage.setItem("librarian.pref.table.logs.pageSize", "50");
    const view = mount();
    await screen.findByText(log.message);
    expect(view.queries).toHaveBeenLastCalledWith({ where: {}, page: { limit: 50, after: null, includeTotalCount: true } });
    expect(screen.queryByRole("combobox", { name: "Rows per page" })).not.toBeInTheDocument();
    expect(window.localStorage.getItem("librarian.pref.table.logs.pageSize")).toBe("50");
  });

  it("requires confirmation and Cancel never sends a deletion", async () => {
    const user = userEvent.setup();
    const { deletes } = mount();
    await screen.findByText(log.message);
    await user.click(screen.getByRole("button", { name: "Clear system log" }));
    const dialog = await screen.findByRole("alertdialog");
    expect(within(dialog).getByText(/outside the current filters/)).toBeInTheDocument();
    expect(deletes).not.toHaveBeenCalled();
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(deletes).not.toHaveBeenCalled();
  });

  it("clears with an explicit non-empty predicate and refetches the empty first page", async () => {
    const user = userEvent.setup();
    const { deletes, queries } = mount();
    await screen.findByText(log.message);
    await user.click(screen.getByRole("button", { name: "Clear system log" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: /^Clear all logs/ }));
    await vi.waitFor(() => expect(deletes).toHaveBeenCalledWith({ where: { id: { isNull: false } } }));
    expect(await screen.findByText("No log entries")).toBeInTheDocument();
    expect(queries).toHaveBeenLastCalledWith({ where: {}, page: { limit: 100, after: null, includeTotalCount: true } });
  });

  it("keeps confirmation open and reports a failed mutation payload", async () => {
    const user = userEvent.setup();
    const { queries } = mount({ deleteSuccess: false });
    await screen.findByText(log.message);
    const before = queries.mock.calls.length;
    await user.click(screen.getByRole("button", { name: "Clear system log" }));
    await user.click(within(await screen.findByRole("alertdialog")).getByRole("button", { name: /^Clear all logs/ }));
    expect((await screen.findAllByText("Log store unavailable")).length).toBeGreaterThan(0);
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
    expect(queries).toHaveBeenCalledTimes(before);
  });

  it("keeps loaded entries readable after a failed live refresh and can retry", async () => {
    const user = userEvent.setup();
    const view = mount();
    await screen.findByText(log.message);
    view.setQueryFailure(true);
    await act(async () => { view.emitChange(); });
    expect(await screen.findByText("Refresh failed. Showing loaded entries.")).toBeInTheDocument();
    expect(screen.getByText(log.message)).toBeInTheDocument();
    view.setQueryFailure(false);
    await user.click(screen.getByRole("button", { name: "Refresh system log" }));
    await vi.waitFor(() => expect(screen.queryByText("Refresh failed. Showing loaded entries.")).not.toBeInTheDocument());
    expect(screen.getByText(log.message)).toBeInTheDocument();
  });

  it("holds new live entries behind the library count while the table is scrolled", async () => {
    const user = userEvent.setup();
    const view = mount();
    await screen.findByText(log.message);
    const scroller = view.container.querySelector(".fa-dt-scroll")!;
    scroller.scrollTop = 400;
    view.addLog();
    await act(async () => { view.emitChange(); });
    const reveal = await screen.findByRole("button", { name: "1 new" });
    expect(screen.queryByText("New live entry")).not.toBeInTheDocument();
    await user.click(reveal);
    expect(await screen.findByText("New live entry")).toBeInTheDocument();
  });

  it("refetches live changes and stops while paused", async () => {
    const user = userEvent.setup();
    const view = mount();
    await screen.findByText(log.message);
    const before = view.queries.mock.calls.length;
    await act(async () => { view.emitChange(); });
    await vi.waitFor(() => expect(view.queries.mock.calls.length).toBeGreaterThan(before));
    await user.click(screen.getByRole("button", { name: "Live" }));
    const paused = view.queries.mock.calls.length;
    await act(async () => { view.emitChange(); });
    expect(view.queries).toHaveBeenCalledTimes(paused);
    expect(screen.getByRole("button", { name: "Paused" })).toHaveAttribute("aria-pressed", "false");
  });
});
