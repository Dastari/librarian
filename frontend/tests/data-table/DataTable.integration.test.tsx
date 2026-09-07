// @vitest-environment jsdom

import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { Button } from "@heroui/button";
import { DataTable } from "../../src/components/data-table";

afterEach(cleanup);

describe("DataTable with the installed table library", () => {
  it("sorts rows using the application's custom comparator", async () => {
    render(
      <DataTable
        data={[
          { id: "first", name: "Alpha", score: 20 },
          { id: "second", name: "Beta", score: 10 },
        ]}
        columns={[
          { key: "name", label: "Name" },
          {
            key: "score",
            label: "Score",
            sortFn: (a, b) => b.score - a.score,
          },
        ]}
        getRowKey={(row) => row.id}
        defaultSortColumn="score"
        defaultSortDirection="desc"
      />,
    );

    const table = screen.getByRole("table");
    const names = () =>
      within(table)
        .getAllByRole("row")
        .slice(1)
        .map((row) => within(row).getAllByRole("cell")[0].textContent);

    expect(names()).toEqual(["Beta", "Alpha"]);
    fireEvent.click(within(table).getByRole("button", { name: /Score/i }));
    await waitFor(() => expect(names()).toEqual(["Alpha", "Beta"]));
  });
});

it("keeps page headings and actions visible when the standard toolbar is hidden", () => {
  render(
    <DataTable
      data={[]}
      columns={[]}
      getRowKey={() => ""}
      hideToolbar
      headerContent={<h1>Notifications</h1>}
      toolbarContent={<Button>Add torrent</Button>}
      filterRowContent={<Button>Unread</Button>}
    />,
  );
  expect(screen.getByRole("heading", { name: "Notifications" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Add torrent" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Unread" })).toBeTruthy();
});

it("does not silently truncate rows when pagination is disabled", () => {
  render(
    <DataTable
      data={Array.from({ length: 15 }, (_, id) => ({
        id,
        name: `Record ${id}`,
      }))}
      columns={[{ key: "name", label: "Name" }]}
      getRowKey={(row) => row.id}
      paginationMode="none"
    />,
  );
  expect(within(screen.getByRole("table")).getAllByRole("row")).toHaveLength(
    16,
  );
});

it("uses the application search function to match fields outside the visible columns", async () => {
  render(
    <DataTable
      data={[
        { id: 1, name: "Alpha", alias: "hiddenmatch" },
        { id: 2, name: "Beta", alias: "other" },
      ]}
      columns={[{ key: "name", label: "Name" }]}
      getRowKey={(row) => row.id}
      searchFn={(row, query) => row.alias.includes(query)}
      toolbarQueryPlaceholder="Find rows"
    />,
  );
  fireEvent.change(screen.getByPlaceholderText("Find rows"), {
    target: { value: "hiddenmatch" },
  });
  await waitFor(() => {
    const rows = within(screen.getByRole("table")).getAllByRole("row");
    expect(rows).toHaveLength(2);
    expect(rows[1].textContent).toContain("Alpha");
  });
});

it("shows an empty-state message without a wide empty table on mobile", () => {
  render(
    <DataTable
      data={[]}
      columns={[{ key: "name", label: "Name", width: 1200 }]}
      getRowKey={() => ""}
      emptyContent={<p>No active downloads</p>}
    />,
  );
  expect(screen.getByText("No active downloads")).toBeTruthy();
  expect(screen.queryByRole("table")).toBeNull();
});

it("renders inline playback controls separately from the file properties menu", async () => {
  let played: number | null = null;
  render(<DataTable data={[{ id: 1, name: "Episode", available: true }, { id: 2, name: "Missing", available: false }]}
    columns={[{ key: "name", label: "Name" }]} getRowKey={row => row.id}
    rowActions={[
      { key: "play", label: "Play", icon: <span>Play icon</span>, inDropdown: false, isVisible: row => row.available, onAction: row => { played = row.id; } },
      { key: "properties", label: "File Properties", inDropdown: true, onAction: () => {} },
    ]} />);
  const play = screen.getByRole("button", { name: "Play" });
  expect(play.closest("tr")?.className).toContain("group");
  expect(screen.getAllByRole("button", { name: "Play" })).toHaveLength(1);
  fireEvent.click(play);
  await waitFor(() => expect(played).toBe(1));
  expect(screen.getAllByRole("button", { name: "Row actions" })).toHaveLength(2);
});
