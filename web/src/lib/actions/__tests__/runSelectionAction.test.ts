import { describe, expect, it, vi } from "vitest";
import { runSelectionAction } from "../runSelectionAction";

describe("selection action execution", () => {
  it("continues independent targets and names partial failures after refreshing once", async () => {
    const run = vi.fn().mockRejectedValueOnce(new Error("No access")).mockResolvedValueOnce(undefined);
    const refresh = vi.fn();
    await expect(runSelectionAction(["A", "B"], "Match", row => row, run, refresh)).rejects.toThrow("Match succeeded for 1 of 2 selected items. 1 failed. A: No access");
    expect(run.mock.calls).toEqual([["A"], ["B"]]);
    expect(refresh).toHaveBeenCalledOnce();
  });
  it("does not operate on an unselected target", async () => {
    const run = vi.fn();
    await runSelectionAction(["chosen"], "Update", row => row, run);
    expect(run).toHaveBeenCalledExactlyOnceWith("chosen");
  });
});
