import { act, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useReducer } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { GeneralSettings } from "../GeneralSettings";

const settings = vi.hoisted(() => ({
  admin: true,
  playback: { values: new Map<string, string>(), loading: false, save: vi.fn() },
  metadata: { values: new Map<string, string>(), loading: false, save: vi.fn() },
  subtitles: { values: new Map<string, string>(), loading: false, save: vi.fn() },
  discovery: { values: new Map<string, string>(), loading: false, save: vi.fn() },
}));
vi.mock("@/lib/auth/useSession", () => ({ useIsAdmin: () => settings.admin }));
vi.mock("../useAppSettings", async (importOriginal) => ({
  ...await importOriginal<typeof import("../useAppSettings")>(),
  useAppSettings: (category: "playback" | "metadata" | "subtitles" | "discovery") => settings[category],
}));

let refresh: () => void;
function Harness() {
  const [, rerender] = useReducer((value: number) => value + 1, 0);
  refresh = rerender;
  return <GeneralSettings />;
}
const mount = () => renderWithProviders(<Harness />, { shell: true, router: false });
const edit = async (label: string, value: string) => {
  const input = screen.getAllByLabelText(label).find((node) => node.tagName === "INPUT")!;
  await userEvent.clear(input);
  if (value) await userEvent.type(input, value);
  await userEvent.tab();
  return input;
};
const language = () => screen.getByRole("textbox", { name: "Metadata language" });
const save = () => screen.getByRole("button", { name: "Save" });

beforeEach(() => {
  vi.clearAllMocks();
  settings.admin = true;
  settings.playback.loading = settings.metadata.loading = settings.subtitles.loading = settings.discovery.loading = false;
  settings.discovery.values = new Map([["discovery.advertise", "true"], ["discovery.web_url", ""]]);
  settings.playback.values = new Map([["playback_sync_interval", "30"]]);
  settings.metadata.values = new Map([["metadata.preferred_language", "en"]]);
  settings.subtitles.values = new Map([["subtitles.preferred_languages", '["en","de"]']]);
  for (const category of [settings.playback, settings.metadata, settings.subtitles, settings.discovery]) category.save.mockReset().mockResolvedValue(undefined);
  localStorage.clear();
});

describe("GeneralSettings bound form", () => {
  it("keeps stored defaults after an unrelated TV preference rerender", async () => {
    settings.metadata.values = new Map([["metadata.preferred_language", "de"]]);
    await mount();
    await userEvent.click(screen.getByRole("switch", { name: /TV mode/ }));
    expect(screen.getByRole("textbox", { name: "Save progress every" })).toHaveValue("30s");
    expect(language()).toHaveValue("de");
    expect(save()).toBeDisabled();
    expect(settings.metadata.save).not.toHaveBeenCalled();
  });

  it("hydrates stored values and restores the saved baseline on discard", async () => {
    await mount();
    expect(screen.getByRole("textbox", { name: "Save progress every" })).toHaveValue("30s");
    // Subtitle languages moved to Settings › Metadata, beside the subtitle provider.
    expect(screen.queryByLabelText("Preferred subtitle languages")).not.toBeInTheDocument();
    expect(save()).toBeDisabled();
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
    await edit("Metadata language", "fr");
    expect(save()).toBeEnabled();
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    expect(language()).toHaveValue("en");
    expect(save()).toBeDisabled();
    expect(settings.metadata.save).not.toHaveBeenCalled();
  });

  it("turns the video preview backfill off", async () => {
    await mount();
    const toggle = screen.getByRole("switch", { name: /Make previews for existing videos/ });
    expect(toggle).toBeChecked();
    await userEvent.click(toggle);
    await userEvent.click(save());
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(settings.playback.save).toHaveBeenCalledWith({ playback_sync_interval: "30", "playback.video_preview_backfill": false });
  });

  it("saves all categories using the existing string and JSON contracts, then adopts the baseline", async () => {
    await mount();
    await edit("Save progress every", "45");
    await edit("Metadata language", " fr ");
    await userEvent.click(save());
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(settings.playback.save).toHaveBeenCalledWith({ playback_sync_interval: "45", "playback.video_preview_backfill": true });
    expect(settings.metadata.save).toHaveBeenCalledWith({ "metadata.preferred_language": "fr" });
    await userEvent.click(screen.getByRole("switch", { name: /TV mode/ }));
    expect(screen.getByRole("textbox", { name: "Save progress every" })).toHaveValue("45s");
    expect(language()).toHaveValue("fr");
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
  });

  it("rejects an empty interval and focuses it without writing settings", async () => {
    await mount();
    const input = await edit("Save progress every", "");
    expect(input).toHaveValue("");
    await userEvent.click(save());
    expect(await screen.findByText("Enter a progress interval")).toBeInTheDocument();
    // Glass moves focus after validation paints on the next animation frame.
    await waitFor(() => expect(input).toHaveFocus());
    expect(settings.playback.save).not.toHaveBeenCalled();
  });

  it.each([
    ["Metadata language", "abcdefghijk"],
  ])("retains the existing length limit for %s", async (label, value) => {
    await mount();
    const input = screen.getAllByLabelText(label).find((node) => node.tagName === "INPUT")!;
    if (label === "Metadata language") await userEvent.clear(input);
    await userEvent.type(input, value);
    await userEvent.tab();
    await userEvent.click(save());
    await vi.waitFor(() => expect(input).toHaveAttribute("aria-invalid", "true"));
    expect(settings.playback.save).not.toHaveBeenCalled();
  });

  it("preserves a draft across live updates and discards to the latest snapshot", async () => {
    await mount();
    await edit("Metadata language", "fr");
    act(() => {
      settings.playback.values = new Map([["playback_sync_interval", "60"]]);
      settings.metadata.values = new Map([["metadata.preferred_language", "es"]]);
      refresh();
    });
    expect(language()).toHaveValue("fr");
    await userEvent.click(screen.getByRole("button", { name: "Discard" }));
    await userEvent.click(screen.getByRole("switch", { name: /TV mode/ }));
    expect(language()).toHaveValue("es");
    expect(screen.getByRole("textbox", { name: "Save progress every" })).toHaveValue("1m");
    expect(save()).toBeDisabled();
  });

  it("refreshes an untouched form when settings arrive", async () => {
    await mount();
    act(() => {
      settings.metadata.values = new Map([["metadata.preferred_language", "de"]]);
      refresh();
    });
    expect(language()).toHaveValue("de");
    expect(save()).toBeDisabled();
  });

  it("keeps the draft and reports a failed save", async () => {
    settings.metadata.save.mockRejectedValueOnce(new Error("Settings service unavailable"));
    await mount();
    await edit("Metadata language", "fr");
    await userEvent.click(save());
    expect(await screen.findByRole("alert")).toHaveTextContent("Settings service unavailable");
    expect(language()).toHaveValue("fr");
    expect(screen.getByRole("button", { name: "Discard" })).toBeEnabled();
    expect(settings.subtitles.save).not.toHaveBeenCalled();
    expect(screen.queryByText("Settings saved")).not.toBeInTheDocument();
  });

  it("keeps live snapshots from resetting the form during a save", async () => {
    let finish: () => void = () => { throw new Error("Save has not started"); };
    settings.metadata.save.mockImplementationOnce(() => new Promise<void>((resolve) => { finish = resolve; }));
    await mount();
    await edit("Metadata language", "fr");
    await userEvent.click(save());
    await vi.waitFor(() => expect(settings.metadata.save).toHaveBeenCalled());
    expect(screen.getByRole("button", { name: "Discard" })).toBeDisabled();
    act(() => {
      settings.metadata.values = new Map([["metadata.preferred_language", "es"]]);
      refresh();
    });
    expect(language()).toHaveValue("fr");
    await act(async () => finish());
    expect(await screen.findByText("Settings saved")).toBeInTheDocument();
    expect(language()).toHaveValue("fr");
  });

  it("retains immediate device preferences for non-admins without exposing server settings", async () => {
    settings.admin = false;
    await mount();
    expect(screen.getByText("This device")).toBeInTheDocument();
    expect(screen.queryByLabelText("Metadata language")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Save" })).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("switch", { name: /TV mode/ }));
    expect(localStorage.getItem("librarian.inputMode")).toBe("tv");
    expect(document.documentElement.dataset.inputMode).toBe("tv");
    expect(settings.playback.save).not.toHaveBeenCalled();
  });
});

describe("GeneralSettings network announcement", () => {
  it("saves the switch and the address for TV apps", async () => {
    await mount();
    await userEvent.click(screen.getByRole("switch", { name: /Announce on the local network/ }));
    await edit("Address for TV apps", "http://192.168.1.20:3001");
    await userEvent.click(save());
    await waitFor(() => expect(settings.discovery.save).toHaveBeenCalledWith({ "discovery.advertise": false, "discovery.web_url": '"http://192.168.1.20:3001"' }));
  });

  it("refuses an address without its scheme", async () => {
    await mount();
    await edit("Address for TV apps", "192.168.1.20:3001");
    await userEvent.click(save());
    expect(await screen.findByText(/Enter a full address/)).toBeInTheDocument();
    expect(settings.discovery.save).not.toHaveBeenCalled();
  });
});
