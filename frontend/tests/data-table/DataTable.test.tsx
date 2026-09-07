// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { DataTable, type DataTableColumn } from "../../src/components/data-table";

const proTableMock = vi.hoisted(() => ({
  props: undefined as Record<string, unknown> | undefined,
}));

vi.mock("data-table-pro/heroui", () => ({
  DataTable: (props: Record<string, unknown> & { children?: ReactNode }) => {
    proTableMock.props = props;
    return <div data-testid="pro-data-table">{props.children}</div>;
  },
}));

interface TestRow {
  id: number;
  name: string;
  score: number;
  status: "active" | "archived";
}

const rows: TestRow[] = [
  { id: 1, name: "Alpha", score: 20, status: "active" },
  { id: 2, name: "Beta", score: 10, status: "archived" },
];

const columns: DataTableColumn<TestRow>[] = [
  {
    key: "name",
    label: "Name",
    width: { width: 220, minWidth: 160, maxWidth: 280, resizable: false },
    hideOnMobile: true,
    render: (row) => row.name.toUpperCase(),
  },
  {
    key: "score",
    label: "Score",
    align: "end",
    truncate: false,
    sortFn: (a, b) => a.score - b.score,
  },
  {
    key: "status",
    label: "Status",
    hidden: true,
  },
];

function latestProps() {
  if (!proTableMock.props) {
    throw new Error("data-table-pro mock was not rendered");
  }
  return proTableMock.props;
}

describe("DataTable adapter", () => {
  beforeEach(() => {
    proTableMock.props = undefined;
  });

  it("maps legacy columns, row ids, and layout props into data-table-pro props", () => {
    render(
      <DataTable
        data={rows}
        columns={columns}
        getRowKey={(row) => row.id}
        headerContent="Movies"
        footerContent={<span>Footer</span>}
        toolbarQueryPlaceholder="Search movies"
        defaultPageSize={25}
        pageSizeOptions={[10, 25, 50]}
        serverTotalCount={100}
        classNames={{ wrapper: "wrapper", table: "table", tableContainer: "container" }}
        ariaLabel="Movies table"
      />,
    );

    const props = latestProps();
    const mappedColumns = props.columns as Array<Record<string, unknown>>;

    expect(screen.getByTestId("pro-data-table").textContent).toContain("Footer");
    expect(mappedColumns).toHaveLength(2);
    expect(mappedColumns[0]).toMatchObject({
      id: "name",
      accessorKey: "name",
      header: "Name",
      size: 220,
      minSize: 160,
      maxSize: 280,
      enableResizing: false,
      enableSorting: true,
    });
    expect(mappedColumns[0].meta).toMatchObject({ hideOn: "md", minWidth: 160 });
    expect(mappedColumns[1].meta).toMatchObject({
      align: "end",
      cellClassName: "whitespace-normal",
    });
    expect(mappedColumns[1].size).toBeUndefined();
    expect((mappedColumns[0].cell as (args: unknown) => ReactNode)({
      row: { original: rows[0], index: 0 },
    })).toBe("ALPHA");
    expect((mappedColumns[1].sortFn as (a: unknown, b: unknown) => number)(
      { original: rows[0] },
      { original: rows[1] },
    )).toBe(10);
    expect((props.getRowId as (row: TestRow) => string)(rows[0])).toBe("1");
    expect(screen.getByText("Movies")).toBeTruthy();
    expect(props.toolbarQueryPlaceholder).toBe("Search movies");
    expect(props.rowsPerPageOptions).toEqual([10, 25, 50]);
    expect(props.pageSize).toBe(25);
    expect(props.totalRowCount).toBe(100);
    expect(screen.getByTestId("pro-data-table").parentElement?.className).toBe("wrapper");
    expect(props.tableClassName).toBe("table");
    expect(props.tableContainerClassName).toBe("container");
    expect(props.stickyHeader).toBe(false);
    expect(props.flexGrow).toBe(false);
    expect(props.layoutMode).toBe("fill");
    expect(props["aria-label"]).toBe("Movies table");
  });

  it("maps sorting, selection, actions, cards, and infinite loading callbacks", () => {
    const onSortChange = vi.fn();
    const onSelectionChange = vi.fn();
    const onViewModeChange = vi.fn();
    const onLoadMore = vi.fn();
    const onSearchChange = vi.fn();
    const rowAction = vi.fn();
    const bulkAction = vi.fn();
    const onRowClick = vi.fn();
    const cardRenderer = vi.fn(({ item, isSelected, onSelect }) => (
      <button type="button" onClick={onSelect}>
        {item.name}:{String(isSelected)}
      </button>
    ));

    render(
      <DataTable
        data={rows}
        columns={columns}
        getRowKey={(row) => row.id}
        selectionMode="multiple"
        selectedKeys={new Set([1])}
        onSelectionChange={onSelectionChange}
        sortColumn="score"
        sortDirection="desc"
        onSortChange={onSortChange}
        serverSide
        onSearchChange={onSearchChange}
        showViewModeToggle
        viewMode="cards"
        onViewModeChange={onViewModeChange}
        cardRenderer={cardRenderer}
        rowActions={[
          {
            key: "delete",
            label: "Delete",
            color: "danger",
            onAction: rowAction,
            isVisible: (row) => row.status === "active",
          },
        ]}
        bulkActions={[
          {
            key: "archive",
            label: "Archive",
            onAction: bulkAction,
            disabled: (selectedRows) => selectedRows.length === 0,
          },
        ]}
        onRowClick={onRowClick}
        fillHeight
        paginationMode="infinite"
        hasMore
        isLoadingMore
        onLoadMore={onLoadMore}
      />,
    );

    const props = latestProps();
    expect(props.flexGrow).toBe(true);
    expect(props.stickyHeader).toBe(true);
    expect(props.className).toContain("h-0");
    expect(props.className).toContain("min-h-0");
    expect(props.className).toContain("flex-1");
    expect(props.cardSizing).toBe("fluid");
    expect(props.manualSorting).toBe(true);
    expect(props.sorting).toEqual([{ id: "score", desc: true }]);
    expect(props.rowSelection).toEqual({ "1": true });
    expect(props.viewMode).toBe("card");
    expect(props.enableViewToggle).toBe(true);
    expect(props.infiniteScroll).toMatchObject({
      enabled: true,
      hasMore: true,
      isLoadingMore: true,
    });

    (props.onSortingChange as (sorting: Array<{ id: string; desc: boolean }>) => void)([
      { id: "name", desc: false },
    ]);
    expect(onSortChange).toHaveBeenCalledWith("name", "asc");

    (props.onRowSelectionChange as (selection: Record<string, boolean>) => void)({
      "1": false,
      "2": true,
    });
    expect(onSelectionChange).toHaveBeenCalledWith(new Set(["2"]));

    (props.onViewModeChange as (mode: "card" | "table") => void)("table");
    expect(onViewModeChange).toHaveBeenCalledWith("table");

    (props.onToolbarQueryValueChange as (value: string) => void)("matrix");
    expect(onSearchChange).toHaveBeenCalledWith("matrix");

    const rowActions = props.rowActions as Array<Record<string, unknown>>;
    expect(rowActions[0]).toMatchObject({ key: "delete", label: "Delete", variant: "destructive" });
    expect((rowActions[0].hidden as (row: TestRow) => boolean)(rows[0])).toBe(false);
    expect((rowActions[0].hidden as (row: TestRow) => boolean)(rows[1])).toBe(true);
    (rowActions[0].onClick as (row: TestRow) => void)(rows[0]);
    expect(rowAction).toHaveBeenCalledWith(rows[0]);

    const selectionActions = props.selectionActions as Array<Record<string, unknown>>;
    expect((selectionActions[0].disabled as (selectedRows: TestRow[]) => boolean)([])).toBe(true);
    (selectionActions[0].onClick as (args: { rows: TestRow[] }) => void)({ rows });
    expect(bulkAction).toHaveBeenCalledWith(rows);

    (props.onRowClick as (args: { row: TestRow }) => void)({ row: rows[0] });
    expect(onRowClick).toHaveBeenCalledWith(rows[0]);

    (props.infiniteScroll as { onLoadMore: () => void }).onLoadMore();
    expect(onLoadMore).toHaveBeenCalledOnce();

    const onSelectedChange = vi.fn();
    render(
      (props.cardRenderer as (args: {
        row: TestRow;
        isSelected: boolean;
        onSelectedChange: (selected: boolean) => void;
      }) => ReactElement)({
        row: rows[0],
        isSelected: false,
        onSelectedChange,
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Alpha:false" }));
    expect(onSelectedChange).toHaveBeenCalledWith(true);
    expect(cardRenderer).toHaveBeenCalledWith(
      expect.objectContaining({
        item: rows[0],
        index: 0,
        isSelected: false,
      }),
    );
  });
});
