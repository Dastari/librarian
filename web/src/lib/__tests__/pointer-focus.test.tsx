import { fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { focusTargetOf, installPointerFocus } from "../pointer-focus";

function Cards() {
  return (
    <div>
      <div className="poster-card" data-testid="a"><a className="poster-navigation" href="/a">A</a></div>
      <div className="poster-card" data-testid="b"><a className="poster-navigation" href="/b">B</a></div>
    </div>
  );
}

describe("pointer focus", () => {
  beforeEach(() => installPointerFocus());
  afterEach(() => { document.documentElement.dataset.inputMode = ""; });

  it("records whichever input was used last", () => {
    render(<Cards />);
    fireEvent.keyDown(window, { key: "ArrowRight" });
    expect(document.documentElement.dataset.lastInput).toBe("keys");
    fireEvent.pointerMove(window, { pointerType: "mouse", movementX: 3, movementY: 0 });
    expect(document.documentElement.dataset.lastInput).toBe("pointer");
    fireEvent.keyDown(window, { key: "Shift" });
    expect(document.documentElement.dataset.lastInput).toBe("pointer");
  });

  it("in TV mode, hovering a card focuses it, but not after a key press until the mouse moves", () => {
    document.documentElement.dataset.inputMode = "tv";
    const { getByTestId, getByText } = render(<Cards />);
    getByText("A").focus();

    fireEvent.keyDown(window, { key: "ArrowRight" });
    fireEvent.pointerOver(getByTestId("b"), { pointerType: "mouse" });
    expect(document.activeElement).toBe(getByText("A"));

    fireEvent.pointerMove(getByTestId("b"), { pointerType: "mouse", movementX: 4, movementY: 1 });
    expect(document.activeElement).toBe(getByText("B"));
  });

  it("leaves focus alone outside TV mode", () => {
    document.documentElement.dataset.inputMode = "pointer";
    const { getByTestId, getByText } = render(<Cards />);
    getByText("A").focus();
    fireEvent.pointerMove(getByTestId("b"), { pointerType: "mouse", movementX: 4, movementY: 1 });
    expect(document.activeElement).toBe(getByText("A"));
  });

  it("finds a card's own focus target", () => {
    const { getByTestId, getByText } = render(<Cards />);
    expect(focusTargetOf(getByTestId("a"))).toBe(getByText("A"));
  });
});
