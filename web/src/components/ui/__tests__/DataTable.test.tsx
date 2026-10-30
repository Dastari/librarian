import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import { useState, type ReactElement, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { DataTable, type DataTableBulkAction, type DataTableColumn, type DataTableRowAction } from "../DataTable";

interface Row { id: string; title: string; year: number }
const rows: Row[] = [
  { id: "1", title: "Patriot Games", year: 1992 },
  { id: "2", title: "The Hunt for Red October", year: 1990 },
];
const columns: Array<DataTableColumn<Row>> = [
  { id: "title", header: "Title", sortable: true, cell: (row) => row.title },
  { id: "year", header: "Year", cell: (row) => row.year, numeric: true, size: 96, align: "end", hideBelow: "md" },
];
const getRowId = (row: Row) => row.id;
const bulkActions: Array<DataTableBulkAction<Row>> = [{ key: "delete", label: "Delete", destructive: true, onAction: () => {} }];
let width = 1200;

function mount(element: ReactElement) {
  return render(element, { wrapper: ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider> });
}

beforeEach(() => {
  width = 1200;
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width, height: 700, top: 0, left: 0, bottom: 700, right: width, x: 0, y: 0, toJSON: () => ({}) };
  });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : width; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 700; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(() => width);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(700);
});
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });

/** Observe the real Glass end-of-list element, then move it into view deterministically. */
function observeEnd() {
  const callbacks: IntersectionObserverCallback[] = [];
  const observe = vi.fn();
  vi.stubGlobal("IntersectionObserver", class {
    constructor(callback: IntersectionObserverCallback) { callbacks.push(callback); }
    observe = observe;
    unobserve = () => {};
    disconnect = () => {};
  });
  return { observe, intersect: () => act(() => {
    callbacks.at(-1)?.([{ isIntersecting: true } as IntersectionObserverEntry], {} as IntersectionObserver);
  }) };
}

async function searchBox() {
  if (!screen.queryByRole("searchbox")) await userEvent.click(screen.getByRole("button", { name: /^Search / }));
  return screen.findByRole("searchbox");
}

function expectNoPager() {
  expect(screen.queryByRole("button", { name: /next page|previous page|first page|last page/i })).toBeNull();
  expect(screen.queryByRole("combobox", { name: /rows per page/i })).toBeNull();
}

describe("Glass compatibility table", () => {
  it("prints the full filtered in-memory snapshot and restores its reveal window", async () => {
    const many = Array.from({ length: 131 }, (_, index) => ({ id: String(index), title: `Print recording ${index + 1}`, year: 2000 }));
    mount(<DataTable columns={columns} rows={many} getRowId={getRowId} initialBatchSize={50} />);
    expect(screen.queryByText("Print recording 131")).toBeNull();
    act(() => window.dispatchEvent(new Event("beforeprint")));
    expect(await screen.findByText("Print recording 131")).toBeInTheDocument();
    act(() => window.dispatchEvent(new Event("afterprint")));
    await waitFor(() => expect(screen.queryByText("Print recording 131")).toBeNull());
    await userEvent.type(await searchBox(), "Print recording 131");
    await waitFor(() => expect(screen.queryByText("Print recording 1")).toBeNull());
    act(() => window.dispatchEvent(new Event("beforeprint")));
    expect(await screen.findByText("Print recording 131")).toBeInTheDocument();
    expect(screen.queryByText("Print recording 1")).toBeNull();
    act(() => window.dispatchEvent(new Event("afterprint")));
  });

  it("does not fetch unloaded server pages while printing", async () => {
    const end = observeEnd();
    const loadMore = vi.fn();
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} infinite={{ hasMore: true, loadMore }} />);
    act(() => window.dispatchEvent(new Event("beforeprint")));
    end.intersect();
    expect(loadMore).not.toHaveBeenCalled();
    act(() => window.dispatchEvent(new Event("afterprint")));
  });

  it("renders the public grid and converts legacy pixel widths to rem", () => {
    const { container } = mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} />);
    expect(screen.getByRole("grid", { name: "items" })).toBeInTheDocument();
    expect(screen.getByText("Patriot Games")).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /Year/ })).toBeInTheDocument();
    // 96 legacy CSS px is 6rem, not 96rem. A fixed-width column does not grow.
    expect(container.querySelector(".fa-dt")?.getAttribute("style")).toContain("6rem");
    expectNoPager();
  });

  it.each([390, 740])("turns records into readable Glass cards in a %ipx container", (containerWidth) => {
    width = containerWidth;
    const { container } = mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} />);
    expect(container.querySelector("[data-mode='cards']")).toBeTruthy();
    expect(screen.getByText("Patriot Games")).toBeInTheDocument();
    expect(screen.getAllByText("Year").length).toBeGreaterThan(0);
    expect(screen.getByText("1992")).toBeInTheDocument();
    expectNoPager();
  });

  it("fills a bounded pane without changing embedded tables' auto height", () => {
    const { container, rerender } = mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} height="fill" />);
    expect(container.querySelector(".fa-dt")).toHaveAttribute("data-height", "fill");
    expect(container.querySelector(".fa-dt-footer")).toBeInTheDocument();
    rerender(<DataTable columns={columns} rows={rows} getRowId={getRowId} />);
    expect(container.querySelector(".fa-dt")).toHaveAttribute("data-height", "auto");
  });

  it("synchronizes a programmatically focused row with Glass's retained active row", async () => {
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} height="fill" />);
    const first = screen.getByText(rows[0]!.title).closest('[role="row"]')!;
    const later = screen.getByText(rows[1]!.title).closest('[role="row"]')!;
    expect(first).toHaveAttribute("tabindex", "0");
    act(() => (later as HTMLElement).focus());
    await waitFor(() => expect(later).toHaveAttribute("tabindex", "0"));
    expect(later).toHaveFocus();
    expect(first).toHaveAttribute("tabindex", "-1");
  });

  it("keeps the gallery count outside its keyboard-scrollable results", () => {
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} height="fill" view="card" renderCard={(row) => <article>{row.title} poster</article>} />);
    const region = screen.getByRole("region", { name: "items results" });
    expect(region).toHaveAttribute("tabindex", "0");
    expect(region).toContainElement(screen.getByText("Patriot Games poster"));
    const footer = screen.getByRole("status");
    expect(footer).toHaveTextContent(`${rows.length} items`);
    expect(region).not.toContainElement(footer);
  });

  it("folds low-priority fields into reachable card details", async () => {
    width = 390;
    const detailColumns: Array<DataTableColumn<Row>> = [...columns, { id: "extra", header: "Extra", cell: () => "Archive notes", hideBelow: "xl" }];
    mount(<DataTable columns={detailColumns} rows={[rows[0]!]} getRowId={getRowId} />);
    expect(screen.queryByText("Archive notes")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: /show 1 more fields/i }));
    expect(screen.getByText("Archive notes")).toBeInTheDocument();
  });

  it("renders directly on the page unless a surface is explicitly requested", () => {
    const { container, rerender } = mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} />);
    expect(container.querySelector(".glass-panel")).toBeNull();
    expect(screen.getByRole("grid")).toBeInTheDocument();
    rerender(<DataTable columns={columns} rows={rows} getRowId={getRowId} frame />);
    expect(container.querySelector(".glass-panel")).toBeTruthy();
    rerender(<DataTable columns={columns} rows={rows} getRowId={getRowId} frame={false} />);
    expect(container.querySelector(".glass-panel")).toBeNull();
    expect(screen.getByRole("grid")).toBeInTheDocument();
  });

  it("keeps loading, empty and error states inside the Glass grid", () => {
    const { rerender } = mount(<DataTable columns={columns} rows={[]} getRowId={getRowId} isLoading />);
    expect(screen.getByRole("grid")).toHaveAttribute("aria-busy", "true");
    expect(screen.queryByText("Nothing here yet")).toBeNull();
    rerender(<DataTable columns={columns} rows={[]} getRowId={getRowId} emptyState={<p>No movies match</p>} />);
    expect(within(screen.getByRole("grid")).getByText("No movies match")).toBeInTheDocument();
    rerender(<DataTable columns={columns} rows={rows} getRowId={getRowId} error={<p role="alert">Could not load</p>} />);
    expect(within(screen.getByRole("grid")).getByRole("alert")).toHaveTextContent("Could not load");
  });

  it("mounts search with inferred filters after initial loading at phone width", async () => {
    width = 322;
    const { rerender } = mount(<DataTable columns={columns} rows={[]} getRowId={getRowId} isLoading />);
    expect(screen.queryByRole("button", { name: "Search items" })).toBeNull();
    expect(screen.queryByRole("searchbox")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Filters/ })).toBeNull();
    rerender(<DataTable columns={columns} rows={rows} getRowId={getRowId} />);
    expect(screen.getByRole("button", { name: /^Filters/ })).toBeInTheDocument();
    expect(await searchBox()).toBeInTheDocument();
  });

  it("preserves the caller's poster grid instead of rendering Glass record cards", () => {
    width = 390;
    const { container } = mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} view="card" renderCard={(row) => <article>{row.title} poster</article>} />);
    expect(screen.getByText("Patriot Games poster")).toBeInTheDocument();
    expect(container.querySelector(".poster-grid")).toBeTruthy();
    expect(screen.queryByRole("grid")).toBeNull();
    expectNoPager();
  });

  it("keeps focused cell controls mounted when caller columns and closures update", async () => {
    const firstAction = vi.fn();
    const nextAction = vi.fn();
    const makeColumns = (onClick: () => void): Array<DataTableColumn<Row>> => [{ id: "title", header: "Title", cell: (row) => <button onClick={onClick}>{row.title}</button> }];
    const { rerender } = mount(<DataTable columns={makeColumns(firstAction)} rows={rows} getRowId={getRowId} />);
    const control = screen.getByRole("button", { name: "Patriot Games" });
    control.focus();
    rerender(<DataTable columns={makeColumns(nextAction)} rows={rows} getRowId={getRowId} />);
    expect(screen.getByRole("button", { name: "Patriot Games" })).toBe(control);
    expect(control).toHaveFocus();
    await userEvent.keyboard("{Enter}");
    expect(nextAction).toHaveBeenCalledOnce();
    expect(firstAction).not.toHaveBeenCalled();
  });
});

describe("complete collection filtering and infinite rendering", () => {
  it("searches the entire collection before revealing chunks, including formatted cell labels", async () => {
    const allRows = Array.from({ length: 80 }, (_, index) => ({ id: String(index), title: `Movie ${index}`, year: 2000 + index }));
    const richerColumns: Array<DataTableColumn<Row>> = [...columns, { id: "computed", header: "Collection", cell: (row) => <span>{row.id === "79" ? "Archive selection" : "Catalogue"}</span> }];
    mount(<DataTable columns={richerColumns} rows={allRows} getRowId={getRowId} initialBatchSize={10} noun="movies" />);
    expect(screen.queryByText("Movie 79")).toBeNull();
    await userEvent.type(await searchBox(), "Archive selection");
    expect(await screen.findByText("Movie 79")).toBeInTheDocument();
    expect(screen.queryByText("Movie 0")).toBeNull();
    expectNoPager();
  });

  it("supports explicit searchable labels for composed domain cells", async () => {
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} getSearchText={(row) => row.id === "2" ? "submarine thriller" : "spy thriller"} />);
    await userEvent.type(await searchBox(), "submarine");
    expect(await screen.findByText(rows[1]!.title)).toBeInTheDocument();
    await waitFor(() => expect(screen.queryByText(rows[0]!.title)).toBeNull());
  });

  it("uses numeric column filters before chunking the complete collection", async () => {
    mount(<DataTable columns={columns} rows={rows.toReversed()} getRowId={getRowId} initialBatchSize={1} />);
    await userEvent.click(screen.getByRole("button", { name: /^Filters/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Year" }));
    // Glass's public filter editor owns its operator, value and apply controls.
    await userEvent.type(await screen.findByLabelText("Year value"), "1991");
    await userEvent.click(screen.getByRole("button", { name: /apply/i }));
    expect(await screen.findByText(rows[0]!.title)).toBeInTheDocument();
    expect(screen.queryByText(rows[1]!.title)).toBeNull();
  });

  it("requires a value accessor for opaque status components and includes it in search", async () => {
    function Status({ status }: { status: { label: string } }) { return <span>{status.label}</span>; }
    const statusColumn: DataTableColumn<Row> = { id: "status", header: "Status", cell: (row) => <Status status={{ label: row.id === "2" ? "Ready" : "Waiting" }} /> };
    const { rerender } = mount(<DataTable columns={[...columns, statusColumn]} rows={rows} getRowId={getRowId} />);
    await userEvent.click(screen.getByRole("button", { name: /^Filters/ }));
    expect(screen.queryByRole("menuitem", { name: "Status" })).toBeNull();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(screen.queryByRole("menu", { hidden: true })).toBeNull());
    rerender(<DataTable columns={[...columns, { ...statusColumn, value: (row) => row.id === "2" ? "Ready" : "Waiting" }]} rows={rows} getRowId={getRowId} />);
    await userEvent.click(screen.getByRole("button", { name: /^Filters/ }));
    await waitFor(() => expect(screen.getByRole("menuitem", { name: "Status" })).toBeInTheDocument());
    await userEvent.keyboard("{Escape}");
    await userEvent.type(await searchBox(), "Ready");
    await waitFor(() => expect(screen.queryByText(rows[0]!.title)).toBeNull());
    expect(screen.getByText(rows[1]!.title)).toBeInTheDocument();
  });

  it("keeps composite numeric presentations as text unless the caller supplies typed values", async () => {
    const usageColumns: Array<DataTableColumn<Row>> = [...columns, { id: "uses", header: "Uses", numeric: true, cell: () => "3/∞" }];
    mount(<DataTable columns={usageColumns} rows={rows} getRowId={getRowId} />);
    await userEvent.click(screen.getByRole("button", { name: /^Filters/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Uses" }));
    expect(await screen.findByRole("textbox", { name: "Uses text" })).toBeInTheDocument();
    expect(screen.queryByLabelText("Uses value")).toBeNull();
  });

  it("reveals another local chunk when the actual Glass sentinel enters view", () => {
    const end = observeEnd();
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} initialBatchSize={1} />);
    expect(screen.queryByText(rows[1]!.title)).toBeNull();
    expect(end.observe).toHaveBeenCalled();
    end.intersect();
    expect(screen.getByText(rows[1]!.title)).toBeInTheDocument();
    expectNoPager();
  });

  it("sorts complete collections locally while preserving the controlled sorting callback", async () => {
    const onSortingChange = vi.fn();
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} onSortingChange={onSortingChange} initialBatchSize={1} />);
    const title = screen.getByRole("columnheader", { name: /Title/ });
    await userEvent.click(within(title).getByRole("button", { name: "Title" }));
    expect(onSortingChange).toHaveBeenLastCalledWith([{ id: "title", desc: false }]);
    await userEvent.click(within(title).getByRole("button", { name: "Title" }));
    expect(onSortingChange).toHaveBeenLastCalledWith([{ id: "title", desc: true }]);
    expect(screen.getByText(rows[1]!.title)).toBeInTheDocument();
  });
});

describe("server infinite collection boundary", () => {
  it("does not offer a misleading local filter for a server batch or external browser toolbar", () => {
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} externalBrowserToolbar infinite={{ hasMore: false, loadMore: () => {} }} />);
    expect(screen.queryByRole("searchbox")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Filters/ })).toBeNull();
    expectNoPager();
  });

  it("forwards server search without filtering downloaded rows", async () => {
    const onSearch = vi.fn();
    function ServerTable() {
      const [value, setValue] = useState("");
      return <DataTable columns={columns} rows={rows} getRowId={getRowId} dataSource="server-infinite" search={{ value, onChange: (next) => { onSearch(next); setValue(next); } }} />;
    }
    mount(<ServerTable />);
    await userEvent.type(await searchBox(), "server query");
    await waitFor(() => expect(onSearch).toHaveBeenLastCalledWith("server query"));
    expect(screen.getByText(rows[0]!.title)).toBeInTheDocument();
    expect(screen.getByText(rows[1]!.title)).toBeInTheDocument();
  });

  it("requests a server batch only once and waits while loading", () => {
    const end = observeEnd();
    const loadMore = vi.fn();
    const { rerender } = mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} infinite={{ hasMore: true, loadMore, loadingMore: true }} />);
    expect(end.observe).not.toHaveBeenCalled();
    rerender(<DataTable columns={columns} rows={rows} getRowId={getRowId} infinite={{ hasMore: true, loadMore }} />);
    end.intersect();
    end.intersect();
    expect(loadMore).toHaveBeenCalledOnce();
  });

  it("keeps loaded rows and offers Retry after a failed server batch", async () => {
    const end = observeEnd();
    const loadMore = vi.fn().mockRejectedValueOnce(new Error("offline")).mockResolvedValue(undefined);
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} infinite={{ hasMore: true, loadMore }} />);
    end.intersect();
    await userEvent.click(await screen.findByRole("button", { name: /Retry/ }));
    expect(loadMore).toHaveBeenCalledTimes(2);
    expect(screen.getByText(rows[0]!.title)).toBeInTheDocument();
  });

  it("preserves poster rows and avoids duplicate gallery requests while retrying a failed batch", async () => {
    const end = observeEnd();
    const loadMore = vi.fn().mockRejectedValueOnce(new Error("offline")).mockResolvedValue(undefined);
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} view="card" renderCard={(row) => <article>{row.title} poster</article>} infinite={{ hasMore: true, loadMore }} />);
    end.intersect();
    end.intersect();
    const retry = await screen.findByRole("button", { name: "Retry" });
    expect(loadMore).toHaveBeenCalledOnce();
    expect(screen.getByText("Patriot Games poster")).toBeInTheDocument();
    await userEvent.click(retry);
    expect(loadMore).toHaveBeenCalledTimes(2);
    expectNoPager();
  });

  it.each(["table", "card"] as const)("rearms %s infinite loading for external filters and library scopes with equal row counts", async (view) => {
    const end = observeEnd();
    const loadMore = vi.fn();
    const renderCard = (row: Row) => <article>{row.title} poster</article>;
    const common = { columns, rows, getRowId, view, renderCard, infinite: { hasMore: true, loadMore } };
    const { rerender } = mount(<DataTable {...common} queryKey={["library-a", "all"]} />);
    end.intersect();
    await waitFor(() => expect(loadMore).toHaveBeenCalledTimes(1));
    await act(async () => {});
    rerender(<DataTable {...common} rows={[...rows]} queryKey={["library-a", "wanted"]} />);
    end.intersect();
    await waitFor(() => expect(loadMore).toHaveBeenCalledTimes(2));
    await act(async () => {});
    rerender(<DataTable {...common} rows={[...rows]} queryKey={["library-b", "wanted"]} />);
    end.intersect();
    await waitFor(() => expect(loadMore).toHaveBeenCalledTimes(3));
    await act(async () => {});
    // A filter with no continuation must still invalidate the preceding request
    // if the reader returns to that previous dataset at the same row count.
    rerender(<DataTable {...common} queryKey={["library-b", "complete"]} infinite={{ hasMore: false, loadMore }} />);
    rerender(<DataTable {...common} queryKey={["library-b", "wanted"]} />);
    end.intersect();
    await waitFor(() => expect(loadMore).toHaveBeenCalledTimes(4));
  });

  it.each(["table", "card"] as const)("pauses %s loading during refresh without consuming the same-count batch", async (view) => {
    const end = observeEnd();
    const loadMore = vi.fn();
    const renderCard = (row: Row) => <article>{row.title} poster</article>;
    const common = { columns, rows, getRowId, view, renderCard, queryKey: ["library-a", "all"], infinite: { hasMore: true, loadMore } };
    const { rerender } = mount(<DataTable {...common} />);
    // Invoke the pre-refresh observer too: queued notifications can outlive cleanup.
    rerender(<DataTable {...common} refetching />);
    end.intersect();
    expect(loadMore).not.toHaveBeenCalled();
    if (view === "table") expect(screen.getByRole("grid")).toHaveAttribute("aria-busy", "true");
    rerender(<DataTable {...common} rows={[...rows]} refetching={false} />);
    end.intersect();
    await waitFor(() => expect(loadMore).toHaveBeenCalledTimes(1));
    expectNoPager();
  });

  it.each(["table", "card"] as const)("rearms a consumed %s batch after equal-count refresh while keeping selection", async (view) => {
    const end = observeEnd();
    const loadMore = vi.fn();
    const renderCard = (row: Row) => <article>{row.title} poster</article>;
    const common = { columns, rows, getRowId, view, renderCard, queryKey: ["library-a", "all"], selectable: true, bulkActions, infinite: { hasMore: true, loadMore } };
    const { rerender } = mount(<DataTable {...common} loadRevision={0} />);
    if (view === "table") await userEvent.click(screen.getByRole("checkbox", { name: "Select Patriot Games" }));
    end.intersect();
    await waitFor(() => expect(loadMore).toHaveBeenCalledTimes(1));
    await act(async () => {});
    const grid = view === "table" ? screen.getByRole("grid") : null;
    const scroller = grid?.querySelector<HTMLElement>("[data-scroll-container]");
    const focusedRow = grid ? screen.getByText("The Hunt for Red October").closest<HTMLElement>('[role="row"]') : null;
    if (scroller && focusedRow) {
      act(() => focusedRow.focus());
      scroller.scrollTop = 437;
    }
    rerender(<DataTable {...common} loadRevision={1} refetching />);
    end.intersect();
    expect(loadMore).toHaveBeenCalledTimes(1);
    rerender(<DataTable {...common} rows={[...rows]} loadRevision={1} refetching={false} />);
    end.intersect();
    await waitFor(() => expect(loadMore).toHaveBeenCalledTimes(2));
    if (view === "table") {
      expect(screen.getByRole("checkbox", { name: "Select Patriot Games" })).toBeChecked();
      expect(scroller?.scrollTop).toBe(437);
      expect(focusedRow).toHaveFocus();
      expect(focusedRow).toHaveAttribute("tabindex", "0");
    }
  });

  it("preserves selection on append, clears it and Glass's target cache across server filters", async () => {
    const onAction = vi.fn();
    const common = { columns, getRowId, selectable: true, bulkActions: [{ ...bulkActions[0]!, onAction }], infinite: { hasMore: false, loadMore: () => {} } };
    const { rerender } = mount(<DataTable {...common} rows={[rows[0]!]} queryKey="all" />);
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Patriot Games" }));
    rerender(<DataTable {...common} rows={rows} queryKey="all" />);
    expect(screen.getByRole("checkbox", { name: "Select Patriot Games" })).toBeChecked();
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(onAction).toHaveBeenLastCalledWith([rows[0]]);
    const third = { id: "3", title: "Clear and Present Danger", year: 1994 };
    rerender(<DataTable {...common} rows={[rows[1]!, third]} queryKey="wanted" />);
    expect(screen.queryByRole("button", { name: "Delete" })).toBeNull();
    expect(screen.getByRole("checkbox", { name: "Select The Hunt for Red October" })).not.toBeChecked();
    await userEvent.click(screen.getByRole("checkbox", { name: "Select The Hunt for Red October" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(onAction).toHaveBeenLastCalledWith([rows[1]]);
    rerender(<DataTable {...common} rows={rows} queryKey="all" />);
    expect(screen.getByRole("checkbox", { name: "Select Patriot Games" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select The Hunt for Red October" })).not.toBeChecked();
  });

  it("never renders page controls even when deprecated callers still pass pagination props", () => {
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} pageIndex={1} pageSize={1} totalCount={50} onPageIndexChange={vi.fn()} onPageSizeChange={vi.fn()} />);
    expect(screen.queryByRole("searchbox")).toBeNull();
    expectNoPager();
  });
});

describe("selection, actions and keyboard", () => {
  it("selects rows with working Glass checkboxes, runs bulk actions and clears selection", async () => {
    const onAction = vi.fn();
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} selectable bulkActions={[{ ...bulkActions[0]!, onAction }]} />);
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Patriot Games" }));
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(onAction).toHaveBeenCalledWith([rows[0]]);
    await userEvent.click(screen.getByRole("button", { name: /^Clear/  }));
    expect(screen.getByRole("checkbox", { name: "Select Patriot Games" })).not.toBeChecked();
  });

  it("selects all loaded records from the header with keyboard controls", async () => {
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} selectable bulkActions={bulkActions} />);
    const all = screen.getByRole("checkbox", { name: /Select all/ });
    all.focus();
    await userEvent.keyboard(" ");
    expect(screen.getByRole("checkbox", { name: "Select Patriot Games" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Select The Hunt for Red October" })).toBeChecked();
  });

  it("preserves dynamic labels and hidden row actions", async () => {
    const onAction = vi.fn();
    const actions: Array<DataTableRowAction<Row>> = [
      { key: "refresh", label: "Refresh", onAction },
      { key: "delete", label: (row) => `Delete ${row.title}`, onAction: () => {}, destructive: true, hidden: (row) => row.id === "1" },
    ];
    mount(<DataTable columns={columns} rows={rows} getRowId={getRowId} rowActions={actions} />);
    await userEvent.click(screen.getByRole("button", { name: "Actions for Patriot Games" }));
    expect(screen.queryByRole("menuitem", { name: "Delete Patriot Games" })).toBeNull();
    await userEvent.click(await screen.findByRole("menuitem", { name: "Refresh" }));
    expect(onAction).toHaveBeenCalledWith(rows[0]);
  });

  it("opens records on click and Enter without opening on nested controls", async () => {
    const onRowClick = vi.fn();
    const controlColumns: Array<DataTableColumn<Row>> = [...columns, { id: "control", header: "Control", cell: () => <button>Preview</button> }];
    mount(<DataTable columns={controlColumns} rows={rows} getRowId={getRowId} onRowClick={onRowClick} />);
    await userEvent.click(screen.getAllByRole("button", { name: "Preview" })[0]!);
    expect(onRowClick).not.toHaveBeenCalled();
    const row = screen.getByText("Patriot Games").closest("[role='row']") as HTMLElement;
    await userEvent.click(row);
    row.focus();
    await userEvent.keyboard("{Enter}");
    expect(onRowClick).toHaveBeenCalledTimes(2);
    expect(onRowClick).toHaveBeenLastCalledWith(rows[0]);
  });
});
