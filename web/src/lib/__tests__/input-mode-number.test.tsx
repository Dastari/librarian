import { fireEvent, render, screen } from "@testing-library/react";
import { Dialog } from "glass-ux/dialog";
import { NumberField } from "glass-ux/number-field";
import { afterEach, describe, expect, it } from "vitest";

import { InputModeProvider, useSpatialNavigation } from "../input-mode";

function Fixture({ tv, dialog }: { tv: boolean; dialog?: boolean }) {
  useSpatialNavigation(true, tv);
  const rows = (
    <>
      <button type="button" data-testid="above">Above</button>
      <NumberField aria-label="Limit" defaultValue={5} />
      <button type="button" data-testid="beneath">Beneath</button>
    </>
  );
  return (
    <main>
      {dialog ? (
        <Dialog.Root defaultOpen>
          <Dialog.Popup>
            <Dialog.Header title="Limits" close={false} />
            <Dialog.Body>{rows}</Dialog.Body>
          </Dialog.Popup>
        </Dialog.Root>
      ) : rows}
    </main>
  );
}

function place(testId: string, y: number) {
  const element = screen.getByTestId(testId);
  element.getBoundingClientRect = () => ({ x: 0, y, left: 0, top: y, right: 40, bottom: y + 20, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
}

function field() {
  const input = screen.getByRole("textbox", { name: "Limit" }) as HTMLInputElement;
  input.getBoundingClientRect = () => ({ x: 0, y: 100, left: 0, top: 100, right: 40, bottom: 120, width: 40, height: 20, toJSON: () => ({}) }) as DOMRect;
  return input;
}

afterEach(() => window.localStorage.clear());

describe.each([{ dialog: false }, { dialog: true }])("a Glass number field in TV mode (dialog: $dialog)", ({ dialog }) => {
  const mount = async (tv: boolean) => {
    if (tv) window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><Fixture tv={tv} dialog={dialog} /></InputModeProvider>);
    if (dialog) await screen.findByRole("dialog");
    place("above", 0);
    place("beneath", 200);
    return field();
  };

  it("lets Up and Down move focus and leaves the value alone", async () => {
    const input = await mount(true);
    input.focus();
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(screen.getByTestId("beneath")).toHaveFocus();
    expect(input.value).toBe("5");
    input.focus();
    fireEvent.keyDown(input, { key: "ArrowUp" });
    expect(screen.getByTestId("above")).toHaveFocus();
    expect(input.value).toBe("5");
  });

  it("keeps stepping with the arrows outside TV mode", async () => {
    const input = await mount(false);
    input.focus();
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(input).toHaveFocus();
    expect(input.value).toBe("4");
  });
});

describe("adjusting a number field with OK in TV mode", () => {
  async function mount() {
    window.localStorage.setItem("librarian.inputMode", "tv");
    render(<InputModeProvider><Fixture tv /></InputModeProvider>);
    place("above", 0);
    place("beneath", 200);
    return field();
  }

  it("steps with Up and Down only between two presses of OK", async () => {
    const input = await mount();
    input.focus();
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input).toHaveAttribute("data-tv-adjusting");
    fireEvent.keyDown(input, { key: "ArrowUp" });
    expect(input).toHaveFocus();
    expect(input.value).toBe("6");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(input.value).toBe("4");
    fireEvent.keyDown(input, { key: "Enter" });
    expect(input).not.toHaveAttribute("data-tv-adjusting");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    expect(screen.getByTestId("beneath")).toHaveFocus();
    expect(input.value).toBe("4");
  });

  it("ends adjusting on Back or when focus leaves, without going back", async () => {
    const input = await mount();
    input.focus();
    fireEvent.keyDown(input, { key: "Enter" });
    const escape = new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true });
    fireEvent(input, escape);
    expect(escape.defaultPrevented).toBe(true);
    expect(input).not.toHaveAttribute("data-tv-adjusting");
    fireEvent.keyDown(input, { key: "Enter" });
    screen.getByTestId("above").focus();
    expect(input).not.toHaveAttribute("data-tv-adjusting");
  });
});
