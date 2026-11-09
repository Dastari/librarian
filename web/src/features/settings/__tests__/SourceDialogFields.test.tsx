import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { SourceDialog } from "../SourceDialog";

vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true, useSession: () => ({ status: "authenticated", user: { id: "me" }, expiresAt: null }) }));

const source = {
  id: "src-a", name: "IPTorrents", sourceType: "torrent_indexer", definitionId: "iptorrents", enabled: true, priority: 1, mediaTypes: "movies,tv", siteUrl: "https://iptorrents.com/",
  supportsSearch: true, supportsTvSearch: true, supportsMovieSearch: true, supportsMusicSearch: false, supportsBookSearch: false, settings: null,
  lastError: null, errorCount: 0, lastSuccessAt: null, lastErrorAt: null, healthStatus: "ok", healthKind: null, lastCheckedAt: null, nextCheckAt: null, loginAttemptedAt: null,
  createdAt: "2026-11-01T00:00:00Z", updatedAt: "2026-11-01T00:00:00Z",
};

const open = () => renderWithProviders(<SourceDialog target={{ kind: "source", source: source as never }} isOpen onOpenChange={() => {}} />, { mocks: [] });

describe("SourceDialog credentials", () => {
  it("takes the address as a URL field and the password as a masked field the browser leaves alone", async () => {
    await open();
    const site = await screen.findByDisplayValue("https://iptorrents.com/");
    expect(site).toHaveAttribute("inputmode", "url");
    const password = screen.getByLabelText(/^Password/, { selector: "input" });
    expect(password).toHaveAttribute("type", "password");
    expect(password).toHaveAttribute("autocomplete", "off");
  });

  it("switches sign-in method with a segmented control, and masks the cookie too", async () => {
    await open();
    await screen.findByDisplayValue("https://iptorrents.com/");
    expect(screen.getByRole("button", { name: "Username and password" })).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Cookie (advanced)" }));
    const cookie = screen.getByLabelText(/^Cookie/, { selector: "input" });
    expect(cookie).toHaveAttribute("type", "password");
    expect(cookie).toHaveAttribute("autocomplete", "off");
    expect(screen.queryByLabelText(/^Username/, { selector: "input" })).not.toBeInTheDocument();
    expect(screen.getByLabelText("User agent", { selector: "input" })).toBeInTheDocument();
  });
});
