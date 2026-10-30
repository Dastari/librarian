import { act, render, screen } from "@testing-library/react";
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

  it("closes through the library header close control", async () => {
    const onOpenChange = vi.fn();
    render(<Dialog isOpen onOpenChange={onOpenChange} title="Downloads">Body</Dialog>);
    await userEvent.click(screen.getByRole("button", { name: /close/i }));
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
    await vi.waitFor(() => expect(screen.getByRole("button", { name: "Open" })).toHaveFocus());
  });
  it("blocks Escape when dismissal is disabled", async () => {
    const onOpenChange = vi.fn();
    render(<Dialog isOpen onOpenChange={onOpenChange} title="Saving" isDismissable={false}>Body</Dialog>);
    await userEvent.keyboard("{Escape}");
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: /close/i })).toBeNull();
  });
});

describe("ConfirmDialog", () => {
  it("asks once and calls back on confirm", async () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    render(<ConfirmDialog isOpen onOpenChange={onOpenChange} title="Delete Standard?" description="Libraries fall back to the default." confirmLabel="Delete" destructive onConfirm={onConfirm} />);
    expect(screen.getByRole("dialog")).toHaveTextContent("Delete Standard?");
    expect(screen.getByText("Libraries fall back to the default.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /^Delete/ }));
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

  it("locks both buttons and ignores dismissal while the action is in flight", async () => {
    const onOpenChange = vi.fn();
    render(<ConfirmDialog isOpen onOpenChange={onOpenChange} title="Delete Standard?" confirmLabel="Delete" isPending onConfirm={() => {}} />);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    const confirm = screen.getByRole("button", { name: /Delete/ });
    expect(confirm).toHaveAttribute("aria-disabled", "true");
    expect(confirm).toHaveAttribute("aria-busy", "true");
    await userEvent.keyboard("{Escape}");
    await userEvent.click(screen.getByRole("dialog").parentElement!);
    expect(onOpenChange).not.toHaveBeenCalled();
  });

  it("focuses Cancel first and restores the host opener", async () => {
    function Host() {
      const [open, setOpen] = useState(false);
      return <><Button onPress={() => setOpen(true)}>Remove library</Button><ConfirmDialog isOpen={open} onOpenChange={setOpen} title="Remove library?" onConfirm={() => {}} /></>;
    }
    render(<Host />);
    const opener = screen.getByRole("button", { name: "Remove library" });
    await userEvent.click(opener);
    await vi.waitFor(() => expect(screen.getByRole("button", { name: "Cancel" })).toHaveFocus());
    await userEvent.click(screen.getByRole("button", { name: "Cancel" }));
    await vi.waitFor(() => expect(opener).toHaveFocus());
  });

  it("also locks dismissal while awaiting onConfirm without an external pending prop", async () => {
    let finish!: () => void;
    const onConfirm = vi.fn(() => new Promise<void>((resolve) => { finish = resolve; }));
    const onOpenChange = vi.fn();
    render(<ConfirmDialog isOpen onOpenChange={onOpenChange} title="Remove library?" onConfirm={onConfirm} />);
    await userEvent.click(screen.getByRole("button", { name: /^Confirm/ }));
    await userEvent.keyboard("{Escape}");
    await userEvent.click(screen.getByRole("button", { name: /Confirm/ }));
    expect(onConfirm).toHaveBeenCalledOnce();
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    await act(async () => finish());
    expect(screen.getByRole("button", { name: "Cancel" })).not.toBeDisabled();
  });

});
