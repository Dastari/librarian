import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  EntityAlbumUpdateDocument,
  EntityAudiobookUpdateDocument,
  EntityLibraryGetDocument,
  EntityMovieUpdateDocument,
  EntityShowUpdateDocument,
  QualityProfilesListDocument,
  SearchMissingDocument,
} from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { AcquisitionDialog, type AcquisitionTarget } from "../AcquisitionDialog";

const profile = (id: string, name: string, isDefault = false) => ({
  __typename: "QualityProfile" as const,
  id,
  name,
  mediaKind: "VIDEO",
  allowedResolutions: ["2160p", "1080p"],
  allowedVideoCodecs: ["hevc"],
  allowedAudioFormats: [],
  allowedHdrTypes: [],
  allowedSources: ["bluray"],
  releaseGroupBlacklist: [],
  releaseGroupWhitelist: [],
  requireHdr: false,
  cutoffResolution: null,
  upgradeUntilCutoff: false,
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
  isDefault,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
});

const profilesMock = {
  request: { query: QualityProfilesListDocument, variables: {} },
  result: {
    data: {
      qualityProfiles: {
        __typename: "QualityProfileConnection",
        edges: [
          { __typename: "QualityProfileEdge", node: profile("qp-1", "Standard", true) },
          { __typename: "QualityProfileEdge", node: profile("qp-2", "Remux") },
        ],
      },
    },
  },
};

const libraryMock = (qualityProfileId: string | null) => ({
  request: { query: EntityLibraryGetDocument, variables: { id: "lib-1" } },
  result: {
    data: {
      library: {
        __typename: "Library",
        id: "lib-1",
        userId: "u1",
        name: "Shows",
        path: "/media/shows",
        libraryType: "tv",
        icon: null,
        color: null,
        autoScan: false,
        autoOrganize: false,
        namingPattern: null,
        scanIntervalMinutes: 60,
        watchForChanges: false,
        scanning: false,
        lastScannedAt: null,
        qualityProfileId,
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-01-01T00:00:00Z",
      },
    },
  },
});

const updateMock = (query: unknown, field: string, variables: unknown) => {
  const called = vi.fn(() => ({ data: { [field]: { __typename: "Result", success: true, error: null } } }));
  return { called, mock: { request: { query, variables }, result: called } };
};

const showTarget: AcquisitionTarget = { kind: "show", id: "show-1", title: "Andor", libraryId: "lib-1", autoDownloadMode: "WANTED", qualityProfileId: null };

const mount = (target: AcquisitionTarget, mocks: unknown[], onSaved = vi.fn()) =>
  renderWithProviders(<AcquisitionDialog isOpen onOpenChange={() => {}} target={target} onSaved={onSaved} />, { mocks: [profilesMock, libraryMock("qp-1"), ...mocks] as never });

describe("AcquisitionDialog for a show", () => {
  it("opens on the title's current mode and explains it", async () => {
    await mount(showTarget, []);
    expect(screen.getByRole("dialog")).toHaveTextContent("Andor");
    expect(screen.getByRole("radio", { name: "Wanted only" })).toBeChecked();
    expect(screen.getByText("Only items you mark as wanted are searched for and downloaded.")).toBeInTheDocument();
  });

  it("maps each mode onto autoDownload and autoDownloadMode", async () => {
    const everything = updateMock(EntityShowUpdateDocument, "updateShow", {
      id: "show-1",
      input: { autoDownload: true, autoDownloadMode: "ALL", qualityProfileId: null },
    });
    await mount(showTarget, [everything.mock]);
    await userEvent.click(screen.getByRole("radio", { name: "Everything missing" }));
    expect(screen.getByText("Every item without a file is searched for and downloaded as it appears.")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(everything.called).toHaveBeenCalled());
    expect(await screen.findByText("Download settings saved")).toBeInTheDocument();
  });

  it("turns automatic downloads off with the NONE mode", async () => {
    const off = updateMock(EntityShowUpdateDocument, "updateShow", { id: "show-1", input: { autoDownload: false, autoDownloadMode: "NONE", qualityProfileId: null } });
    const onSaved = vi.fn();
    await mount(showTarget, [off.mock], onSaved);
    await userEvent.click(screen.getByRole("radio", { name: "Off" }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(off.called).toHaveBeenCalled());
    expect(onSaved).toHaveBeenCalled();
  });

  it("names the library's profile in the inherit option and saves null for it", async () => {
    const inherit = updateMock(EntityShowUpdateDocument, "updateShow", { id: "show-1", input: { autoDownload: true, autoDownloadMode: "WANTED", qualityProfileId: null } });
    await mount(showTarget, [inherit.mock]);
    // The label appears on the trigger and in the list; both must name the library default.
    expect((await screen.findAllByText("Inherit from library (Standard)")).length).toBeGreaterThan(0);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(inherit.called).toHaveBeenCalled());
  });

  it("saves the chosen profile id when the title overrides the library", async () => {
    const override = updateMock(EntityShowUpdateDocument, "updateShow", { id: "show-1", input: { autoDownload: true, autoDownloadMode: "WANTED", qualityProfileId: "qp-2" } });
    await mount({ ...showTarget, qualityProfileId: "qp-2" }, [override.mock]);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(override.called).toHaveBeenCalled());
  });

  it("reports a rejected save", async () => {
    const mock = {
      request: { query: EntityShowUpdateDocument, variables: { id: "show-1", input: { autoDownload: true, autoDownloadMode: "WANTED", qualityProfileId: null } } },
      result: { data: { updateShow: { __typename: "Result", success: false, error: "Library is read only", show: null } } },
    };
    const onSaved = vi.fn();
    await mount(showTarget, [mock], onSaved);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Library is read only")).toBeInTheDocument();
    expect(onSaved).not.toHaveBeenCalled();
  });
});

describe("AcquisitionDialog for a movie", () => {
  const movieTarget: AcquisitionTarget = { kind: "movie", id: "mv-1", title: "Dune", libraryId: "lib-1", monitored: false, wanted: false, qualityProfileId: null };

  it("uses the monitored and wanted switches instead of a mode", async () => {
    await mount(movieTarget, []);
    expect(screen.getByRole("switch", { name: "Monitored" })).not.toBeChecked();
    expect(screen.getByRole("switch", { name: "Wanted" })).not.toBeChecked();
    expect(screen.queryByRole("radiogroup", { name: "Automatic downloads" })).toBeNull();
  });

  it("turning on wanted also monitors the movie", async () => {
    const save = updateMock(EntityMovieUpdateDocument, "updateMovie", { id: "mv-1", input: { monitored: true, wanted: true, qualityProfileId: null } });
    await mount(movieTarget, [save.mock]);
    await userEvent.click(screen.getByRole("switch", { name: "Wanted" }));
    expect(screen.getByRole("switch", { name: "Monitored" })).toBeChecked();
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(save.called).toHaveBeenCalled());
  });
});

describe("AcquisitionDialog search now", () => {
  it("scopes the search to the title and reports the counts", async () => {
    const search = vi.fn(() => ({ data: { searchMissing: { __typename: "SearchMissingResult", success: true, error: null, queued: 2, searched: 5 } } }));
    await mount(showTarget, [{ request: { query: SearchMissingDocument, variables: { input: { showId: "show-1" } } }, result: search }]);
    await userEvent.click(screen.getByRole("button", { name: /Search now/ }));
    await vi.waitFor(() => expect(search).toHaveBeenCalled());
    expect(await screen.findByText("Searched 5, grabbed 2")).toBeInTheDocument();
  });

  it("says when there is nothing missing", async () => {
    const search = vi.fn(() => ({ data: { searchMissing: { __typename: "SearchMissingResult", success: true, error: null, queued: 0, searched: 0 } } }));
    await mount(showTarget, [{ request: { query: SearchMissingDocument, variables: { input: { showId: "show-1" } } }, result: search }]);
    await userEvent.click(screen.getByRole("button", { name: /Search now/ }));
    expect(await screen.findByText("Nothing is missing here")).toBeInTheDocument();
  });

  it("scopes by album, audiobook and movie id too", async () => {
    for (const [target, input] of [
      [{ kind: "album", id: "al-1", title: "Kid A", libraryId: "lib-1" }, { albumId: "al-1" }],
      [{ kind: "audiobook", id: "ab-1", title: "Dune", libraryId: "lib-1" }, { audiobookId: "ab-1" }],
      [{ kind: "movie", id: "mv-1", title: "Dune", libraryId: "lib-1" }, { movieId: "mv-1" }],
    ] as Array<[AcquisitionTarget, unknown]>) {
      const search = vi.fn(() => ({ data: { searchMissing: { __typename: "SearchMissingResult", success: true, error: null, queued: 1, searched: 1 } } }));
      const { unmount } = await mount(target, [{ request: { query: SearchMissingDocument, variables: { input } }, result: search }]);
      await userEvent.click(screen.getByRole("button", { name: /Search now/ }));
      await vi.waitFor(() => expect(search).toHaveBeenCalled());
      unmount();
    }
  });
});

describe("AcquisitionDialog for albums and audiobooks", () => {
  it("saves an album through the album mutation", async () => {
    const save = updateMock(EntityAlbumUpdateDocument, "updateAlbum", { id: "al-1", input: { autoDownload: true, autoDownloadMode: "ALL", qualityProfileId: null } });
    await mount({ kind: "album", id: "al-1", title: "Kid A", libraryId: "lib-1", autoDownloadMode: "ALL" }, [save.mock]);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(save.called).toHaveBeenCalled());
  });

  it("saves an audiobook through the audiobook mutation", async () => {
    const save = updateMock(EntityAudiobookUpdateDocument, "updateAudiobook", { id: "ab-1", input: { autoDownload: false, autoDownloadMode: "NONE", qualityProfileId: null } });
    await mount({ kind: "audiobook", id: "ab-1", title: "Dune", libraryId: "lib-1" }, [save.mock]);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(save.called).toHaveBeenCalled());
  });
});
