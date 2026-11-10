import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";

import { ServerUpdateStatusDocument, UpgradeServerDocument, type ServerUpdateStatusFieldsFragment } from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { UpdatesSettings } from "../UpdatesSettings";
import { buildKindLabel, serverUpdateDetails, summary, upgradeRunning } from "../serverUpdate";

const settings = vi.hoisted(() => ({ values: new Map<string, string>(), loading: false, save: vi.fn() }));
vi.mock("../useAppSettings", async (importOriginal) => ({
  ...await importOriginal<typeof import("../useAppSettings")>(),
  useAppSettings: () => settings,
}));
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-1", role: "admin" } }), useIsAdmin: () => true }));

function status(overrides: Partial<ServerUpdateStatusFieldsFragment> = {}): ServerUpdateStatusFieldsFragment {
  return {
    currentVersion: "0.6.0",
    buildKind: "release",
    platform: "linux-x86_64",
    repository: "Dastari/librarian",
    channel: "stable",
    channelIsDefault: true,
    autoCheck: true,
    lastCheckedAt: "2026-11-09T12:00:00Z",
    lastError: null,
    nextCheckAt: null,
    updateAvailable: true,
    canSelfUpgrade: true,
    selfUpgradeBlockedReason: null,
    selfUpgradeInstructions: null,
    latest: {
      version: "0.7.0",
      tag: "v0.7.0",
      name: "v0.7.0",
      publishedAt: "2026-11-09T10:00:00Z",
      url: "https://github.com/Dastari/librarian/releases/tag/v0.7.0",
      notes: "## What's Changed\n* Things",
      prerelease: false,
      assetName: "librarian-0.7.0-linux-x86_64.tar.gz",
      assetSize: 32_000_000,
      hasChecksums: true,
    },
    progress: null,
    history: [],
    tvApp: null,
    ...overrides,
  };
}

/** The cache needs every object's `__typename`, which the fragment type leaves out. */
function typed(value: ServerUpdateStatusFieldsFragment) {
  return {
    __typename: "ServerUpdateStatus",
    ...value,
    latest: value.latest ? { __typename: "ServerRelease", ...value.latest } : null,
    progress: value.progress ? { __typename: "ServerUpgradeProgress", ...value.progress } : null,
    history: value.history.map((row) => ({ __typename: "ServerUpgradeRecord", ...row })),
    tvApp: value.tvApp ? { __typename: "TvAppRelease", ...value.tvApp } : null,
  };
}
const mockStatus = (value: ServerUpdateStatusFieldsFragment) => ({ request: { query: ServerUpdateStatusDocument }, result: { data: { serverUpdateStatus: typed(value) } } });

beforeAll(() => {
  // The release notes scroll in Base UI's ScrollArea, which reads animations jsdom doesn't have.
  if (!("getAnimations" in Element.prototype)) Object.defineProperty(Element.prototype, "getAnimations", { configurable: true, value: () => [] });
});

beforeEach(() => {
  settings.save.mockReset().mockResolvedValue(undefined);
});

describe("serverUpdate helpers", () => {
  it("names build kinds and summarises the check", () => {
    expect(buildKindLabel("docker")).toBe("Docker image");
    expect(buildKindLabel("other")).toBe("other");
    expect(summary(status())).toBe("Librarian 0.7.0 is available");
    expect(summary(status({ updateAvailable: false }))).toBe("Up to date (0.6.0)");
    expect(summary(status({ lastCheckedAt: null }))).toBe("Not checked yet");
    expect(summary(status({ latest: null }))).toBe("No stable releases found");
  });

  it("treats only an unfinished, unfailed upgrade as running", () => {
    expect(upgradeRunning(status())).toBe(false);
    const progress = { targetVersion: "0.7.0", stage: "downloading", message: "Downloading", startedAt: "", updatedAt: "", failed: false };
    expect(upgradeRunning(status({ progress }))).toBe(true);
    expect(upgradeRunning(status({ progress: { ...progress, stage: "failed", failed: true } }))).toBe(false);
  });

  it("reads a release notification and links only to GitHub", () => {
    expect(serverUpdateDetails(JSON.stringify({ version: "0.7.0", url: "https://github.com/Dastari/librarian/releases/tag/v0.7.0", canUpgrade: true }))).toEqual({ version: "0.7.0", url: "https://github.com/Dastari/librarian/releases/tag/v0.7.0", canUpgrade: true });
    expect(serverUpdateDetails(JSON.stringify({ version: "0.7.0", url: "javascript:alert(1)" }))?.url).toBeUndefined();
    expect(serverUpdateDetails("{nope")).toBeNull();
    expect(serverUpdateDetails(null)).toBeNull();
  });
});

describe("UpdatesSettings", () => {
  it("shows the version and offers the upgrade behind a confirmation", async () => {
    const upgrade = vi.fn(() => ({ data: { upgradeServer: { __typename: "UpgradeServerResult", success: true, error: null } } }));
    await renderWithProviders(<UpdatesSettings />, { mocks: [mockStatus(status()), { request: { query: UpgradeServerDocument, variables: { version: "0.7.0" } }, result: upgrade }] });
    expect(await screen.findByText("Release build")).toBeInTheDocument();
    expect(screen.getByText("0.6.0")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Upgrade to 0\.7\.0/ }));
    const dialog = await screen.findByRole("dialog", { name: "Upgrade to Librarian 0.7.0?" });
    expect(within(dialog).getByText(/checks it against SHA256SUMS/)).toBeInTheDocument();
    expect(upgrade).not.toHaveBeenCalled();
    await userEvent.click(within(dialog).getByRole("button", { name: "Upgrade to 0.7.0" }));
    await waitFor(() => expect(upgrade).toHaveBeenCalledTimes(1));
  });

  it("explains why a Docker install can't upgrade itself and offers no button", async () => {
    await renderWithProviders(<UpdatesSettings />, { mocks: [mockStatus(status({
      buildKind: "docker",
      canSelfUpgrade: false,
      selfUpgradeBlockedReason: "Librarian is running in a container, which is replaced, not upgraded in place.",
      selfUpgradeInstructions: "Pull the new image and recreate the container: docker compose pull && docker compose up -d.",
    }))] });
    expect(await screen.findByText("Docker image")).toBeInTheDocument();
    expect(screen.getByText(/docker compose pull/)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Upgrade to/ })).not.toBeInTheDocument();
  });

  it("explains an old release without checksums", async () => {
    await renderWithProviders(<UpdatesSettings />, { mocks: [mockStatus(status({
      canSelfUpgrade: false,
      selfUpgradeBlockedReason: "v0.7.0 was published without a SHA256SUMS file, so Librarian can't verify it and won't install it.",
      latest: { ...status().latest!, hasChecksums: false },
    }))] });
    expect(await screen.findByText(/without a SHA256SUMS file/)).toBeInTheDocument();
    expect(screen.getByText(/no SHA256SUMS/)).toBeInTheDocument();
  });

  it("saves the channel as an app setting", async () => {
    await renderWithProviders(<UpdatesSettings />, { mocks: [mockStatus(status())] });
    await screen.findByText("Release build");
    await userEvent.click(screen.getByRole("button", { name: "Dev" }));
    expect(settings.save).toHaveBeenCalledWith({ "updates.channel": "dev" });
  });
});
