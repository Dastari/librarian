import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createDataTableColumns, filters } from "glass-ux/data-table";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ActionProvider, defineActions } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactElement, ReactNode } from "react";

const mount = (element: ReactElement) => render(element, { wrapper: ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider> });

import { LoadedDataTable } from "../LoadedDataTable";

type Row = { id: string; title: string };
const column = createDataTableColumns<Row>();
const columns = column.columns([column.accessor("title", { header: "Title", cardRole: "title", filter: filters.text({ field: "title" }) })]);
const rows = Array.from({ length: 201 }, (_, index) => ({ id: String(index + 1), title: `Recording ${index + 1}` }));
const search = { fields: ["title"], placeholder: "Search recordings" };
const callbacks = new Set<IntersectionObserverCallback>();
const table = (id = "recordings", complete = true) => <LoadedDataTable id={id} label="Recordings" rows={rows} columns={columns} search={search} getRowId={(row) => row.id} complete={complete} height="auto" selectable={false} />;
async function searchBox() {
  if (!screen.queryByRole("searchbox")) await userEvent.click(screen.getByRole("button", { name: /^Search / }));
  return screen.findByRole("searchbox");
}

beforeEach(() => {
  callbacks.clear();
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const width = this.hasAttribute("data-measure") ? 110 : 1200;
    return { x: 0, y: 0, top: 0, left: 0, bottom: 800, right: width, width, height: 800, toJSON: () => ({}) };
  });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : 1200; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 800; });
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

describe("complete media table", () => {
  it("retains later-batch selection during updates and removes targets leaving an active filter", async () => {
    const action = vi.fn();
    const actions = defineActions<Row>("recording", [{ id: "act", label: "Act on selected", group: "manage", bulk: true, run: action }]);
    const element = (items: Row[]) => <LoadedDataTable id="selected-recordings" label="Recordings" rows={items} columns={columns} search={search} getRowId={(row) => row.id} getRowLabel={(row) => row.title} height="auto" selectable actions={actions} />;
    const view = mount(element(rows));
    await waitFor(() => expect(callbacks.size).toBeGreaterThan(0));
    act(() => callbacks.forEach((callback) => callback([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver)));
    await userEvent.click(await screen.findByRole("checkbox", { name: "Select Recording 101" }));
    view.rerender(element(rows.map((row) => ({ ...row }))));
    expect(screen.getByRole("checkbox", { name: "Select Recording 101" })).toBeChecked();
    await userEvent.type(await searchBox(), "Recording 101");
    await waitFor(() => expect(screen.getByRole("checkbox", { name: "Select Recording 101" })).not.toBeChecked());
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Recording 101" }));
    view.rerender(element(rows.map((row) => row.id === "101" ? { ...row, title: "Renamed outside filter" } : row)));
    await waitFor(() => expect(screen.queryByRole("button", { name: /Actions for 1 selected/ })).toBeNull());
    expect(action).not.toHaveBeenCalled();
  });
  it("prints every already loaded record and restores the screen window", async () => {
    mount(table());
    expect(screen.queryByText("Recording 201")).toBeNull();
    act(() => window.dispatchEvent(new Event("beforeprint")));
    expect(await screen.findByText("Recording 201")).toBeInTheDocument();
    expect(callbacks.size).toBe(0);
    act(() => window.dispatchEvent(new Event("afterprint")));
    await waitFor(() => expect(screen.queryByText("Recording 201")).toBeNull());
    expect(screen.getByText("Recording 1")).toBeInTheDocument();
  });

  it("filters the complete collection, including rows outside the rendered batch", async () => {
    mount(table());
    expect(screen.queryByText("Recording 201")).toBeNull();
    await userEvent.type(await searchBox(), "Recording 201");
    expect(await screen.findByText("Recording 201")).toBeVisible();
    expect(screen.queryByText("Recording 1")).toBeNull();
    expect(screen.queryByRole("button", { name: /next page|previous page/i })).toBeNull();
  });

  it("appends the next batch when the real Glass sentinel enters view", async () => {
    mount(table());
    expect(screen.queryByText("Recording 101")).toBeNull();
    await waitFor(() => expect(callbacks.size).toBeGreaterThan(0));
    act(() => callbacks.forEach((callback) => callback([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver)));
    expect(await screen.findByText("Recording 101")).toBeVisible();
    expect(screen.getByText("Recording 1")).toBeVisible();
    expect(screen.queryByText("Recording 201")).toBeNull();
    await userEvent.type(await searchBox(), "Recording 201");
    await screen.findByText("Recording 201");
    await userEvent.clear(await searchBox());
    await screen.findByText("Recording 1");
    expect(screen.queryByText("Recording 101")).toBeNull();
    await waitFor(() => expect(callbacks.size).toBeGreaterThan(0));
    act(() => callbacks.forEach((callback) => callback([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver)));
    expect(await screen.findByText("Recording 101")).toBeVisible();
  });

  it("withholds filters for incomplete rows and retains filters for an already loaded refresh", async () => {
    const view = mount(table("recordings", false));
    expect(screen.queryByRole("searchbox")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Filters/ })).toBeNull();
    view.rerender(table());
    await userEvent.type(await searchBox(), "Recording 201");
    await screen.findByText("Recording 201");
    view.rerender(table("recordings", false));
    expect(screen.getByRole("searchbox")).toHaveValue("Recording 201");
    expect(screen.getByText("Recording 201")).toBeVisible();
    view.rerender(table("another-recording"));
    expect(await searchBox()).toHaveValue("");
    expect(screen.getByText("Recording 1")).toBeVisible();
  });
});
