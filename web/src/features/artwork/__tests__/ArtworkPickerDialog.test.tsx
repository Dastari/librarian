import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ArtworkOptionsDocument, ChooseArtworkDocument, EntityAlbumUpdateDocument, RevertArtworkDocument } from "@/graphql/generated/graphql";
import { artworkUrl, sizedArtwork } from "@/lib/api/urls";
import { renderWithProviders } from "@/test";

import { ArtworkPickerDialog, languageName, optionFacts } from "../ArtworkPickerDialog";
import { withArtworkLocks, type ArtworkTarget } from "../slots";

const movie: ArtworkTarget = { entityType: "MOVIE", entityId: "m1", title: "Heat" };
const album: ArtworkTarget = { entityType: "ALBUM", entityId: "a1", title: "Appetite for Destruction" };

const slot = (over: Record<string, unknown> = {}) => ({
  __typename: "ArtworkSlotState" as const,
  entityType: "MOVIE", entityId: "m1", kind: "POSTER", hasArtwork: true, locked: false, uploaded: false, hasProvider: true, lockedFields: ["title"],
  ...over,
});

const option = (url: string, over: Record<string, unknown> = {}) => ({
  __typename: "ArtworkOption" as const,
  url, previewUrl: url.replace("/original/", "/w342/"), source: "TMDB", language: "en", width: 2000, height: 3000,
  voteAverage: 5.6, voteCount: 25, label: null, isDefault: false, isCurrent: false,
  ...over,
});

const POSTER_A = "https://image.tmdb.org/t/p/original/a.jpg";
const POSTER_B = "https://image.tmdb.org/t/p/original/b.jpg";

function optionsMock(target: ArtworkTarget, kind: string, result: { slot?: Record<string, unknown>; options?: unknown[]; unavailable?: string[] }) {
  return {
    request: { query: ArtworkOptionsDocument, variables: { entityType: target.entityType, entityId: target.entityId, kind } },
    result: { data: { artworkOptions: { __typename: "ArtworkOptions", slot: slot({ entityType: target.entityType, entityId: target.entityId, kind, ...result.slot }), options: result.options ?? [], unavailable: result.unavailable ?? [] } } },
  };
}

afterEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
});

describe("ArtworkPickerDialog", () => {
  it("lists the provider's posters with their language, size and votes, and marks the one shown now", async () => {
    await renderWithProviders(<ArtworkPickerDialog open target={movie} onOpenChange={() => {}} />, {
      router: false,
      mocks: [optionsMock(movie, "POSTER", { options: [option(POSTER_A, { isCurrent: true, isDefault: true }), option(POSTER_B, { language: "de", voteCount: 2, voteAverage: 5.3 })] })],
    });
    const grid = await screen.findByRole("group", { name: "Posters from the provider" });
    const tiles = within(grid).getAllByRole("button");
    expect(tiles).toHaveLength(2);
    expect(tiles[0]).toHaveAttribute("aria-pressed", "true");
    expect(tiles[1]).toHaveAttribute("aria-pressed", "false");
    expect(within(grid).getByText("Shown now")).toBeInTheDocument();
    expect(within(grid).getByText(/German · 2000×3000 · ★ 5\.3 \(2\)/)).toBeInTheDocument();
    // Tabs for each of a movie's pictures.
    expect(screen.getByRole("tab", { name: "Backdrop" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "Logo" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Use this poster/ })).toBeDisabled();
  });

  it("chooses another poster, locks it and loads the new picture on this device", async () => {
    const choose = vi.fn(() => ({ data: { chooseArtwork: { __typename: "ArtworkChangeResult", success: true, error: null } } }));
    const close = vi.fn();
    await renderWithProviders(<ArtworkPickerDialog open target={movie} onOpenChange={close} />, {
      router: false,
      mocks: [
        optionsMock(movie, "POSTER", { options: [option(POSTER_A, { isCurrent: true }), option(POSTER_B)] }),
        { request: { query: ChooseArtworkDocument, variables: { entityType: "MOVIE", entityId: "m1", kind: "POSTER", url: POSTER_B } }, result: choose },
      ],
    });
    const grid = await screen.findByRole("group", { name: "Posters from the provider" });
    await userEvent.click(within(grid).getAllByRole("button")[1]!);
    const use = screen.getByRole("button", { name: /Use this poster/ });
    await waitFor(() => expect(use).toBeEnabled());
    await userEvent.click(use);
    await waitFor(() => expect(choose).toHaveBeenCalled());
    await waitFor(() => expect(close).toHaveBeenCalledWith(false));
    expect(artworkUrl("movie", "m1", "poster")).toMatch(/^\/api\/artwork\/movie\/m1\/poster\?v=[0-9a-z]+$/);
  });

  it("says which provider couldn't be reached and offers an upload when there is nothing to choose", async () => {
    await renderWithProviders(<ArtworkPickerDialog open target={album} onOpenChange={() => {}} />, {
      router: false,
      mocks: [optionsMock(album, "COVER", { options: [], unavailable: ["Cover Art Archive"] })],
    });
    expect(await screen.findByText(/Couldn't reach Cover Art Archive/)).toBeInTheDocument();
    expect(screen.getByText("The provider has no covers for this")).toBeInTheDocument();
    await userEvent.click(screen.getAllByRole("button", { name: /Upload your own/ }).at(-1)!);
    expect(await screen.findByText(/JPEG, PNG or WebP, up to 20 MB/)).toBeInTheDocument();
  });

  it("uploads a picture with PUT and its own type, then closes", async () => {
    const fetch = vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(JSON.stringify({ success: true, error: null }), { status: 200, headers: { "content-type": "application/json" } }));
    const close = vi.fn();
    await renderWithProviders(<ArtworkPickerDialog open target={album} onOpenChange={close} />, {
      router: false,
      mocks: [optionsMock(album, "COVER", { options: [option("https://coverartarchive.org/release/x/1-1200.jpg", { source: "Cover Art Archive", isCurrent: true, language: null })] })],
    });
    await screen.findByRole("group", { name: "Covers from the provider" });
    await userEvent.click(screen.getByRole("tab", { name: "Upload your own" }));
    const input = document.querySelector<HTMLInputElement>('input[type="file"]')!;
    const file = new File([new Uint8Array([0xff, 0xd8, 0xff, 0xe0])], "cover.jpg", { type: "image/jpeg" });
    await userEvent.upload(input, file);
    await userEvent.click(screen.getByRole("button", { name: /Upload and use/ }));
    await waitFor(() => expect(fetch).toHaveBeenCalled());
    const [url, init] = fetch.mock.calls[0]!;
    expect(url).toBe("/api/artwork/album/a1/cover");
    expect(init).toMatchObject({ method: "PUT", headers: { "content-type": "image/jpeg" }, body: file });
    await waitFor(() => expect(close).toHaveBeenCalledWith(false));
  });

  it("shows the server's reason when an upload is refused", async () => {
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(JSON.stringify({ success: false, error: "The picture is too small: it needs to be at least 100 pixels on each side" }), { status: 422 }));
    const close = vi.fn();
    await renderWithProviders(<ArtworkPickerDialog open target={{ entityType: "PLAYLIST", entityId: "p1", title: "Road trip" }} onOpenChange={close} />, {
      router: false,
      mocks: [optionsMock({ entityType: "PLAYLIST", entityId: "p1", title: "" }, "COVER", { slot: { hasProvider: false, hasArtwork: false } })],
    });
    // A playlist has no provider: the picker opens on the upload.
    const input = await waitFor(() => { const found = document.querySelector<HTMLInputElement>('input[type="file"]'); expect(found).not.toBeNull(); return found!; });
    expect(screen.queryByRole("tab", { name: "From the provider" })).not.toBeInTheDocument();
    await userEvent.upload(input, new File([new Uint8Array([0x89, 0x50])], "tiny.png", { type: "image/png" }));
    await userEvent.click(screen.getByRole("button", { name: /Upload and use/ }));
    expect(await screen.findByText(/too small/)).toBeInTheDocument();
    expect(close).not.toHaveBeenCalled();
  });

  it("unlocks through the entity's own update and reverts through revertArtwork", async () => {
    const unlock = vi.fn(() => ({ data: { updateAlbum: { __typename: "AlbumMutationResult", success: true, error: null, album: null } } }));
    const revert = vi.fn(() => ({ data: { revertArtwork: { __typename: "ArtworkChangeResult", success: true, error: null } } }));
    await renderWithProviders(<ArtworkPickerDialog open target={album} onOpenChange={() => {}} />, {
      router: false,
      mocks: [
        optionsMock(album, "COVER", { slot: { locked: true, lockedFields: ["coverUrl", "name"] }, options: [option("https://coverartarchive.org/release/x/1-1200.jpg", { isCurrent: true, language: null })] }),
        { request: { query: EntityAlbumUpdateDocument, variables: { id: "a1", input: { lockedFields: ["name"] } } }, result: unlock },
        { request: { query: RevertArtworkDocument, variables: { entityType: "ALBUM", entityId: "a1", kind: "COVER" } }, result: revert },
      ],
    });
    expect(await screen.findByText("Locked: a metadata refresh keeps it")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Unlock/ }));
    await waitFor(() => expect(unlock).toHaveBeenCalled());
    await userEvent.click(screen.getByRole("button", { name: /Revert to provider/ }));
    await waitFor(() => expect(revert).toHaveBeenCalled());
  });
});

describe("artwork helpers", () => {
  it("keeps the entity's artwork locks when a metadata form saves its own", () => {
    expect(withArtworkLocks(["title", "posterUrl"], ["overview", "backdropUrl"])).toEqual(["title", "backdropUrl"]);
    expect(withArtworkLocks(["title"], null)).toEqual(["title"]);
  });

  it("sizes a revised picture's address without losing its revision", () => {
    expect(sizedArtwork("/api/artwork/movie/m1/poster?v=abc", 100)).toMatch(/^\/api\/artwork\/movie\/m1\/poster\?v=abc&w=\d+$/);
    expect(sizedArtwork("/api/artwork/movie/m1/poster?w=100", 300)).toBe("/api/artwork/movie/m1/poster?w=100");
  });

  it("names languages and leaves out what a picture doesn't have", () => {
    expect(languageName("en")).toBe("English");
    expect(languageName(null)).toBe("No text");
    expect(optionFacts({ source: "Cover Art Archive", label: "Back · 1987 · US", language: null, width: null, height: null, voteAverage: null, voteCount: null }, false)).toBe("Cover Art Archive");
  });
});
