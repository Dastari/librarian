import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { fileFormat, isFileLinked, useFileActions, type FileEntity } from "../useFileActions";

vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true }));
vi.mock("@/features/libraries/ManualMatchDialog", () => ({ ManualMatchDialog: () => null }));
vi.mock("@apollo/client/react", () => ({ useMutation: () => [vi.fn()] }));

const file = { id: "f1", path: "/media/Film (2020).mkv", movieId: null, episodeId: null, trackId: null, chapterId: null } as unknown as FileEntity;
const ids = (actions: ReturnType<typeof useFileActions>["actions"]) => actions.actions.map((action) => action.id);

describe("file actions", () => {
  it("says plainly that deleting a file removes it from disk for good", () => {
    const { result } = renderHook(() => useFileActions());
    const remove = result.current.actions.actions.find((action) => action.id === "delete")!;
    expect(typeof remove.label === "function" ? remove.label([file]) : remove.label).toBe("Delete file");
    const confirm = typeof remove.confirm === "function" ? remove.confirm([file]) : remove.confirm;
    expect(confirm?.title).toContain("Film (2020).mkv");
    expect(confirm?.body).toContain("can't be undone");
    expect(confirm?.tone).toBe("danger");
  });

  it("offers the details action everywhere but the dialog itself", () => {
    expect(ids(renderHook(() => useFileActions()).result.current.actions)).toContain("details");
    expect(ids(renderHook(() => useFileActions(undefined, { inDialog: true })).result.current.actions)).not.toContain("details");
  });

  it("describes a file by what it is and what it belongs to", () => {
    expect(isFileLinked(file)).toBe(false);
    expect(isFileLinked({ ...file, movieId: "m1" })).toBe(true);
    expect(fileFormat({ resolution: "1920x808", videoCodec: "hevc", audioCodec: "ac3", isHdr: true, hdrType: "HDR10" })).toBe("1920x808 · HEVC · AC3 · HDR10");
  });
});
