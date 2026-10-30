import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useReducer } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";
import { OrganizationSettings } from "../OrganizationSettings";

const state = vi.hoisted(() => ({
  values: new Map<string, string>(),
  save: vi.fn(),
  mutate: vi.fn(),
  refetch: vi.fn(),
}));
vi.mock("../useAppSettings", async (original) => ({
  ...await original<typeof import("../useAppSettings")>(),
  useAppSettings: () => ({ values: state.values, save: state.save }),
}));
vi.mock("../useSettingsConnection", () => ({
  useSettingsConnection: () => ({ rows: [], loading: false, error: undefined, refetch: state.refetch }),
}));
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "user-1" } }) }));
vi.mock("@apollo/client/react", async (original) => ({
  ...await original<typeof import("@apollo/client/react")>(),
  useMutation: () => [state.mutate, { loading: false }],
}));

let refresh: () => void;
function Harness() {
  const [, rerender] = useReducer((value: number) => value + 1, 0);
  refresh = rerender;
  return <OrganizationSettings />;
}
const mount = () => renderWithProviders(<Harness />, { router: false });
const save = () => screen.getByRole("button", { name: "Save" });
const automatic = () => screen.getByRole("switch", { name: /Organize imported files/ });
const emptyFolders = () => screen.getByRole("switch", { name: /Remove empty folders/ });

beforeEach(() => {
  vi.clearAllMocks();
  state.values = new Map([["organize.auto_organize", "true"], ["organize.delete_empty_folders", "false"], ["organize.copy_mode", "copy"]]);
  state.save.mockReset().mockResolvedValue(undefined);
  state.mutate.mockReset().mockResolvedValue({ data: { createNamingPattern: { success: true, error: null } } });
});

describe("OrganizationSettings bound forms", () => {
  it("keeps stored untouched values across unrelated rerenders, and discards edits", async () => {
    await mount();
    expect(automatic()).toBeChecked();
    expect(emptyFolders()).not.toBeChecked();
    expect(screen.getByRole("combobox", { name: "Import method" })).toHaveTextContent("Always copy");
    act(() => refresh());
    expect(automatic()).toBeChecked();
    expect(emptyFolders()).not.toBeChecked();
    expect(save()).toBeDisabled();
    await userEvent.click(automatic());
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(automatic()).toBeChecked();
    expect(save()).toBeDisabled();
    expect(state.save).not.toHaveBeenCalled();
  });

  it("preserves dirty fields while adopting live values and the latest discard baseline", async () => {
    await mount();
    await userEvent.click(automatic());
    act(() => {
      state.values = new Map([["organize.auto_organize", "true"], ["organize.delete_empty_folders", "true"], ["organize.copy_mode", "move"]]);
      refresh();
    });
    expect(automatic()).not.toBeChecked();
    expect(emptyFolders()).toBeChecked();
    expect(screen.getByRole("combobox", { name: "Import method" })).toHaveTextContent("Move");
    act(() => refresh());
    expect(automatic()).not.toBeChecked();
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(automatic()).toBeChecked();
    expect(emptyFolders()).toBeChecked();
    expect(save()).toBeDisabled();
  });

  it("disables discard while saving, sends existing setting keys and adopts the saved baseline", async () => {
    let resolve!: () => void;
    state.save.mockImplementationOnce(() => new Promise<void>((done) => { resolve = done; }));
    await mount();
    await userEvent.click(automatic());
    await userEvent.click(save());
    await waitFor(() => expect(state.save).toHaveBeenCalledWith({ "organize.auto_organize": false, "organize.delete_empty_folders": false, "organize.copy_mode": "copy" }));
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
    await act(async () => resolve());
    await screen.findByText("Settings saved");
    act(() => refresh());
    expect(automatic()).not.toBeChecked();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
  });

  it("reports failure and retains a retryable draft", async () => {
    state.save.mockRejectedValueOnce(new Error("Storage unavailable"));
    await mount();
    await userEvent.click(automatic());
    await userEvent.click(save());
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Storage unavailable"));
    expect(automatic()).not.toBeChecked();
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(automatic()).toBeChecked();
  });

  it("validates the naming dialog then trims names and preserves optional empty descriptions", async () => {
    await mount();
    await userEvent.click(screen.getByRole("button", { name: /New pattern/ }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByRole("textbox", { name: "Name" }), "   ");
    await userEvent.click(dialog.getByRole("button", { name: "Save" }));
    expect(await dialog.findByText("Name the pattern")).toBeInTheDocument();
    expect(state.mutate).not.toHaveBeenCalled();
    const name = dialog.getByRole("textbox", { name: "Name" });
    await userEvent.clear(name);
    await userEvent.type(name, " My pattern ");
    await userEvent.click(dialog.getByRole("textbox", { name: "Pattern" }));
    await userEvent.paste(" {title} ");
    await userEvent.click(dialog.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(state.mutate).toHaveBeenCalledWith({ variables: { input: { name: "My pattern", libraryType: "movies", pattern: "{title}", description: null, isDefault: false, userId: "user-1", isSystem: false } } }));
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "New naming pattern" })).not.toBeInTheDocument());
    expect(state.refetch).toHaveBeenCalled();
  });

  it("keeps failed pattern edits available, and resets them when opening a new dialog", async () => {
    state.mutate.mockResolvedValueOnce({ data: { createNamingPattern: { success: false, error: "Pattern rejected" } } });
    await mount();
    await userEvent.click(screen.getByRole("button", { name: /New pattern/ }));
    const dialog = within(await screen.findByRole("dialog"));
    await userEvent.type(dialog.getByRole("textbox", { name: "Name" }), "Draft");
    await userEvent.click(dialog.getByRole("textbox", { name: "Pattern" }));
    await userEvent.paste("{title}");
    await userEvent.click(dialog.getByRole("button", { name: "Save" }));
    await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("Pattern rejected"));
    expect(dialog.getByRole("textbox", { name: "Name" })).toHaveValue("Draft");
    await userEvent.click(dialog.getByRole("button", { name: "Cancel" }));
    await userEvent.click(screen.getByRole("button", { name: /New pattern/ }));
    expect(within(await screen.findByRole("dialog")).getByRole("textbox", { name: "Name" })).toHaveValue("");
  });
});
