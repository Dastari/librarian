import { useApolloClient } from "@apollo/client/react";
import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useReducer } from "react";
import { describe, expect, it, vi } from "vitest";

import {
  AppSettingsByCategoryDocument,
  EntityAppSettingCreateDocument,
  EntityAppSettingUpdateDocument,
  EntityReleaseBlocklistListDocument,
  TriggerAutoDownloadDocument,
} from "@/graphql/generated/graphql";
import { renderWithProviders } from "@/test";

import { DownloadsSettings, minutesToSeconds, secondsToMinutes } from "../DownloadsSettings";

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
    ["auto_download.search_backoff_minutes", "360"],
  ]),
  blocklistMock,
  ...extra,
];

const mount = (extra: unknown[] = []) => renderWithProviders(<DownloadsSettings />, { mocks: base(extra) as never });

/** A server that has never been given an automatic-downloads setting. */
const mountFresh = (extra: unknown[] = []) => renderWithProviders(<DownloadsSettings />, {
  mocks: [categoryMock("torrent", TORRENT), categoryMock("auto_download", []), blocklistMock, ...extra] as never,
});

/** Duration and size fields read what was typed and commit on blur. */
const setText = async (label: string, value: string) => {
  const input = screen.getByRole("textbox", { name: label });
  await userEvent.clear(input);
  await userEvent.type(input, value);
  await userEvent.tab();
};

/** Number fields commit on blur, so every edit ends with a Tab out of the field. */
const setNumber = async (label: string, value: string) => {
  // The label text is on both the <label> and the group, so pick the input itself.
  const input = (await screen.findAllByLabelText(label)).find((node) => node.tagName === "INPUT")!;
  await userEvent.clear(input);
  await userEvent.type(input, value);
  await userEvent.tab();
};

describe("interval conversion", () => {
  it("moves between the stored minutes and the form's seconds", () => {
    expect(minutesToSeconds(90)).toBe(5400);
    expect(secondsToMinutes(5400)).toBe(90);
    expect(secondsToMinutes(minutesToSeconds(10080))).toBe(10080);
    expect(minutesToSeconds(0)).toBe(0);
  });
});

describe("DownloadsSettings", () => {
  it("fills the form from the stored settings", async () => {
    await mount();
    expect(await screen.findByText("/data/downloads")).toBeInTheDocument();
    expect(screen.getByDisplayValue("/data/session")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Listen port" })).toHaveValue("6881");
    expect(screen.getByRole("switch", { name: /Enable DHT/ })).toBeChecked();
    expect(screen.getByRole("switch", { name: /Search and download automatically/ })).toBeChecked();
  });

  it("shows automatic downloads off, with the real defaults, until the server has a setting", async () => {
    await mountFresh();
    await screen.findByDisplayValue("/data/session");
    // The server does nothing without `auto_download.enabled`, so the switch must not claim otherwise.
    expect(screen.getByRole("switch", { name: /Search and download automatically/ })).not.toBeChecked();
    // Stored as 60 and 360 minutes, shown as durations.
    expect(screen.getByRole("textbox", { name: "Check everything every" })).toHaveValue("1h");
    expect(screen.getByRole("textbox", { name: "Search a missing item again after" })).toHaveValue("6h");
    expect(screen.getByText("Nothing has run yet.")).toBeInTheDocument();
  });

  it("writes the scheduling keys the server reads", async () => {
    const grace = createMock("auto_download.air_grace_minutes", "120", "auto_download");
    const window = createMock("auto_download.air_window_days", "3", "auto_download");
    const cap = createMock("auto_download.max_concurrent_grabs", "2", "auto_download");
    await mount([grace.mock, window.mock, cap.mock]);
    await setText("Wait after an episode airs", "2h");
    await setNumber("Watch new episodes for", "3");
    await setNumber("Most automatic downloads at once", "2");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(grace.called).toHaveBeenCalled();
    expect(window.called).toHaveBeenCalled();
    expect(cap.called).toHaveBeenCalled();
  });

  it("offers the search budget and partial pack downloads, with the real defaults and the ratio warning", async () => {
    await mountFresh();
    await screen.findByDisplayValue("/data/session");
    // 30 searches an hour, and only the missing files of a pack, until someone says otherwise.
    expect(screen.getAllByDisplayValue("30").length).toBeGreaterThan(0);
    expect(screen.getByRole("switch", { name: /Download only the missing files from packs/ })).toBeChecked();
    expect(screen.getByText(/can cost ratio/)).toBeInTheDocument();
  });

  it("writes the keys the server reads for the budget and partial downloads", async () => {
    const budget = createMock("auto_download.hourly_search_budget", "10", "auto_download");
    const partial = createMock("auto_download.only_missing_files", "false", "auto_download");
    await mountFresh([budget.mock, partial.mock]);
    await setNumber("Indexer searches for recommendations", "10");
    await userEvent.click(screen.getByRole("switch", { name: /Download only the missing files from packs/ }));
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(budget.called).toHaveBeenCalled();
    expect(partial.called).toHaveBeenCalled();
  });

  it("says what the scheduler last did", async () => {
    const at = new Date(Date.now() - 5 * 60_000).toISOString();
    const status = JSON.stringify({ sweep: { at, searched: 3, grabbed: 1, capped: 2 }, rss: { at, feeds: 2, failed: 1, newItems: 40, matched: 3, grabbed: 1 } });
    await renderWithProviders(<DownloadsSettings />, {
      mocks: [categoryMock("torrent", TORRENT), categoryMock("auto_download", [["auto_download.enabled", "true"], ["auto_download.status", status]]), blocklistMock] as never,
    });
    expect(await screen.findByText(/searched 3, grabbed 1, 2 waiting for a free slot/)).toBeInTheDocument();
    expect(screen.getByText(/2 feed\(s\), 1 failed, 40 new item\(s\), 3 matched, 1 grabbed/)).toBeInTheDocument();
  });

  it("explains what each automatic-download choice does", async () => {
    await mount();
    expect(await screen.findByText("Shows, albums and audiobooks")).toBeInTheDocument();
    expect(screen.getByText(/Wanted only: Only items you mark as wanted/)).toBeInTheDocument();
    expect(screen.getByText(/Everything missing: Every item without a file/)).toBeInTheDocument();
    expect(screen.getByText("Movies")).toBeInTheDocument();
    expect(screen.getByText("New episodes")).toBeInTheDocument();
  });

  it("keeps Save disabled until something changes", async () => {
    await mount();
    await screen.findByDisplayValue("/data/session");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
  });

  it("writes the seeding keys when the seeding panel changes", async () => {
    const ratio = updateMock("set-torrent.seed_ratio_limit", "2");
    const time = updateMock("set-torrent.seed_time_minutes", "120");
    const remove = createMock("torrent.remove_after_import", "true", "torrent");
    await mount([ratio.mock, time.mock, remove.mock]);

    await setNumber("Seed until ratio", "2");
    await setText("Seed for", "2h");
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
    await screen.findByDisplayValue("/data/session");
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

  it("keeps a cleared numeric limit empty and lets the form schema reject it", async () => {
    const update = updateMock("set-torrent.download_limit", "0");
    await mount([update.mock]);
    await screen.findByDisplayValue("/data/session");
    const input = screen.getByRole("textbox", { name: "Download speed limit" });
    await userEvent.clear(input);
    await userEvent.tab();
    expect(input).toHaveValue("");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(input).toHaveAttribute("aria-invalid", "true"));
    await vi.waitFor(() => expect(input).toHaveFocus());
    expect(update.called).not.toHaveBeenCalled();
  });

  it("refuses an empty session folder", async () => {
    await mount();
    const folder = await screen.findByDisplayValue("/data/session");
    await userEvent.clear(folder);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Choose a session folder")).toBeInTheDocument();
  });

  it("shows the download folder and moves it only through the move dialog", async () => {
    await mount();
    expect(await screen.findByText("/data/downloads")).toBeInTheDocument();
    expect(screen.queryByDisplayValue("/data/downloads")).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /Move…/ }));
    expect(await screen.findByRole("dialog", { name: "Move the download folder" })).toBeInTheDocument();
    expect(screen.getByText(/Librarian does not copy your downloads/)).toBeInTheDocument();
  });

  it("preserves an edited folder while another settings snapshot arrives", async () => {
    let receiveSnapshot: () => void = () => { throw new Error("Settings client has not mounted"); };
    function SettingsWithSnapshot() {
      const client = useApolloClient();
      receiveSnapshot = () => {
        client.writeQuery({
          query: AppSettingsByCategoryDocument,
          variables: { category: "auto_download" },
          data: { appSettings: { edges: [
            setting("auto_download.enabled", "true", "auto_download"),
            setting("auto_download.interval_minutes", "60", "auto_download"),
            setting("auto_download.search_backoff_minutes", "480", "auto_download"),
          ] } },
        });
      };
      return <DownloadsSettings />;
    }
    await renderWithProviders(<SettingsWithSnapshot />, { mocks: base() as never });
    const folder = await screen.findByDisplayValue("/data/session");
    await screen.findByDisplayValue("6h");
    await userEvent.clear(folder);
    act(() => receiveSnapshot());
    await screen.findByDisplayValue("8h");
    expect(folder).toHaveValue("");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Choose a session folder")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(folder).toHaveValue("/data/session");
    expect(screen.getByDisplayValue("8h")).toBeInTheDocument();
  });
  it("retains the folder draft on a failed save and can discard it", async () => {
    await mount([{
      request: { query: EntityAppSettingUpdateDocument, variables: { id: "set-torrent.session_dir", input: { value: "/new/session" } } },
      error: new Error("Download settings unavailable"),
    }]);
    const folder = await screen.findByDisplayValue("/data/session");
    await userEvent.clear(folder);
    await userEvent.type(folder, "/new/session");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Download settings unavailable"));
    expect(folder).toHaveValue("/new/session");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(folder).toHaveValue("/data/session");
  });

  it("shows stored minutes as durations and writes edited ones back as minutes", async () => {
    const interval = updateMock("set-auto_download.interval_minutes", "90");
    const backoff = updateMock("set-auto_download.search_backoff_minutes", "1440");
    const untouched = updateMock("set-torrent.seed_time_minutes", "0");
    await mount([interval.mock, backoff.mock, untouched.mock]);
    expect(await screen.findByRole("textbox", { name: "Check everything every" })).toHaveValue("1h");
    await setText("Check everything every", "1.5 hours");
    await setText("Search a missing item again after", "1 day");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(interval.called).toHaveBeenCalled();
    expect(backoff.called).toHaveBeenCalled();
    expect(untouched.called).not.toHaveBeenCalled();
  });

  it("reads a bare number in an interval as minutes, the unit the setting was in", async () => {
    const grace = createMock("auto_download.air_grace_minutes", "45", "auto_download");
    await mountFresh([grace.mock]);
    await setText("Wait after an episode airs", "45");
    expect(screen.getByRole("textbox", { name: "Wait after an episode airs" })).toHaveValue("45m");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(grace.called).toHaveBeenCalled();
  });

  it("refuses an interval outside what the server accepts", async () => {
    await mount();
    await setText("Check everything every", "1m");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await vi.waitFor(() => expect(screen.getByRole("textbox", { name: "Check everything every" })).toHaveAttribute("aria-invalid", "true"));
  });

  it("writes speed limits in bytes per second", async () => {
    const down = updateMock("set-torrent.download_limit", "2000000");
    const up = updateMock("set-torrent.upload_limit", "500000");
    await mount([down.mock, up.mock]);
    expect(await screen.findByRole("textbox", { name: "Download speed limit" })).toBeInTheDocument();
    await setText("Download speed limit", "2 MB");
    await setText("Upload speed limit", "500 kB");
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(down.called).toHaveBeenCalled();
    expect(up.called).toHaveBeenCalled();
  });

  it("keeps stored values across an unrelated rerender", async () => {
    let refresh = () => {};
    function Harness() {
      const [, rerender] = useReducer((value: number) => value + 1, 0);
      refresh = rerender;
      return <DownloadsSettings />;
    }
    await renderWithProviders(<Harness />, { mocks: base() as never });
    await screen.findByDisplayValue("/data/session");
    act(() => refresh());
    expect(screen.getByDisplayValue("/data/session")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
  });

});
