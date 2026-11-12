import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useReducer } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";
import { CastingSettings } from "../CastingSettings";

const state = vi.hoisted(() => ({
  admin: true,
  setting: null as null | { id: string; autoDiscoveryEnabled: boolean; discoveryIntervalSeconds: number; defaultVolume: number; transcodeIncompatible: boolean; preferredQuality: string | null },
  loading: false,
  settingsRefetch: vi.fn(), devicesRefetch: vi.fn(),
  create: vi.fn(), update: vi.fn(), add: vi.fn(), discover: vi.fn(), other: vi.fn(),
}));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => state.admin, useSession: () => ({ user: { id: "u1" } }) }));
vi.mock("../useSettingsConnection", () => ({
  useSettingsConnection: () => ({ rows: [], loading: false, error: undefined, refetch: state.devicesRefetch }),
}));
vi.mock("@/lib/graphql/live/hooks", () => ({
  useLiveQuery: () => ({ data: { castSettings: { edges: state.setting ? [{ node: state.setting }] : [] } }, loading: state.loading, refetch: state.settingsRefetch }),
}));
vi.mock("@apollo/client/react", async (original) => {
  const docs = await import("@/graphql/generated/graphql");
  return {
    ...await original<typeof import("@apollo/client/react")>(),
    useMutation: (document: unknown) => [
      document === docs.EntityCastSettingCreateDocument ? state.create
        : document === docs.EntityCastSettingUpdateDocument ? state.update
          : document === docs.EntityCastDeviceCreateDocument ? state.add
            : document === docs.DiscoverCastDevicesDocument ? state.discover : state.other,
      { loading: false },
    ],
  };
});
let refresh: () => void;
function Harness() {
  const [, rerender] = useReducer((value: number) => value + 1, 0);
  refresh = rerender;
  return <CastingSettings />;
}
const mount = () => renderWithProviders(<Harness />, { router: false });
const input = (label: string) => screen.getAllByLabelText(label).find((node) => node.tagName === "INPUT")!;
const edit = async (label: string, value: string) => {
  const field = input(label);
  await userEvent.clear(field);
  if (value) await userEvent.type(field, value);
  await userEvent.tab();
  return field;
};
const save = () => screen.getByRole("button", { name: "Save" });
const quality = () => screen.getByRole("combobox", { name: "Highest quality" });
const chooseQuality = async (label: RegExp) => {
  await userEvent.click(quality());
  await userEvent.click(await screen.findByRole("option", { name: label }));
};
const openDevice = async () => {
  await userEvent.click(screen.getByRole("button", { name: "Add by address" }));
  return within(await screen.findByRole("dialog"));
};

beforeEach(() => {
  vi.clearAllMocks();
  state.admin = true;
  state.loading = false;
  state.setting = { id: "cast-settings", autoDiscoveryEnabled: false, discoveryIntervalSeconds: 600, defaultVolume: 0.5, transcodeIncompatible: false, preferredQuality: "720p" };
  state.settingsRefetch.mockReset().mockResolvedValue(undefined);
  state.devicesRefetch.mockReset().mockResolvedValue(undefined);
  state.create.mockReset().mockResolvedValue({ data: { createCastSetting: { success: true, error: null } } });
  state.update.mockReset().mockResolvedValue({ data: { updateCastSetting: { success: true, error: null } } });
  state.add.mockReset().mockResolvedValue({ data: { createCastDevice: { success: true, error: null } } });
  state.discover.mockReset().mockResolvedValue({ data: { discoverCastDevices: [{ id: "found-1" }] } });
});

describe("CastingSettings bound forms", () => {
  it("keeps saved values across unrelated rerenders and restores them after discard", async () => {
    await mount();
    expect(quality()).toHaveTextContent("720p");
    expect(input("Look every")).toHaveValue("10m");
    expect(screen.getByRole("switch", { name: "Look for receivers automatically" })).not.toBeChecked();
    act(() => refresh());
    expect(quality()).toHaveTextContent("720p");
    expect(save()).toBeDisabled();
    await chooseQuality(/^1080p/);
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(quality()).toHaveTextContent("720p");
    expect(save()).toBeDisabled();
  });

  it("preserves a draft across live settings changes and discards to the latest server baseline", async () => {
    await mount();
    await chooseQuality(/^1080p/);
    act(() => {
      state.setting = { ...state.setting!, preferredQuality: "2160p", discoveryIntervalSeconds: 900 };
      refresh();
    });
    expect(quality()).toHaveTextContent("1080p");
    expect(input("Look every")).toHaveValue("15m");
    act(() => refresh());
    expect(quality()).toHaveTextContent("1080p");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(quality()).toHaveTextContent("4K (2160p)");
    expect(input("Look every")).toHaveValue("15m");
    expect(save()).toBeDisabled();
  });

  it("updates the existing record, preserves zero volume and sends no limit as null", async () => {
    await mount();
    await edit("Starting volume", "0");
    await chooseQuality(/^No limit/);
    await userEvent.click(save());
    await screen.findByText("Settings saved");
    expect(state.update).toHaveBeenCalledWith({ variables: { id: "cast-settings", input: { autoDiscoveryEnabled: false, discoveryIntervalSeconds: 600, defaultVolume: 0, transcodeIncompatible: false, preferredQuality: null } } });
    expect(state.create).not.toHaveBeenCalled();
    expect(state.settingsRefetch).toHaveBeenCalled();
    act(() => refresh());
    expect(quality()).toHaveTextContent("No limit");
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
  });

  it("creates settings when there is no stored record", async () => {
    state.setting = null;
    await mount();
    await chooseQuality(/^1080p/);
    await userEvent.click(save());
    await waitFor(() => expect(state.create).toHaveBeenCalledWith({ variables: { input: { autoDiscoveryEnabled: true, discoveryIntervalSeconds: 300, defaultVolume: 0.8, transcodeIncompatible: true, preferredQuality: "1080p" } } }));
    expect(state.update).not.toHaveBeenCalled();
  });

  it("rejects an empty numeric value and focuses the field without mutation", async () => {
    await mount();
    const interval = await edit("Look every", "");
    await userEvent.click(save());
    expect(await screen.findByText("Enter a number")).toBeInTheDocument();
    await waitFor(() => expect(interval).toHaveFocus());
    expect(interval).toHaveValue("");
    expect(state.update).not.toHaveBeenCalled();
  });

  it("holds the draft and disables controls during a pending save, including live refreshes", async () => {
    let reject!: (error: Error) => void;
    state.update.mockImplementationOnce(() => new Promise((_resolve, fail) => { reject = fail; }));
    await mount();
    await chooseQuality(/^1080p/);
    await userEvent.click(save());
    await waitFor(() => expect(state.update).toHaveBeenCalled());
    expect(quality()).toBeDisabled();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
    act(() => {
      state.setting = { ...state.setting!, preferredQuality: "2160p" };
      refresh();
    });
    expect(quality()).toHaveTextContent("1080p");
    expect(quality()).toBeDisabled();
    await act(async () => reject(new Error("Cast settings unavailable")));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Cast settings unavailable"));
    expect(quality()).toHaveTextContent("1080p");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(quality()).toHaveTextContent("4K (2160p)");
  });

  it("keeps settings, discovery and manual additions to administrators (the server requires it)", async () => {
    state.admin = false;
    await mount();
    expect(screen.queryByRole("button", { name: "Save" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Add by address" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Look now" })).not.toBeInTheDocument();
    expect(state.update).not.toHaveBeenCalled();
  });

  it("looks for receivers now and refreshes the list", async () => {
    await mount();
    await userEvent.click(screen.getByRole("button", { name: "Look now" }));
    await waitFor(() => expect(state.discover).toHaveBeenCalled());
    expect(await screen.findByText("Found 1 receiver")).toBeInTheDocument();
    expect(state.devicesRefetch).toHaveBeenCalled();
  });

  it("validates the manual address then trims it and sends an optional name as null", async () => {
    await mount();
    const dialog = await openDevice();
    await userEvent.type(dialog.getByRole("textbox", { name: "IP address" }), "  ");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    expect(await dialog.findByText("Enter the device address")).toBeInTheDocument();
    expect(state.add).not.toHaveBeenCalled();
    await edit("IP address", " 192.168.1.40 ");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(state.add).toHaveBeenCalledWith({ variables: { input: expect.objectContaining({ name: "", address: "192.168.1.40", port: 8009, deviceType: "CHROMECAST", isManual: true, isFavorite: false, enabled: true, discoveryOrigin: "MANUAL", playbackSupported: true }) } }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Add a receiver by address" })).not.toBeInTheDocument());
    expect(state.devicesRefetch).toHaveBeenCalled();
  });

  it("rejects a cleared port without adding a device", async () => {
    await mount();
    const dialog = await openDevice();
    await edit("IP address", "192.168.1.40");
    const port = await edit("Port", "");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    expect(await dialog.findByText("Enter a number")).toBeInTheDocument();
    await waitFor(() => expect(port).toHaveFocus());
    expect(state.add).not.toHaveBeenCalled();
  });

  it("retains a failed device draft, and clears it on cancel and reopen", async () => {
    state.add.mockResolvedValueOnce({ data: { createCastDevice: { success: false, error: "Device unreachable" } } });
    await mount();
    const dialog = await openDevice();
    await edit("IP address", "192.168.1.40");
    await edit("Name", "Living room");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(screen.getByRole("alert", { hidden: true })).toHaveTextContent("Device unreachable"));
    expect(dialog.getByRole("textbox", { name: "IP address" })).toHaveValue("192.168.1.40");
    await userEvent.click(dialog.getByRole("button", { name: "Cancel" }));
    const reopened = await openDevice();
    expect(reopened.getByRole("textbox", { name: "IP address" })).toHaveValue("");
    expect(reopened.getByRole("textbox", { name: "Name" })).toHaveValue("");
    expect(input("Port")).toHaveValue("8009");
  });
});
