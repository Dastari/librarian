import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  EntityQualityProfileCreateDocument,
  EntityQualityProfileDeleteDocument,
  EntityQualityProfileUpdateDocument,
  QualityProfilesListDocument,
} from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { QualitySettings } from "../QualitySettings";

vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true, useSession: () => ({ status: "authenticated", user: null, expiresAt: null }) }));

type Profile = Record<string, unknown>;

const profile = (overrides: Profile = {}): Profile => ({
  __typename: "QualityProfile",
  id: "qp-1",
  name: "Standard",
  mediaKind: "VIDEO",
  allowedResolutions: ["2160p", "1080p"],
  allowedVideoCodecs: ["hevc", "h264"],
  allowedAudioFormats: [],
  allowedHdrTypes: ["hdr10"],
  allowedSources: ["bluray", "web"],
  releaseGroupBlacklist: [],
  releaseGroupWhitelist: [],
  requireHdr: false,
  cutoffResolution: "1080p",
  upgradeUntilCutoff: true,
  preferredLanguages: ["en"],
  requireLanguageMatch: false,
  minSizeMb: null,
  maxSizeMb: null,
  minSeeders: 1,
  maxReleaseAgeDays: null,
  preferredReleaseGroups: ["NTb"],
  allowSeasonPacks: true,
  preferProperRepack: true,
  resolutionPreference: ["2160p"],
  isDefault: false,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  ...overrides,
});

const listMock = (profiles: Profile[]) => ({
  request: { query: QualityProfilesListDocument, variables: {} },
  result: { data: { qualityProfiles: { __typename: "QualityProfileConnection", edges: profiles.map((node) => ({ __typename: "QualityProfileEdge", node })) } } },
});

/** Everything the form sends for the default profile above, so tests only state their delta. */
const savedInput = (overrides: Record<string, unknown> = {}) => ({
  name: "Standard",
  mediaKind: "VIDEO",
  allowedResolutions: ["2160p", "1080p"],
  allowedVideoCodecs: ["hevc", "h264"],
  allowedAudioFormats: [],
  allowedHdrTypes: ["hdr10"],
  allowedSources: ["bluray", "web"],
  releaseGroupWhitelist: [],
  releaseGroupBlacklist: [],
  requireHdr: false,
  cutoffResolution: "1080p",
  upgradeUntilCutoff: true,
  isDefault: false,
  preferredLanguages: ["en"],
  requireLanguageMatch: false,
  minSizeMb: null,
  maxSizeMb: null,
  minSeeders: 1,
  maxReleaseAgeDays: null,
  preferredReleaseGroups: ["NTb"],
  allowSeasonPacks: true,
  preferProperRepack: true,
  resolutionPreference: ["2160p"],
  ...overrides,
});

const updateMock = (id: string, input: unknown) => {
  const called = vi.fn(() => ({ data: { updateQualityProfile: { __typename: "UpdateQualityProfileResult", success: true, error: null, qualityProfile: null } } }));
  return { called, mock: { request: { query: EntityQualityProfileUpdateDocument, variables: { id, input } }, result: called } };
};

const mount = (profiles: Profile[], extra: unknown[] = []) => renderWithProviders(<QualitySettings />, { mocks: [listMock(profiles), ...extra] as never });

const openEditor = async (name = "Standard") => {
  await userEvent.click(await screen.findByText(name));
  return screen.findByRole("dialog");
};

describe("QualitySettings list", () => {
  it("summarises each profile", async () => {
    await mount([profile({ requireHdr: true }), profile({ id: "qp-2", name: "Lossless", mediaKind: "AUDIO", allowedAudioFormats: ["flac", "alac"], upgradeUntilCutoff: false, allowedSources: [] })]);
    const video = (await screen.findByText("Standard")).closest("tr")!;
    expect(video).toHaveTextContent("2160p/1080p · hevc/h264 · HDR required");
    expect(within(video).getByText("Video")).toBeInTheDocument();
    expect(video).toHaveTextContent("1080p");
    const audio = screen.getByText("Lossless").closest("tr")!;
    expect(audio).toHaveTextContent("flac/alac");
    expect(within(audio).getByText("Audio")).toBeInTheDocument();
    expect(audio).toHaveTextContent("Off");
    expect(audio).toHaveTextContent("Any");
  });

  it("says when there are no profiles at all", async () => {
    await mount([]);
    expect(await screen.findByText("No profiles yet")).toBeInTheDocument();
  });
});

describe("QualitySettings editor", () => {
  it("loads the profile into the form, lists first", async () => {
    await mount([profile()]);
    const dialog = await openEditor();
    expect(dialog).toHaveTextContent("Edit Standard");
    expect(within(dialog).getByDisplayValue("2160p, 1080p")).toBeInTheDocument();
    expect(within(dialog).getByDisplayValue("hevc, h264")).toBeInTheDocument();
    expect(within(dialog).getByDisplayValue("bluray, web")).toBeInTheDocument();
    expect(within(dialog).getByRole("switch", { name: /Keep upgrading until the cutoff/ })).toBeChecked();
    expect(within(dialog).getByText("English")).toBeInTheDocument();
    expect(within(dialog).getByText("NTb")).toBeInTheDocument();
  });

  it("saves the comma lists back as arrays", async () => {
    const save = updateMock("qp-1", savedInput({ allowedVideoCodecs: ["hevc", "h264", "av1"] }));
    await mount([profile()], [save.mock]);
    const dialog = await openEditor();
    await userEvent.type(within(dialog).getByDisplayValue("hevc, h264"), ", av1");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(save.called).toHaveBeenCalled());
    expect(await screen.findByText("Profile saved")).toBeInTheDocument();
  });

  it("turns an empty cutoff into null", async () => {
    const save = updateMock("qp-1", savedInput({ cutoffResolution: null }));
    await mount([profile()], [save.mock]);
    const dialog = await openEditor();
    await userEvent.clear(within(dialog).getByDisplayValue("1080p"));
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(save.called).toHaveBeenCalled());
  });

  it("drops a preferred resolution that is no longer allowed", async () => {
    const save = updateMock("qp-1", savedInput({ allowedResolutions: ["1080p"], resolutionPreference: [] }));
    await mount([profile()], [save.mock]);
    const dialog = await openEditor();
    const resolutions = within(dialog).getByDisplayValue("2160p, 1080p");
    await userEvent.clear(resolutions);
    await userEvent.type(resolutions, "1080p");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(save.called).toHaveBeenCalled());
  });

  it("offers only the allowed resolutions as preferences", async () => {
    await mount([profile({ resolutionPreference: [] })]);
    const dialog = await openEditor();
    await userEvent.click(within(dialog).getByRole("button", { name: /Add to Resolution preference/ }));
    const listbox = await screen.findByRole("listbox");
    expect(listbox).toHaveTextContent("2160p");
    expect(listbox).toHaveTextContent("1080p");
    expect(listbox.textContent).not.toContain("720p");
  });

  it("swaps the video fields for audio formats when the kind changes", async () => {
    await mount([profile({ mediaKind: "AUDIO", allowedAudioFormats: ["flac"] })]);
    const dialog = await openEditor();
    expect(within(dialog).queryByLabelText("Resolutions")).toBeNull();
    expect(within(dialog).getByDisplayValue("flac")).toBeInTheDocument();
  });

  it("refuses a profile with no name", async () => {
    await mount([profile()]);
    const dialog = await openEditor();
    await userEvent.clear(within(dialog).getByDisplayValue("Standard"));
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Name the profile")).toBeInTheDocument();
  });

  it("reports a rejected save", async () => {
    const mock = {
      request: { query: EntityQualityProfileUpdateDocument, variables: { id: "qp-1", input: savedInput() } },
      result: { data: { updateQualityProfile: { __typename: "UpdateQualityProfileResult", success: false, error: "Name already used", qualityProfile: null } } },
    };
    await mount([profile()], [mock]);
    const dialog = await openEditor();
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Name already used")).toBeInTheDocument();
  });

  it("creates a new profile from the built-in defaults", async () => {
    const created = vi.fn(() => ({ data: { createQualityProfile: { __typename: "CreateQualityProfileResult", success: true, error: null, qualityProfile: null } } }));
    await mount(
      [profile()],
      [
        {
          request: {
            query: EntityQualityProfileCreateDocument,
            variables: {
              input: {
                name: "Remux",
                mediaKind: "VIDEO",
                allowedResolutions: ["2160p", "1080p", "720p"],
                allowedVideoCodecs: ["hevc", "h264", "av1"],
                allowedAudioFormats: [],
                allowedHdrTypes: [],
                allowedSources: ["bluray", "web"],
                releaseGroupWhitelist: [],
                releaseGroupBlacklist: [],
                requireHdr: false,
                cutoffResolution: null,
                upgradeUntilCutoff: false,
                isDefault: false,
                preferredLanguages: [],
                requireLanguageMatch: false,
                minSizeMb: null,
                maxSizeMb: null,
                minSeeders: 1,
                maxReleaseAgeDays: null,
                preferredReleaseGroups: [],
                allowSeasonPacks: true,
                preferProperRepack: true,
                resolutionPreference: [],
              },
            },
          },
          result: created,
        },
      ],
    );
    await userEvent.click(await screen.findByRole("button", { name: /New profile/ }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("New quality profile");
    await userEvent.type(within(dialog).getByLabelText("Name"), "Remux");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(created).toHaveBeenCalled());
  });
});

describe("QualitySettings row actions", () => {
  it("moves the default flag off the old profile before setting the new one", async () => {
    const clear = updateMock("qp-2", { isDefault: false });
    const set = updateMock("qp-1", { isDefault: true });
    await mount([profile(), profile({ id: "qp-2", name: "Old default", isDefault: true })], [clear.mock, set.mock]);
    await userEvent.click(within((await screen.findByText("Standard")).closest("tr")!).getByRole("button", { name: /Row actions|Actions|More/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Make default" }));
    await vi.waitFor(() => expect(set.called).toHaveBeenCalled());
    expect(clear.called).toHaveBeenCalled();
  });

  it("leaves an audio default alone when a video profile becomes the default", async () => {
    const audio = updateMock("qp-3", { isDefault: false });
    const set = updateMock("qp-1", { isDefault: true });
    await mount([profile(), profile({ id: "qp-3", name: "Lossless", mediaKind: "AUDIO", isDefault: true })], [audio.mock, set.mock]);
    await userEvent.click(within((await screen.findByText("Standard")).closest("tr")!).getByRole("button", { name: /Row actions|Actions|More/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Make default" }));
    await vi.waitFor(() => expect(set.called).toHaveBeenCalled());
    expect(audio.called).not.toHaveBeenCalled();
  });

  it("confirms before deleting a profile", async () => {
    const deleted = vi.fn(() => ({ data: { deleteQualityProfile: { __typename: "DeleteQualityProfileResult", success: true, error: null } } }));
    await mount([profile()], [{ request: { query: EntityQualityProfileDeleteDocument, variables: { id: "qp-1" } }, result: deleted }]);
    await userEvent.click(within((await screen.findByText("Standard")).closest("tr")!).getByRole("button", { name: /Row actions|Actions|More/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Delete" }));
    expect(await screen.findByText("Delete Standard?")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Delete" }));
    await vi.waitFor(() => expect(deleted).toHaveBeenCalled());
  });

  it("hides both actions on the default profile", async () => {
    await mount([profile({ isDefault: true })]);
    const row = (await screen.findByText("Standard")).closest("tr")!;
    expect(within(row).queryByRole("button", { name: /Row actions|Actions|More/ })).toBeNull();
  });
});
