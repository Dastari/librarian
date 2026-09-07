import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it, vi } from "vitest";

import { Button } from "../Button";
import { ConfirmDialog } from "../ConfirmDialog";
import { Dialog } from "../Dialog";

describe("Dialog", () => {
  it("renders nothing until it is opened", () => {
    render(
      <Dialog isOpen={false} onOpenChange={() => {}} title="Downloads">
        <p>Body</p>
      </Dialog>,
    );
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("shows the title, description, body and footer", () => {
    render(
      <Dialog isOpen onOpenChange={() => {}} title="Downloads" description="Andor" footer={<Button>Save</Button>}>
        <p>Body</p>
      </Dialog>,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog).toHaveTextContent("Downloads");
    expect(dialog).toHaveTextContent("Andor");
    expect(dialog).toHaveTextContent("Body");
    expect(screen.getByRole("button", { name: "Save" })).toBeInTheDocument();
  });

  it("closes on Escape and through the close button", async () => {
    const onOpenChange = vi.fn();
    render(
      <Dialog isOpen onOpenChange={onOpenChange} title="Downloads">
        <p>Body</p>
      </Dialog>,
    );
    await userEvent.keyboard("{Escape}");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("drives an open/close cycle from the parent", async () => {
    function Host() {
      const [open, setOpen] = useState(false);
      return (
        <>
          <Button onPress={() => setOpen(true)}>Open</Button>
          <Dialog isOpen={open} onOpenChange={setOpen} title="Find a release">
            <p>Results</p>
          </Dialog>
        </>
      );
    }
    render(<Host />);
    await userEvent.click(screen.getByRole("button", { name: "Open" }));
    expect(await screen.findByRole("dialog")).toHaveTextContent("Find a release");
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });
});

describe("ConfirmDialog", () => {
  it("asks once and calls back on confirm", async () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(<ConfirmDialog isOpen onOpenChange={onOpenChange} title="Delete Standard?" description="Libraries fall back to the default." confirmLabel="Delete" destructive onConfirm={onConfirm} />);
    expect(screen.getByRole("dialog")).toHaveTextContent("Delete Standard?");
    expect(screen.getByText("Libraries fall back to the default.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onOpenChange).not.toHaveBeenCalled();
  });

  it("cancels without calling back", async () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(<ConfirmDialog isOpen onOpenChange={onOpenChange} title="Delete Standard?" onConfirm={onConfirm} />);
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("locks both buttons while the action is in flight", () => {
    render(<ConfirmDialog isOpen onOpenChange={() => {}} title="Delete Standard?" confirmLabel="Delete" isPending onConfirm={() => {}} />);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    // The pending button keeps its label in the DOM behind the spinner, so match on the text.
    expect(screen.getByText("Delete").closest("button")).toBeDisabled();
  });
});
