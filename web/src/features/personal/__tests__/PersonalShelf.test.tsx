import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { EntityLibraryListDocument, EntityPersonalListListDocument, EntitySavedViewGetDocument, EntityTrackListDocument, PersonalEntriesDocument, type LibraryFieldsFragment, type TrackFieldsFragment } from "@/graphql/generated/graphql";

import { PersonalShelf } from "../PersonalShelf";
import { connection, operationName, OWNER, personalHarness, personalList, respond, savedView, STAMP } from "./personalHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-self", role: "admin" }, status: "authenticated" }) }));

const library: LibraryFieldsFragment = {
  id: "library-a", userId: OWNER, name: "Music", path: "/media/music", libraryType: "music", icon: null, color: null,
  autoScan: false, autoOrganize: false, namingPattern: "", scanIntervalMinutes: 60,
  watchForChanges: false, scanning: false, lastScannedAt: null, qualityProfileId: null, createdAt: STAMP, updatedAt: STAMP,
};
const track: TrackFieldsFragment = {
  id: "track-a", title: "Saved shelf track", albumId: "album-a", libraryId: "library-a", trackNumber: 1, discNumber: 1,
  musicbrainzId: null, isrc: null, durationSecs: 180, explicit: false, artistName: "Artist", artistId: null,
  wanted: false, ignored: false, mediaFileId: "file-track-a", createdAt: STAMP, updatedAt: STAMP,
};

describe("PersonalShelf saved-view recovery", () => {
  it("shows failed rules with retry and keeps the saved view's cards through a later rule refresh failure", async () => {
    const view = personalHarness({ live: true });
    await view.mount(<PersonalShelf listId="list-a" />);
    // Until the list snapshot identifies a smart collection, the existing manual
    // preview query may start. Settle that transport without mocking the shelf.
    await respond(await view.next(PersonalEntriesDocument), connection("listEntries", []));
    await respond(await view.next(EntityPersonalListListDocument), connection("personalLists", [personalList({ kind: "SMART", savedViewId: "view-a" })]));
    const first = await view.next(EntitySavedViewGetDocument);
    expect(first.operation.variables).toEqual({ id: "view-a" });
    await act(async () => { first.reject(new Error("Collection rules unavailable")); });
    expect(await screen.findByRole("alert")).toHaveTextContent("Collection rules unavailable");
    expect(screen.getByRole("link", { name: "See all" })).toHaveAttribute("href", "/lists/list-a");
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    const rules = savedView({ contentType: "TRACK", availability: "ANY" });
    await respond(await view.next(EntitySavedViewGetDocument, 1), { savedView: rules });
    await respond(await view.next(EntityLibraryListDocument), connection("libraries", [library]));
    await respond(await view.next(EntityTrackListDocument), connection("tracks", [track]));
    const card = await screen.findByRole("button", { name: "Saved shelf track" });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();

    act(() => view.registry.invalidate({ topic: "SavedView" }));
    const refresh = await view.next(EntitySavedViewGetDocument, 2);
    expect(screen.getByRole("button", { name: "Saved shelf track" })).toBe(card);
    await act(async () => { refresh.reject(new Error("Rules refresh failed")); });
    expect(await screen.findByRole("alert")).toHaveTextContent("Rules refresh failed");
    expect(screen.getByRole("button", { name: "Saved shelf track" })).toBe(card);
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    await respond(await view.next(EntitySavedViewGetDocument, 3), { savedView: rules });
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
    expect(screen.getByRole("button", { name: "Saved shelf track" })).toBe(card);
    expect(view.requests.filter(({ operation }) => operation.operationName === operationName(EntityTrackListDocument))).toHaveLength(1);
    expect(view.mutations()).toEqual([]);
    view.registry.reset(false);
  });
});
