import { createMemoryHistory, createRootRoute, createRouter, RouterProvider } from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { createDataTableColumns, filters } from "glass-ux/data-table";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LoadedDataTable } from "../LoadedDataTable";

type Row = { id: string; title: string };
const rows: Row[] = Array.from({ length: 131 }, (_, i) => ({ id: String(i), title: `Recording ${i + 1}` }));
const col = createDataTableColumns<Row>();
const columns = col.columns([col.accessor("title", { header: "Title", sortable: true, filter: filters.text({ field: "title" }) })]);
const callbacks = new Set<IntersectionObserverCallback>();
async function mount(children: ReactNode, initial: Record<string, unknown>) {
  const history = createMemoryHistory({ initialEntries: [`/?${new URLSearchParams({ q: "unrelated", "table.records": JSON.stringify(initial) })}`] });
  const root = createRootRoute({ component: () => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider> });
  const router = createRouter({ routeTree: root, history, defaultPendingMinMs: 0 });
  await router.load();
  render(<RouterProvider router={router} />);
  return { router, history };
}
async function searchBox() {
  if (!screen.queryByRole("searchbox")) await userEvent.click(screen.getByRole("button", { name: /^(Search |Edit search:)/ }));
  return screen.findByRole("searchbox");
}
beforeEach(() => {
  callbacks.clear();
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ width: 1200, height: 800, top: 0, left: 0, bottom: 800, right: 1200, x: 0, y: 0, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : 1200; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockReturnValue(800);
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(800);
  vi.stubGlobal("IntersectionObserver", class {
    constructor(private callback: IntersectionObserverCallback) {}
    observe() { callbacks.add(this.callback); }
    unobserve() { callbacks.delete(this.callback); }
    disconnect() { callbacks.delete(this.callback); }
  });
});
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

describe("complete-table router integration", () => {
  it("rearms complete media continuation after restoring a shared query", async () => {
    const { history } = await mount(<LoadedDataTable id="records" label="Recordings" columns={columns} rows={rows} search={{ fields: ["title"] }} getRowId={(row) => row.id} height="auto" batchSize={50} />, { search: "Recording 131", sorting: [], filters: [] });
    expect(await screen.findByText("Recording 131")).toBeInTheDocument();
    await userEvent.clear(await searchBox());
    await screen.findByText("Recording 1");
    expect(screen.queryByText("Recording 51")).toBeNull();
    await waitFor(() => expect(callbacks.size).toBeGreaterThan(0));
    act(() => callbacks.forEach((callback) => callback([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver)));
    await screen.findByText("Recording 51");
    act(() => history.back());
    await screen.findByText("Recording 131");
    act(() => history.forward());
    await screen.findByText("Recording 1");
    expect(screen.queryByText("Recording 51")).toBeNull();
    await waitFor(() => expect(callbacks.size).toBeGreaterThan(0));
    act(() => callbacks.forEach((callback) => callback([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver)));
    expect(await screen.findByText("Recording 51")).toBeInTheDocument();
  });
});
