import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionBar, resolveActions, type ActionSet } from "glass-ux/actions";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { EntityPlaybackProgressDeleteManyDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import type { MovieEntity } from "../MovieIdentity";
import type { ShowEntity } from "../ShowIdentity";
import type { ProgressReset } from "../progressReset";
import { useMovieActions } from "../useMovieActions";
import { useShowActions } from "../useShowActions";

let admin = false;
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => admin, useSession: () => ({ user: { id: "viewer", role: admin ? "admin" : "member" } }) }));
vi.mock("@/features/player/usePlayer", () => ({ usePlayer: () => ({ playVideo: vi.fn() }) }));
vi.mock("@/features/downloads/ReleaseSearchDialog", () => ({ ReleaseSearchDialog: () => null }));
vi.mock("@/features/personal/SaveToListDialog", () => ({ SaveToListDialog: () => null }));
beforeEach(() => { admin = false; });

const movie = { id: "movie-1", userId: "viewer", title: "Dune", year: 2021, mediaFileId: "movie-file", hasFile: true } as unknown as MovieEntity;
const show = { id: "show-1", userId: "viewer", name: "Andor" } as unknown as ShowEntity;

function MovieBar({ reset }: { reset?: ProgressReset<MovieEntity> }) {
  const { actions } = useMovieActions<MovieEntity>({ settings: vi.fn(), reset });
  return <ActionBar set={actions} targets={[movie]} label={movie.title} />;
}
function ShowBar({ reset }: { reset?: ProgressReset<ShowEntity> }) {
  const { actions } = useShowActions<ShowEntity>({ settings: vi.fn(), reset });
  return <ActionBar set={actions} targets={[show]} label={show.name} />;
}
const deleteMock = (mediaFileIds: string[]) => {
  const called = vi.fn(() => ({ data: { deletePlaybackProgresses: { __typename: "DeletePlaybackProgressesResult", success: true, error: null } } }));
  return { called, mock: { request: { query: EntityPlaybackProgressDeleteManyDocument, variables: { where: { userId: { eq: "viewer" }, mediaFileId: { inList: mediaFileIds } } } }, result: called } };
};
const openMore = async () => userEvent.click(await screen.findByRole("button", { name: /^More actions/ }));

describe("Mark as unwatched on a movie", () => {
  it("resets the movie's file for a member", async () => {
    const { called, mock } = deleteMock(["movie-file"]);
    await renderWithProviders(<MovieBar reset={{ files: () => ["movie-file"] }} />, { mocks: [mock] });
    await openMore();
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unwatched" }));
    await waitFor(() => expect(called).toHaveBeenCalledOnce());
  });

  it("is not offered when there is no progress, or where the page never read it", async () => {
    let actions!: ActionSet<MovieEntity>;
    const Probe = ({ reset }: { reset?: ProgressReset<MovieEntity> }) => { actions = useMovieActions<MovieEntity>({ settings: vi.fn(), reset }).actions; return null; };
    const labels = () => resolveActions(actions, [movie], "menu").flatMap((group) => group.actions.map((action) => action.label));
    const view = await renderWithProviders(<Probe reset={{ files: () => ["movie-file"] }} />);
    expect(labels()).toContain("Mark as unwatched");
    view.unmount();
    const none = await renderWithProviders(<Probe reset={{ files: () => [] }} />);
    expect(labels()).not.toContain("Mark as unwatched");
    expect(labels()).toContain("Add to playlist…");
    none.unmount();
    await renderWithProviders(<Probe />);
    expect(labels()).not.toContain("Mark as unwatched");
  });
});

describe("Mark as unwatched on a show", () => {
  it("resets every episode's file that has progress, in one request", async () => {
    const { called, mock } = deleteMock(["ep-1", "ep-2", "ep-9"]);
    await renderWithProviders(<ShowBar reset={{ files: () => ["ep-1", "ep-2", "ep-9"] }} />, { mocks: [mock] });
    await openMore();
    await userEvent.click(await screen.findByRole("menuitem", { name: "Mark as unwatched" }));
    await waitFor(() => expect(called).toHaveBeenCalledOnce());
  });

  it("shows why it waits while episodes or progress are still loading", async () => {
    await renderWithProviders(<ShowBar reset={{ files: () => ["ep-1"], disabledReason: () => "Loading the complete episode list" }} />);
    await openMore();
    expect(await screen.findByRole("menuitem", { name: "Mark as unwatched" })).toHaveAttribute("aria-disabled", "true");
  });
});
