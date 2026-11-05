import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { PageScroller } from "../PageScroller";

describe("PageScroller", () => {
  it("is the page's scroller, marked for the remote's reveal", () => {
    render(<PageScroller data-testid="pane">Content</PageScroller>);
    const pane = screen.getByTestId("pane");
    expect(pane).toHaveAttribute("data-page-scroller");
    expect(pane).toHaveClass("overflow-y-auto");
  });

  it("clips a bounded pane, so focus or a reveal cannot slide the page away", () => {
    render(<PageScroller data-testid="pane" scrolling={false}>Content</PageScroller>);
    const pane = screen.getByTestId("pane");
    expect(pane).not.toHaveAttribute("data-page-scroller");
    expect(pane).toHaveClass("overflow-y-clip");
    expect(pane).not.toHaveClass("overflow-y-auto");
  });
});
