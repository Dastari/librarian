import { act, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resetTvVolumeForTests, TV_VOLUME_EVENT } from "@/lib/tv-app";
import { renderWithProviders } from "@/test";

import { VolumeControl } from "../controls";

const shell = { step: 6, max: 15, muted: false, fixed: false };
const setVolume = vi.fn((level: number) => { shell.step = Math.round(level * shell.max); });
const setMuted = vi.fn((muted: boolean) => { shell.muted = muted; return true; });

function installShell() {
  (window as Window & { LibrarianTV?: unknown }).LibrarianTV = {
    setKeepScreenOn: () => {}, version: () => "1.0.0", openServerSettings: () => {}, serverUrl: () => "",
    volume: () => JSON.stringify({ ...shell, level: shell.step / shell.max }),
    setVolume,
    setMuted,
  };
  resetTvVolumeForTests();
}

function remoteChanges(patch: Partial<typeof shell>) {
  Object.assign(shell, patch);
  act(() => {
    window.dispatchEvent(new CustomEvent(TV_VOLUME_EVENT, { detail: { ...shell, level: shell.step / shell.max } }));
  });
}

const onVolume = vi.fn();
const onMuted = vi.fn();
const slider = () => screen.getByRole("slider", { name: "Volume" });

beforeEach(() => {
  Object.assign(shell, { step: 6, max: 15, muted: false, fixed: false });
  vi.clearAllMocks();
  // jsdom has no layout; the Glass slider needs its track and thumb geometry (as in controls.test).
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    const thumb = this.classList.contains("fa-slider-thumb");
    const width = thumb ? 16 : 96;
    const height = thumb ? 16 : 24;
    return { width, height, x: 0, y: 0, top: 0, left: 0, right: width, bottom: height, toJSON: () => ({}) } as DOMRect;
  });
});
afterEach(() => {
  vi.restoreAllMocks();
  delete (window as Window & { LibrarianTV?: unknown }).LibrarianTV;
  resetTvVolumeForTests();
});

describe("the player's volume in the TV app", () => {
  it("shows the TV's volume and follows the remote live", async () => {
    installShell();
    await renderWithProviders(<VolumeControl volume={1} muted={false} onVolume={onVolume} onMuted={onMuted} />, { router: false });
    await screen.findByRole("slider", { name: "Volume" });
    expect(Number(slider().getAttribute("aria-valuenow"))).toBeCloseTo(6 / 15);
    remoteChanges({ step: 11 });
    expect(Number(slider().getAttribute("aria-valuenow"))).toBeCloseTo(11 / 15);
    remoteChanges({ muted: true });
    expect(screen.getByRole("button", { name: "Unmute" })).toBeInTheDocument();
  });

  it("sets the TV's volume a step at a time and mutes the TV, not the player", async () => {
    installShell();
    await renderWithProviders(<VolumeControl volume={1} muted={false} onVolume={onVolume} onMuted={onMuted} />, { router: false });
    (await screen.findByRole("slider", { name: "Volume" })).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(setVolume).toHaveBeenCalledWith(7 / 15);
    expect(onVolume).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Mute" }));
    expect(setMuted).toHaveBeenCalledWith(true);
    expect(onMuted).not.toHaveBeenCalled();
  });

  it("shows a note instead of the slider when the TV's volume is fixed", async () => {
    shell.fixed = true;
    installShell();
    await renderWithProviders(<VolumeControl volume={1} muted={false} onVolume={onVolume} onMuted={onMuted} />, { router: false });
    expect(await screen.findByText("Volume is controlled by your TV or sound system")).toBeInTheDocument();
    expect(screen.queryByRole("slider", { name: "Volume" })).toBeNull();
    expect(screen.getByText("Volume is controlled by your TV or sound system")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Mute" })).toBeInTheDocument();
  });

  it("is the player's own volume in a browser", async () => {
    await renderWithProviders(<VolumeControl volume={0.5} muted={false} onVolume={onVolume} onMuted={onMuted} />, { router: false });
    await screen.findByRole("slider", { name: "Volume" });
    expect(Number(slider().getAttribute("aria-valuenow"))).toBeCloseTo(0.5);
    slider().focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onVolume).toHaveBeenCalled();
    expect(setVolume).not.toHaveBeenCalled();
  });
});
