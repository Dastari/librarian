import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import {
  AppSettingsByCategoryDocument,
  EntityAppSettingCreateDocument,
  EntityAppSettingUpdateDocument,
  EntityReleaseBlocklistListDocument,
  TriggerAutoDownloadDocument,
} from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { DownloadsSettings } from "../DownloadsSettings";

vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => true, useSession: () => ({ status: "authenticated", user: null, expiresAt: null }) }));

const setting = (key: string, value: string, category: string) => ({
  __typename: "AppSettingEdge" as const,
  node: { __typename: "AppSetting" as const, id: `set-${key}`, key, value, description: null, category, createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" },
});

const TORRENT: Array<[string, string]> = [
  ["torrent.download_dir", '"/data/downloads"'],
  ["torrent.session_dir", '"/data/session"'],
  ["torrent.listen_port", "6881"],
  ["torrent.enable_dht", "true"],
  ["torrent.max_concurrent", "5"],
  ["torrent.download_limit", "0"],
  ["torrent.upload_limit", "0"],
  ["torrent.seed_ratio_limit", "1"],
  ["torrent.seed_time_minutes", "0"],
];

const categoryMock = (category: string, rows: Array<[string, string]>) => ({
  request: { query: AppSettingsByCategoryDocument, variables: { category } },
  result: { data: { appSettings: { __typename: "AppSettingConnection", edges: rows.map(([key, value]) => setting(key, value, category)) } } },
});

const blocklistMock = {
  request: { query: EntityReleaseBlocklistListDocument, variables: { orderBy: [{ createdAt: "DESC" }], page: { limit: 100, offset: 0 } } },
  result: { data: { releaseBlocklists: { __typename: "ReleaseBlocklistConnection", edges: [], pageInfo: { __typename: "PageInfo", hasNextPage: false, hasPreviousPage: false, totalCount: 0 } } } },
};

const updateMock = (id: string, value: string) => {
  const called = vi.fn(() => ({ data: { updateAppSetting: { __typename: "UpdateAppSettingResult", success: true, error: null, appSetting: null } } }));
  return { called, mock: { request: { query: EntityAppSettingUpdateDocument, variables: { id, input: { value } } }, result: called } };
};

const createMock = (key: string, value: string, category: string) => {
  const called = vi.fn(() => ({ data: { createAppSetting: { __typename: "CreateAppSettingResult", success: true, error: null, appSetting: null } } }));
  return { called, mock: { request: { query: EntityAppSettingCreateDocument, variables: { input: { key, value, category } } }, result: called } };
};

const base = (extra: unknown[] = []) => [
  categoryMock("torrent", TORRENT),
  categoryMock("auto_download", [
    ["auto_download.enabled", "true"],
    ["auto_download.interval_minutes", "60"],
    ["auto_download.retry_after_minutes", "240"],
  ]),
  blocklistMock,
  ...extra,
];

const mount = (extra: unknown[] = []) => renderWithProviders(<DownloadsSettings />, { mocks: base(extra) as never });

/** Number fields commit on blur, so every edit ends with a Tab out of the field. */
const setNumber = async (label: string, value: string) => {
  // The label text is on both the <label> and the group, so pick the input itself.
  const input = (await screen.findAllByLabelText(label)).find((node) => node.tagName === "INPUT")!;
  await userEvent.clear(input);
  await userEvent.type(input, value);
  await userEvent.tab();
};

describe("DownloadsSettings", () => {
  it("fills the form from the stored settings", async () => {
    await mount();
    expect(await screen.findByDisplayValue("/data/downloads")).toBeInTheDocument();
    expect(screen.getByDisplayValue("/data/session")).toBeInTheDocument();
    expect(screen.getByDisplayValue("6,881")).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: /Enable DHT/ })).toBeChecked();
    expect(screen.getByRole("switch", { name: /Search and download automatically/ })).toBeChecked();
  });

  it("keeps Save disabled until something changes", async () => {
    await mount();
    await screen.findByDisplayValue("/data/downloads");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
  });

  it("writes the seeding keys when the seeding panel changes", async () => {
    const ratio = updateMock("set-torrent.seed_ratio_limit", "2");
    const time = updateMock("set-torrent.seed_time_minutes", "120");
    const remove = createMock("torrent.remove_after_import", "true", "torrent");
    await mount([ratio.mock, time.mock, remove.mock]);

    await setNumber("Seed until ratio", "2");
    await setNumber("Seed for (minutes)", "120");
    await userEvent.click(screen.getByRole("switch", { name: /Remove the torrent and its files after import/ }));

    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(ratio.called).toHaveBeenCalled();
    expect(time.called).toHaveBeenCalled();
    // `torrent.remove_after_import` has no row yet, so it is created rather than updated.
    expect(remove.called).toHaveBeenCalled();
  });

  it("only writes the keys that actually changed", async () => {
    const dht = updateMock("set-torrent.enable_dht", "false");
    const untouched = updateMock("set-torrent.listen_port", "6881");
    await mount([dht.mock, untouched.mock]);
    await screen.findByDisplayValue("/data/downloads");
    await userEvent.click(screen.getByRole("switch", { name: /Enable DHT/ }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(dht.called).toHaveBeenCalled());
    expect(untouched.called).not.toHaveBeenCalled();
  });

  it("discards edits", async () => {
    await mount();
    await setNumber("Seed until ratio", "9");
    expect(screen.getByRole("button", { name: "Save" })).toBeEnabled();
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    await vi.waitFor(() => expect(screen.getByRole("button", { name: "Save" })).toBeDisabled());
  });

  it("runs auto-download now and reports the counts", async () => {
    const trigger = vi.fn(() => ({
      data: { triggerAutoDownload: { __typename: "AutoDownloadResult", success: true, candidatesConsidered: 12, searched: 5, grabbed: 2, errors: [], error: null } },
    }));
    await mount([{ request: { query: TriggerAutoDownloadDocument, variables: { libraryId: null } }, result: trigger }]);
    await userEvent.click(await screen.findByRole("button", { name: /Run now/ }));
    await vi.waitFor(() => expect(trigger).toHaveBeenCalled());
    expect(await screen.findByText("Considered 12, searched 5, grabbed 2")).toBeInTheDocument();
  });

  it("warns when auto-download refuses to run", async () => {
    const trigger = vi.fn(() => ({
      data: { triggerAutoDownload: { __typename: "AutoDownloadResult", success: false, candidatesConsidered: 0, searched: 0, grabbed: 0, errors: ["no sources"], error: null } },
    }));
    await mount([{ request: { query: TriggerAutoDownloadDocument, variables: { libraryId: null } }, result: trigger }]);
    await userEvent.click(await screen.findByRole("button", { name: /Run now/ }));
    expect(await screen.findByText("no sources")).toBeInTheDocument();
  });

  it("refuses an empty download folder", async () => {
    await mount();
    const folder = await screen.findByDisplayValue("/data/downloads");
    await userEvent.clear(folder);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Choose a download folder")).toBeInTheDocument();
  });
});
