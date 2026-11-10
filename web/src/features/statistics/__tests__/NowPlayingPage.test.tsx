import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { EntityUserListDocument, NowPlayingDocument, StopPlaybackSessionDocument } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { NowPlayingPage } from "../NowPlayingPage";

const auth = vi.hoisted(() => ({ admin: false, userId: "me" }));
vi.mock("@/lib/auth/useSession", () => ({
  useIsAdmin: () => auth.admin,
  useSession: () => ({ status: "authenticated", user: { id: auth.userId, role: auth.admin ? "admin" : "member" } }),
}));

const recently = () => new Date(Date.now() - 10_000).toISOString();

function session(id: string, userId: string, extra: Record<string, unknown> = {}) {
  return {
    __typename: "PlaybackSession",
    id, userId, contentType: "episode", mediaFileId: "f1", episodeId: "e1", movieId: null, trackId: null, audiobookId: null, tvShowId: "sh1", albumId: null, chapterId: null,
    currentPosition: 600, duration: 2400, volume: 1, isMuted: false, isPlaying: true, startedAt: recently(), lastUpdatedAt: recently(), completedAt: null,
    createdAt: recently(), updatedAt: recently(), deviceId: "d1", state: "playing", playMethod: "transcode", quality: "1080p", bitrate: 8_200_000, endedAt: null, stopRequestedAt: null,
    device: { __typename: "Device", id: "d1", name: "Living room TV", formFactor: "tv", platform: "webOS", product: null },
    movie: null, track: null, audiobook: null,
    episode: { __typename: "Episode", id: "e1", showId: "sh1", season: 2, episode: 4, title: "Woe's Hollow", show: { __typename: "Show", id: "sh1", name: "Severance" } },
    ...extra,
  };
}

function sessionsMock(nodes: unknown[]) {
  return {
    request: { query: NowPlayingDocument, variables: () => true },
    result: { data: { playbackSessions: { __typename: "PlaybackSessionConnection", edges: nodes.map((node) => ({ __typename: "PlaybackSessionEdge", node })) } } },
  };
}

const usersMock = {
  request: { query: EntityUserListDocument, variables: () => true },
  result: { data: { users: { __typename: "UserConnection", edges: [{ __typename: "UserEdge", node: { __typename: "User", id: "amy", username: "amy", email: null, role: "member", displayName: "Amy", avatarUrl: null, isActive: true, lastLoginAt: null, createdAt: "", updatedAt: "" } }], pageInfo: { __typename: "PageInfo", hasNextPage: false, hasPreviousPage: false, totalCount: 1 } } } },
};

describe("Now playing", () => {
  beforeEach(() => {
    auth.admin = false;
  });

  it("shows each player with its title, device, method, state and progress", async () => {
    await renderWithProviders(<NowPlayingPage />, { mocks: [sessionsMock([session("s1", "me")])] });
    expect(await screen.findByText("Woe's Hollow")).toBeInTheDocument();
    expect(screen.getByText("Severance · S2 · E4")).toBeInTheDocument();
    expect(screen.getByText("You · Living room TV")).toBeInTheDocument();
    expect(screen.getByText("Transcoding · 1080p · 8.2 Mbps")).toBeInTheDocument();
    expect(screen.getByText("Playing")).toBeInTheDocument();
    expect(screen.getByText("10:00 of 40:00")).toBeInTheDocument();
  });

  it("leaves out players that went quiet or closed", async () => {
    const old = new Date(Date.now() - 10 * 60_000).toISOString();
    await renderWithProviders(<NowPlayingPage />, { mocks: [sessionsMock([session("s1", "me", { lastUpdatedAt: old }), session("s2", "me", { endedAt: recently() })])] });
    expect(await screen.findByText("Nothing is playing")).toBeInTheDocument();
  });

  it("lets an administrator see who is playing and stop it", async () => {
    auth.admin = true;
    const stop = vi.fn(() => ({ data: { updatePlaybackSession: { __typename: "PlaybackSessionResult", success: true, error: null } } }));
    await renderWithProviders(<NowPlayingPage />, {
      mocks: [sessionsMock([session("s9", "amy", { state: "paused", isPlaying: false })]), usersMock, { request: { query: StopPlaybackSessionDocument, variables: () => true }, result: stop }],
    });
    expect(await screen.findByText("Amy · Living room TV")).toBeInTheDocument();
    expect(screen.getByText("Paused")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Stop Woe's Hollow/ }));
    await waitFor(() => expect(stop).toHaveBeenCalled());
  });
});
