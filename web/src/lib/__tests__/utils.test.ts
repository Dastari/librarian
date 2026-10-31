import { describe, expect, it } from "vitest";

import { cn } from "../utils";

describe("cn", () => {
  it("keeps a type-scale size next to a text colour", () => {
    expect(cn("text-label-sm text-muted")).toBe("text-label-sm text-muted");
    expect(cn("truncate text-display-xl", "text-foreground")).toBe("truncate text-display-xl text-foreground");
    expect(cn("text-caption text-white/75")).toBe("text-caption text-white/75");
  });

  it("lets a later size replace an earlier one", () => {
    expect(cn("text-title-sm", "text-label-sm")).toBe("text-label-sm");
    expect(cn("text-sm", "text-caption")).toBe("text-caption");
  });

  it("treats tabular figures as their own concern", () => {
    expect(cn("text-numeric text-label-sm text-muted")).toBe("text-numeric text-label-sm text-muted");
  });
});
