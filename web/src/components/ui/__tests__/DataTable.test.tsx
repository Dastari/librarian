import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { DataTable, type DataTableColumn } from "../DataTable";

interface Row {
  id: string;
  title: string;
  year: number;
}

const rows: Row[] = [
  { id: "1", title: "Patriot Games", year: 1992 },
  { id: "2", title: "The Hunt for Red October", year: 1990 },
];
const columns: Array<DataTableColumn<Row>> = [
  { id: "title", header: "Title", sortable: true, cell: (row) => row.title },
  { id: "year", header: "Year", cell: (row) => row.year, numeric: true },
];

afterEach(cleanup);

describe("DataTable", () => {
  it("renders rows and reports sort changes", () => {
    const onSortingChange = vi.fn();
    render(<DataTable columns={columns} rows={rows} getRowId={(row) => row.id} sorting={[]} onSortingChange={onSortingChange} />);
    expect(screen.getByText("Patriot Games")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /title/i }));
    expect(onSortingChange).toHaveBeenCalledWith([{ id: "title", desc: false }]);
  });

  it("switches to the card renderer", () => {
    render(<DataTable columns={columns} rows={rows} getRowId={(row) => row.id} view="card" renderCard={(row) => <article>{row.title} card</article>} />);
    expect(screen.getByText("Patriot Games card")).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
  });

  it("shows the empty state and pagination summary", () => {
    render(<DataTable columns={columns} rows={[]} getRowId={(row) => row.id} emptyState={<p>Nothing to see</p>} />);
    expect(screen.getByText("Nothing to see")).toBeTruthy();
    cleanup();
    render(<DataTable columns={columns} rows={rows} getRowId={(row) => row.id} pageIndex={0} pageSize={1} totalCount={2} pageSizeOptions={[1, 2]} onPageIndexChange={() => undefined} noun="movies" />);
    expect(screen.getByText("1–1 of 2 movies")).toBeTruthy();
  });
});
