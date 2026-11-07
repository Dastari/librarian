import { render, screen } from "@testing-library/react";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import { describe, expect, it, vi } from "vitest";

import { LibraryIdentity, libraryState } from "../LibraryIdentity";

const library = { id: "l1", name: "Audio Books", libraryType: "AUDIOBOOKS", scanning: false, lastScannedAt: "1794000000" };

describe("a library's identity", () => {
  it("puts scanning before everything else, then issues", () => {
    expect(libraryState({ ...library, scanning: true }, 3)).toMatchObject({ id: "scanning", activity: true, tone: "info" });
    expect(libraryState(library, 3)).toMatchObject({ id: "issues", label: "3 issues", tone: "warning" });
    expect(libraryState(library)).toMatchObject({ id: "ready", tone: "success" });
    expect(libraryState({ ...library, lastScannedAt: null })).toMatchObject({ label: "Not scanned yet", tone: "neutral" });
  });

  it("shows what it holds and its state", () => {
    render(<TooltipProvider><ActionProvider navigate={vi.fn()}><LibraryIdentity tier="header" heading="h2" library={library} items={12} issues={2} /></ActionProvider></TooltipProvider>);
    expect(screen.getByRole("heading", { name: "Audio Books" })).toBeTruthy();
    expect(screen.getByText("2 issues")).toBeTruthy();
    expect(screen.getByText(/12 audiobooks/)).toBeTruthy();
  });
});
