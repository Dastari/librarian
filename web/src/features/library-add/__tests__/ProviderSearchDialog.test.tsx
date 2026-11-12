import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ActionProvider } from "glass-ux/actions";
import { TooltipProvider } from "glass-ux/tooltip";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AddAudiobookDocument, OwnedAudiobooksByAudibleDocument, SearchAudiobooksDocument } from "@/graphql/generated/graphql";

import { ProviderSearchDialog } from "../ProviderSearchDialog";

const mocks = vi.hoisted(() => ({ addBook: vi.fn(), onAdded: vi.fn(), owned: undefined as unknown }));
const results = { searchAudiobooks: [{ provider: "openlibrary", providerId: "OL1W", title: "The Long Way", authorName: "An Author", year: 2001, coverUrl: "https://covers.test/one.jpg", description: "Blurb" }] };

vi.mock("@apollo/client/react", () => ({
  useLazyQuery: (document: unknown) => [vi.fn(), { data: document === SearchAudiobooksDocument ? results : document === OwnedAudiobooksByAudibleDocument ? mocks.owned : undefined, loading: false, error: undefined }],
  useMutation: (document: unknown) => [document === AddAudiobookDocument ? mocks.addBook : vi.fn()],
}));
vi.mock("@/hooks/useDebouncedValue", () => ({ useDebouncedValue: (value: string) => value }));
vi.mock("@/features/downloads/ReleaseSearchDialog", () => ({ ReleaseSearchDialog: () => null }));
vi.mock("@tanstack/react-router", async (original) => ({
  ...(await original<typeof import("@tanstack/react-router")>()),
  Link: ({ children, className }: { children?: ReactNode; className?: string }) => <a href="/page" className={className}>{children}</a>,
}));

Element.prototype.getAnimations ??= () => [];
const wrapper = ({ children }: { children: ReactNode }) => <TooltipProvider><ActionProvider>{children}</ActionProvider></TooltipProvider>;

describe("adding from a provider search", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.owned = undefined;
    mocks.addBook.mockResolvedValue({ data: { addAudiobook: { success: true, error: null, audiobook: { id: "book-1" } } } });
  });

  it("adds without downloading, asks how to get a copy beside the artwork, then returns to the results", async () => {
    const user = userEvent.setup();
    const { baseElement } = render(<ProviderSearchDialog kind="audiobook" libraryId="library-1" isOpen onOpenChange={vi.fn()} onAdded={mocks.onAdded} initialQuery="the long way" />, { wrapper });

    expect(screen.queryByRole("checkbox")).toBeNull();
    await user.click(await screen.findByRole("button", { name: "Add The Long Way" }));

    await waitFor(() => expect(mocks.addBook).toHaveBeenCalledWith({ variables: { input: { libraryId: "library-1", providerId: "OL1W", autoDownloadMode: "NONE" } } }));
    expect(await screen.findByText("The Long Way is in your library")).toBeInTheDocument();
    expect(screen.getByText("Download automatically")).toBeInTheDocument();
    expect(baseElement.querySelector('img[src*="one.jpg"]')).toBeTruthy();
    expect(mocks.onAdded).toHaveBeenCalledOnce();

    await user.click(screen.getByRole("button", { name: "Not now" }));
    await waitFor(() => expect(screen.queryByText("The Long Way is in your library")).toBeNull());
    expect(screen.getByRole("button", { name: "The Long Way added" })).toBeInTheDocument();
  });

  it("says what's already in the library instead of offering to add it", async () => {
    mocks.owned = { audiobooks: { edges: [{ node: { id: "book-1", audibleId: "OL1W" } }] } };
    render(<ProviderSearchDialog kind="audiobook" libraryId="library-1" isOpen onOpenChange={vi.fn()} onAdded={mocks.onAdded} initialQuery="the long way" />, { wrapper });
    expect(await screen.findByText("Already in library")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Add The Long Way" })).toBeNull();
  });
});
