import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { defineActions } from "glass-ux/actions";
import { createDataTableColumns, DataTable } from "glass-ux/data-table";
import { Dialog } from "glass-ux/dialog";
import { Menu } from "glass-ux/menu";
import { Tooltip, TooltipProvider } from "glass-ux/tooltip";
import { useState, type ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { layoutColumn, pressBack, remoteStops, walkRemote } from "@/test/remote";

import { InputModeProvider, useRouteFocus, useSpatialNavigation } from "../input-mode";
import { backHandled } from "../remote-back";

// The Glass table draws rows only once it has measured a viewport.
beforeEach(() => {
  window.localStorage.setItem("librarian.inputMode", "tv");
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({ width: 1200, height: 700, top: 0, left: 0, bottom: 700, right: 1200, x: 0, y: 0, toJSON: () => ({}) });
  vi.spyOn(HTMLElement.prototype, "offsetWidth", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 32 : 1200; });
  vi.spyOn(HTMLElement.prototype, "offsetHeight", "get").mockImplementation(function (this: HTMLElement) { return this.classList.contains("fa-dt-probe") ? 48 : 700; });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(1200);
  vi.spyOn(HTMLElement.prototype, "clientHeight", "get").mockReturnValue(700);
});
afterEach(() => {
  vi.restoreAllMocks();
  document.documentElement.removeAttribute("data-input-mode");
});

function Remote({ children, onBack = () => true }: { children: ReactNode; onBack?: () => boolean | void }) {
  useSpatialNavigation(true, true, onBack);
  return <main id="main">{children}</main>;
}

const tv = (ui: ReactNode, onBack?: () => boolean | void) => render(
  <InputModeProvider>
    <TooltipProvider>
      <Remote onBack={onBack}>{ui}</Remote>
    </TooltipProvider>
  </InputModeProvider>,
);

interface Row { id: string; name: string; size: number }
const rows: Row[] = [{ id: "1", name: "Alpha", size: 3 }, { id: "2", name: "Beta", size: 1 }, { id: "3", name: "Gamma", size: 2 }];
const column = createDataTableColumns<Row>();
const columns = column.columns([
  column.accessor("name", { header: "Name", width: 20, priority: 1, cardRole: "title", sortable: true }),
  column.accessor("size", { header: "Size", width: 8, priority: 2, sortable: true }),
]);
const actions = defineActions<Row>("row", [{ id: "open", label: "Open", run: () => undefined }]);

function Table() {
  return (
    <>
      <button type="button">Above</button>
      <DataTable<Row> id="remote-test" label="Releases" view="table" height="auto" columns={columns} rows={rows} getRowId={(row) => row.id} getRowLabel={(row) => row.name} selectable actions={actions} />
      <button type="button">Below</button>
    </>
  );
}

describe("what a remote stops on", () => {
  it("never stops on a data table's header, resize handles or row controls; each row is one stop", () => {
    tv(<Table />);
    layoutColumn(document.body);
    const stops = remoteStops(document.body);
    // The header is there for the pointer...
    const grid = screen.getByRole("grid", { name: "Releases" });
    expect(within(grid).getAllByRole("columnheader").length).toBeGreaterThan(0);
    // ...but none of it, nor a row's checkbox or ⋯, is a remote stop.
    for (const name of ["Name", "Size", "Select all", "Select page", "Resize Name"]) expect(stops).not.toContain(name);
    expect(stops.filter((stop) => stop.startsWith("row:"))).toEqual(["row:1", "row:2", "row:3"]);
    expect(stops.some((stop) => /^(Select|Actions for)/.test(stop))).toBe(false);
  });

  it("puts sorting in the table's toolbar on a TV instead", () => {
    tv(<Table />);
    const toolbar = document.querySelector(".fa-dt-toolbar");
    expect(toolbar).not.toBeNull();
    expect(within(toolbar as HTMLElement).getByRole("button", { name: /sort/i })).toBeInTheDocument();
  });

  it("walks down from above the table, through its rows, and out past it", () => {
    tv(<Table />);
    layoutColumn(document.body);
    const walk = walkRemote(screen.getByRole("button", { name: "Above" }));
    expect(walk[0]).toBe("Above");
    expect(walk[1]).toMatch(/sort/i);
    expect(walk.slice(2, 5)).toEqual(["row:1", "row:2", "row:3"]);
    expect(walk).toContain("Below");
    expect(walk.some((stop) => ["Name", "Size"].includes(stop))).toBe(false);
  });

  it("leaves the table upwards from its first row instead of sticking there", () => {
    tv(<Table />);
    layoutColumn(document.body);
    const first = document.querySelector<HTMLElement>('.fa-dt-grid [data-row-id="1"]')!;
    const walk = walkRemote(first, "ArrowUp");
    expect(walk[0]).toBe("row:1");
    expect(walk[1]).toMatch(/sort/i);
    expect(walk.at(-1)).toBe("Above");
  });

  it("skips disabled, hidden and collapsed controls", () => {
    tv(
      <>
        <button type="button">First</button>
        <button type="button" disabled>Disabled</button>
        <button type="button" aria-disabled="true">Explained</button>
        <div hidden><button type="button">Hidden</button></div>
        <button type="button" data-nav-skip tabIndex={-1}>Pointer only</button>
        <button type="button">Last</button>
      </>,
    );
    layoutColumn(document.body);
    expect(walkRemote(screen.getByRole("button", { name: "First" }))).toEqual(["First", "Last"]);
  });
});

function Layers({ onBack }: { onBack: () => boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <Remote onBack={onBack}>
      <button type="button" onClick={() => setOpen(true)}>Open settings</button>
      <Dialog.Root open={open} onOpenChange={setOpen}>
        <Dialog.Popup>
          <Dialog.Header title="Settings" close={false} />
          <Dialog.Body>
            <Menu.Root>
              <Menu.Trigger render={<button type="button">More</button>} />
              <Menu.Popup>
                <Menu.Item>Rename</Menu.Item>
              </Menu.Popup>
            </Menu.Root>
          </Dialog.Body>
        </Dialog.Popup>
      </Dialog.Root>
    </Remote>
  );
}

describe("the remote's Back", () => {
  it("closes the menu, then the dialog (focus back on its opener), then goes back a page", async () => {
    const user = userEvent.setup();
    const onBack = vi.fn(() => true);
    render(<InputModeProvider><TooltipProvider><Layers onBack={onBack} /></TooltipProvider></InputModeProvider>);
    const opener = screen.getByRole("button", { name: "Open settings" });
    await user.click(opener);
    await user.click(await screen.findByRole("button", { name: "More" }));
    expect(await screen.findByRole("menu")).toBeInTheDocument();

    pressBack();
    await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(onBack).not.toHaveBeenCalled();

    pressBack();
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(document.activeElement).toBe(opener));
    expect(onBack).not.toHaveBeenCalled();

    pressBack();
    expect(onBack).toHaveBeenCalledOnce();
  });

  it("is not swallowed by a focused icon button's tooltip", async () => {
    const onBack = vi.fn(() => true);
    tv(
      <Tooltip content="Close now playing">
        <button type="button" aria-label="Close now playing">×</button>
      </Tooltip>,
      onBack,
    );
    const button = screen.getByRole("button", { name: "Close now playing" });
    act(() => button.focus());
    fireEvent.focus(button);
    await waitFor(() => expect(screen.getByRole("tooltip")).toBeInTheDocument(), { timeout: 2000 });

    expect(pressBack()).toBe(true);
    expect(onBack).toHaveBeenCalledOnce();
  });

  it("tells a tooltip closing on Escape from a control that took Back", () => {
    const seen: boolean[] = [];
    const onDocument = (event: KeyboardEvent) => event.preventDefault();
    document.addEventListener("keydown", onDocument, true);
    const probe = (event: KeyboardEvent) => seen.push(backHandled(event));
    window.addEventListener("keydown", probe);
    render(<button type="button" onKeyDown={(event) => { if (event.key === "Backspace") event.preventDefault(); }}>Control</button>);
    const button = screen.getByRole("button", { name: "Control" });
    fireEvent.keyDown(button, { key: "Escape" });
    fireEvent.keyDown(button, { key: "Backspace" });
    document.removeEventListener("keydown", onDocument, true);
    window.removeEventListener("keydown", probe);
    // Prevented by a document-level dismiss layer only: not taken. By the control: taken.
    expect(seen).toEqual([false, true]);
  });

  it("clears a search field's text before going back, and leaves Home to the TV app", () => {
    const onBack = vi.fn(() => false);
    tv(<input type="search" aria-label="Search" defaultValue="dune" />, onBack);
    const field = screen.getByRole("searchbox", { name: "Search" }) as HTMLInputElement;
    field.focus();
    expect(pressBack("GoBack")).toBe(true);
    expect(field.value).toBe("");
    expect(onBack).not.toHaveBeenCalled();
    // Nothing left to leave: Home's back step declines, so the key stays unhandled.
    expect(pressBack("GoBack")).toBe(false);
    expect(onBack).toHaveBeenCalledOnce();
  });

  it("aims Back at the now playing drawer when focus has left it for the page", () => {
    const onClose = vi.fn();
    tv(
      <>
        <button type="button">Page control</button>
        <section role="dialog" aria-modal="false" aria-label="Now playing" onKeyDown={(event) => { if (event.key === "Escape") { event.stopPropagation(); onClose(); } }}>
          <button type="button">Pause</button>
        </section>
      </>,
    );
    screen.getByRole("button", { name: "Page control" }).focus();
    pressBack("GoBack");
    expect(onClose).toHaveBeenCalledOnce();
  });
});

function MenuOpensDialog() {
  const [open, setOpen] = useState(false);
  return (
    <Remote>
      <button type="button">Elsewhere</button>
      <Menu.Root>
        <Menu.Trigger render={<button type="button" aria-label="Actions for Dune">⋯</button>} />
        <Menu.Popup>
          <Menu.Item onClick={() => setOpen(true)}>Settings…</Menu.Item>
        </Menu.Popup>
      </Menu.Root>
      <Dialog.Root open={open} onOpenChange={setOpen}>
        <Dialog.Popup>
          <Dialog.Header title="Dune settings" />
          <Dialog.Body><button type="button">Save</button></Dialog.Body>
        </Dialog.Popup>
      </Dialog.Root>
    </Remote>
  );
}

describe("focus after an overlay closes", () => {
  it("returns to the control that opened a dialog from its menu, never the body", async () => {
    const user = userEvent.setup();
    render(<InputModeProvider><TooltipProvider><MenuOpensDialog /></TooltipProvider></InputModeProvider>);
    const trigger = screen.getByRole("button", { name: "Actions for Dune" });
    await user.click(trigger);
    await user.click(await screen.findByRole("menuitem", { name: "Settings…" }));
    expect(await screen.findByRole("dialog", { name: "Dune settings" })).toBeInTheDocument();
    pressBack();
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(document.activeElement).toBe(trigger));
  });

  it("falls back to the page's first stop when the opener is gone", async () => {
    function Gone() {
      const [open, setOpen] = useState(false);
      const [opener, setOpener] = useState(true);
      return (
        <Remote>
          <button type="button">Start here</button>
          {opener ? <button type="button" onClick={() => { setOpen(true); }}>Delete</button> : null}
          <Dialog.Root open={open} onOpenChange={setOpen}>
            <Dialog.Popup>
              <Dialog.Header title="Delete?" close={false} />
              <Dialog.Body><button type="button" onClick={() => { setOpener(false); setOpen(false); }}>Confirm</button></Dialog.Body>
            </Dialog.Popup>
          </Dialog.Root>
        </Remote>
      );
    }
    const user = userEvent.setup();
    render(<InputModeProvider><TooltipProvider><Gone /></TooltipProvider></InputModeProvider>);
    await user.click(screen.getByRole("button", { name: "Delete" }));
    await user.click(await screen.findByRole("button", { name: "Confirm" }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Start here" })));
  });
});

function Pages({ path }: { path: string }) {
  useRouteFocus(path, true);
  return (
    <main id="main">
      {path === "/library" ? (
        ["Alien", "Blade Runner", "Dune"].map((title) => <a key={title} href={`/movies/${title}`}>{title}</a>)
      ) : (
        <button type="button" data-tv-autofocus>Play</button>
      )}
    </main>
  );
}

describe("focus after navigating back", () => {
  it("returns to the card the person opened", async () => {
    const { rerender } = render(<Pages path="/library" />);
    await waitFor(() => expect(document.activeElement?.textContent).toBe("Alien"));
    act(() => screen.getByRole("link", { name: "Dune" }).focus());
    rerender(<Pages path="/movies/Dune" />);
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Play" })));
    rerender(<Pages path="/library" />);
    await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("link", { name: "Dune" })));
  });
});
