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
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => state.admin }));
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
          : document === docs.AddCastDeviceDocument ? state.add
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
const openDevice = async () => {
  await userEvent.click(screen.getByRole("button", { name: "Add manually" }));
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
  state.add.mockReset().mockResolvedValue({ data: { addCastDevice: { success: true, error: null } } });
  state.discover.mockReset().mockResolvedValue({ data: { discoverCastDevices: [{ id: "found-1" }] } });
});

describe("CastingSettings bound forms", () => {
  it("keeps saved values across unrelated rerenders and restores them after discard", async () => {
    await mount();
    expect(input("Preferred quality")).toHaveValue("720p");
    expect(input("Discovery interval (seconds)")).toHaveValue("600");
    expect(screen.getByRole("switch", { name: "Discover devices automatically" })).not.toBeChecked();
    act(() => refresh());
    expect(input("Preferred quality")).toHaveValue("720p");
    expect(save()).toBeDisabled();
    await edit("Preferred quality", "1080p");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(input("Preferred quality")).toHaveValue("720p");
    expect(save()).toBeDisabled();
  });

  it("preserves a draft across live settings changes and discards to the latest server baseline", async () => {
    await mount();
    await edit("Preferred quality", "1080p");
    act(() => {
      state.setting = { ...state.setting!, preferredQuality: "2160p", discoveryIntervalSeconds: 900 };
      refresh();
    });
    expect(input("Preferred quality")).toHaveValue("1080p");
    expect(input("Discovery interval (seconds)")).toHaveValue("900");
    act(() => refresh());
    expect(input("Preferred quality")).toHaveValue("1080p");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(input("Preferred quality")).toHaveValue("2160p");
    expect(input("Discovery interval (seconds)")).toHaveValue("900");
    expect(save()).toBeDisabled();
  });

  it("updates the existing record, preserves zero volume and sends empty quality as null", async () => {
    await mount();
    await edit("Default volume (0–1)", "0");
    await edit("Preferred quality", "  ");
    await userEvent.click(save());
    await screen.findByText("Settings saved");
    expect(state.update).toHaveBeenCalledWith({ variables: { id: "cast-settings", input: { autoDiscoveryEnabled: false, discoveryIntervalSeconds: 600, defaultVolume: 0, transcodeIncompatible: false, preferredQuality: null } } });
    expect(state.create).not.toHaveBeenCalled();
    expect(state.settingsRefetch).toHaveBeenCalled();
    act(() => refresh());
    expect(input("Preferred quality")).toHaveValue("");
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
  });

  it("creates settings when there is no stored record", async () => {
    state.setting = null;
    await mount();
    await edit("Preferred quality", " 1080p ");
    await userEvent.click(save());
    await waitFor(() => expect(state.create).toHaveBeenCalledWith({ variables: { input: { autoDiscoveryEnabled: true, discoveryIntervalSeconds: 300, defaultVolume: 0.8, transcodeIncompatible: true, preferredQuality: "1080p" } } }));
    expect(state.update).not.toHaveBeenCalled();
  });

  it("rejects an empty numeric value and focuses the field without mutation", async () => {
    await mount();
    const interval = await edit("Discovery interval (seconds)", "");
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
    await edit("Preferred quality", "1080p");
    await userEvent.click(save());
    await waitFor(() => expect(state.update).toHaveBeenCalled());
    expect(input("Preferred quality")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
    act(() => {
      state.setting = { ...state.setting!, preferredQuality: "2160p" };
      refresh();
    });
    expect(input("Preferred quality")).toHaveValue("1080p");
    expect(input("Preferred quality")).toBeDisabled();
    await act(async () => reject(new Error("Cast settings unavailable")));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Cast settings unavailable"));
    expect(input("Preferred quality")).toHaveValue("1080p");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(input("Preferred quality")).toHaveValue("2160p");
  });

  it("keeps admin settings and manual additions gated while discovery remains available", async () => {
    state.admin = false;
    await mount();
    expect(screen.queryByRole("button", { name: "Save" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Add manually" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Discover" }));
    await waitFor(() => expect(state.discover).toHaveBeenCalled());
    expect(state.devicesRefetch).toHaveBeenCalled();
    expect(state.update).not.toHaveBeenCalled();
  });

  it("validates the manual address then trims it and sends an optional name as null", async () => {
    await mount();
    const dialog = await openDevice();
    await userEvent.type(dialog.getByRole("textbox", { name: "IP address or hostname" }), "  ");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    expect(await dialog.findByText("Enter the device address")).toBeInTheDocument();
    expect(state.add).not.toHaveBeenCalled();
    await edit("IP address or hostname", " receiver.local ");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(state.add).toHaveBeenCalledWith({ variables: { input: { address: "receiver.local", port: 8009, name: null } } }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Add cast device" })).not.toBeInTheDocument());
    expect(state.devicesRefetch).toHaveBeenCalled();
  });

  it("rejects a cleared port without adding a device", async () => {
    await mount();
    const dialog = await openDevice();
    await edit("IP address or hostname", "receiver.local");
    const port = await edit("Port", "");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    expect(await dialog.findByText("Enter a number")).toBeInTheDocument();
    await waitFor(() => expect(port).toHaveFocus());
    expect(state.add).not.toHaveBeenCalled();
  });

  it("retains a failed device draft, and clears it on cancel and reopen", async () => {
    state.add.mockResolvedValueOnce({ data: { addCastDevice: { success: false, error: "Device unreachable" } } });
    await mount();
    const dialog = await openDevice();
    await edit("IP address or hostname", "receiver.local");
    await edit("Name", "Living room");
    await userEvent.click(dialog.getByRole("button", { name: "Add" }));
    await waitFor(() => expect(screen.getByRole("alert", { hidden: true })).toHaveTextContent("Device unreachable"));
    expect(dialog.getByRole("textbox", { name: "IP address or hostname" })).toHaveValue("receiver.local");
    await userEvent.click(dialog.getByRole("button", { name: "Cancel" }));
    const reopened = await openDevice();
    expect(reopened.getByRole("textbox", { name: "IP address or hostname" })).toHaveValue("");
    expect(reopened.getByRole("textbox", { name: "Name" })).toHaveValue("");
    expect(input("Port")).toHaveValue("8,009");
  });
});
