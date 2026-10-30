import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, expect, it, vi } from "vitest";

import type { ResumeShelfQuery } from "@/graphql/generated/graphql";
import { playerStore } from "@/features/player/store";
import { renderWithProviders } from "@/test";

import { HomePage } from "../HomePage";
import { resumeItems } from "../useContinueWatching";

const navigate = vi.hoisted(() => vi.fn());
vi.mock("@tanstack/react-router", async (importOriginal) => ({
  ...await importOriginal<typeof import("@tanstack/react-router")>(), useNavigate: () => navigate,
}));
vi.mock("@/lib/graphql/live/hooks", () => ({ useLiveQuery: () => ({ data: undefined, loading: false }) }));
vi.mock("@/features/libraries/browser/useCompleteConnection", () => ({
  useCompleteConnection: () => ({ rows: [{ id: "library" }], loading: false }),
}));
vi.mock("@/features/personal/usePersonalLibrary", () => ({
  useHomeShelves: () => ({ shelves: [{ key: "continue", visible: true }] }),
  useLibraryPreferences: () => ({ rows: [] }),
}));
vi.mock("../useContinueWatching", async (importOriginal) => ({
  ...await importOriginal<typeof import("../useContinueWatching")>(),
  useContinueWatching: () => ({ items: resumeItems(rows), loading: false }),
}));

const rows: ResumeShelfQuery["resumeShelf"]["items"] = [{
  id: "checkpoint", userId: "me", mediaFileId: "episode-file", currentPosition: 123,
  duration: 600, progressPercent: 20.5, isWatched: false, watchedAt: null, createdAt: "", updatedAt: "",
  mediaFile: {
    id: "episode-file", duration: 600, movie: null, track: null, chapter: null,
    episode: { id: "episode-id", showId: "show-id", season: 1, episode: 2, title: "The second episode",
      show: { id: "show-id", name: "A series", posterUrl: null, backdropUrl: null } },
  },
}];
afterEach(() => { act(() => playerStore.setVideo(null)); });

it("resumes the episode entity and preserves its show link for next/back navigation", async () => {
  await renderWithProviders(<HomePage />);
  await userEvent.click(screen.getByRole("button", { name: /A series/ }));
  expect(playerStore.getSnapshot().video).toMatchObject({
    mediaFileId: "episode-file", entity: { kind: "episode", id: "episode-id" },
    href: "/shows/show-id", startPosition: 123,
  });
  expect(navigate).toHaveBeenCalledWith({ to: "/watch/$mediaFileId", params: { mediaFileId: "episode-file" } });
});
