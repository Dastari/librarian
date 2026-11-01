import { describe, expect, it, vi } from "vitest";

import { runSettingsSelection } from "../runSettingsSelection";

describe("settings selection actions", () => {
  it("continues after a failed target, refreshes once and reports partial success", async () => {
    const refresh = vi.fn();
    const run = vi.fn().mockRejectedValueOnce(new Error("Denied")).mockResolvedValueOnce(undefined);
    await expect(runSettingsSelection(["first", "second"], "Remove", (row) => row, run, refresh)).rejects.toThrow("Remove succeeded for 1 of 2. 1 failed. first: Denied");
    expect(run).toHaveBeenNthCalledWith(2, "second");
    expect(refresh).toHaveBeenCalledOnce();
  });

  it("refreshes successful batches without adding a second feedback path", async () => {
    const refresh = vi.fn();
    await expect(runSettingsSelection(["first", "second"], "Update", (row) => row, vi.fn().mockResolvedValue(undefined), refresh)).resolves.toBeUndefined();
    expect(refresh).toHaveBeenCalledOnce();
  });

  it("distinguishes completed writes from a failed refresh without an unhandled rejection", async () => {
    await expect(runSettingsSelection(["first"], "Update", (row) => row, vi.fn().mockResolvedValue(undefined), vi.fn().mockRejectedValue(new Error("Offline")))).rejects.toThrow("Update completed, but refreshing the results failed: Offline");
  });
});
