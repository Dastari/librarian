import { fireEvent, render } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";

import { useTypeToSearch } from "../useTypeToSearch";

function Harness() {
  const [query, setQuery] = useState("");
  useTypeToSearch((update) => setQuery(update));
  return (
    <div>
      <output data-testid="query">{query}</output>
      <button type="button">Result</button>
      <input aria-label="Other field" />
      <div role="dialog"><button type="button">In dialog</button></div>
    </div>
  );
}

describe("useTypeToSearch", () => {
  it("types into the search from a focused button, with space and backspace", () => {
    const { getByTestId, getByText } = render(<Harness />);
    const button = getByText("Result");
    for (const key of ["h", "a", " ", "x", "Backspace", "P"]) fireEvent.keyDown(button, { key });
    expect(getByTestId("query").textContent).toBe("ha P");
  });

  it("leaves shortcuts, fields and dialogs alone", () => {
    const { getByTestId, getByText, getByLabelText } = render(<Harness />);
    fireEvent.keyDown(getByText("Result"), { key: "f", ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(getByText("Result"), { key: "k", metaKey: true });
    fireEvent.keyDown(getByLabelText("Other field"), { key: "a" });
    fireEvent.keyDown(getByText("In dialog"), { key: "b" });
    fireEvent.keyDown(getByText("Result"), { key: "Enter" });
    expect(getByTestId("query").textContent).toBe("");
  });
});
