import { act, fireEvent, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { PlayPauseButton, SeekBar, SkipButton, VolumeControl } from "../controls";

function VolumeFixture() {
  const [volume, setVolume] = useState(0.5);
  const [muted, setMuted] = useState(false);
  return <VolumeControl volume={volume} muted={muted} onMuted={setMuted} onVolume={(value) => { setVolume(value); setMuted(value === 0); }} />;
}

describe("Glass player controls", () => {
  afterEach(() => vi.restoreAllMocks());

  it("keeps transport names and keyboard activation and refuses an unavailable skip", async () => {
    const toggle = vi.fn(), next = vi.fn(), previous = vi.fn();
    await renderWithProviders(<><PlayPauseButton playing={false} onToggle={toggle} /><SkipButton direction="back" onPress={previous} /><SkipButton direction="forward" onPress={next} disabled /></>, { router: false });
    screen.getByRole("button", { name: "Play" }).focus();
    await userEvent.keyboard(" ");
    expect(toggle).toHaveBeenCalledTimes(1);
    screen.getByRole("button", { name: "Previous" }).focus();
    await userEvent.keyboard("{Enter}");
    expect(previous).toHaveBeenCalledTimes(1);
    const unavailable = screen.getByRole("button", { name: "Next" });
    expect(unavailable).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(unavailable);
    expect(next).not.toHaveBeenCalled();
  });

  it("steps volume by two percent, supports endpoints, and retains the pre-mute volume", async () => {
    // Base UI reveals its accessible thumb after measuring a nonzero track. jsdom
    // has no layout; provide the track/thumb geometry instead of querying hidden UI.
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
      const thumb = this.classList.contains("fa-slider-thumb");
      const width = thumb ? 16 : 96;
      const height = thumb ? 16 : 24;
      return { width, height, x: 0, y: 0, top: 0, left: 0, right: width, bottom: height, toJSON: () => ({}) };
    });
    await renderWithProviders(<VolumeFixture />, { router: false });
    const slider = await screen.findByRole("slider", { name: "Volume" });
    slider.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(slider).toHaveAttribute("aria-valuenow", "0.52");
    expect(slider).toHaveAttribute("aria-valuetext", "52%");
    await userEvent.click(screen.getByRole("button", { name: "Mute" }));
    expect(slider).toHaveAttribute("aria-valuenow", "0");
    await userEvent.click(screen.getByRole("button", { name: "Unmute" }));
    expect(slider).toHaveAttribute("aria-valuenow", "0.52");
    slider.focus();
    await userEvent.keyboard("{End}");
    expect(slider).toHaveAttribute("aria-valuenow", "1");
    await userEvent.keyboard("{Home}");
    expect(slider).toHaveAttribute("aria-valuenow", "0");
  });

  it("puts a scrub back on Back, and seeks after a pause otherwise", async () => {
    vi.useFakeTimers();
    try {
      const seek = vi.fn();
      await renderWithProviders(<SeekBar position={60} duration={180} onSeek={seek} labelAt={(at) => (at >= 60 ? "Chapter three" : "Chapter two")} />, { router: false });
      const slider = screen.getByRole("slider", { name: "Seek" });
      fireEvent.keyDown(slider, { key: "ArrowLeft" });
      expect(screen.getByText("Chapter two · 0:50")).toBeInTheDocument();
      const back = fireEvent.keyDown(slider, { key: "Escape" });
      expect(back).toBe(false); // consumed: the page does not go back
      act(() => { vi.advanceTimersByTime(2000); });
      expect(seek).not.toHaveBeenCalled();
      fireEvent.keyDown(slider, { key: "ArrowRight" });
      act(() => { vi.advanceTimersByTime(800); });
      expect(seek.mock.calls).toEqual([[70]]);
    } finally {
      vi.useRealTimers();
    }
  });

  it("scrubs with the arrows, seeking once on OK, and keeps Home and End", async () => {
    const seek = vi.fn();
    await renderWithProviders(<SeekBar position={60} duration={180} buffered={120} chapters={[{ start: 30, title: "Chapter two" }]} onSeek={seek} />, { router: false });
    const slider = screen.getByRole("slider", { name: "Seek" });
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    fireEvent.keyDown(slider, { key: "ArrowRight" });
    // The preview names the target and how far it is; nothing has seeked yet.
    expect(screen.getByText("Chapter two · 1:20")).toBeInTheDocument();
    expect(screen.getByText("+0:20")).toBeInTheDocument();
    expect(seek).not.toHaveBeenCalled();
    fireEvent.keyDown(slider, { key: "Enter" });
    expect(seek.mock.calls).toEqual([[80]]);
    fireEvent.keyDown(slider, { key: "Home" });
    fireEvent.keyDown(slider, { key: "End" });
    expect(seek.mock.calls).toEqual([[80], [0], [180]]);
    seek.mockClear();
    slider.getBoundingClientRect = () => ({ width: 180, height: 24, x: 0, y: 0, top: 0, left: 0, right: 180, bottom: 24, toJSON: () => ({}) });
    fireEvent.pointerDown(slider, { clientX: 45, pointerId: 1, pointerType: "mouse" });
    fireEvent.pointerMove(slider, { clientX: 90, pointerId: 1, pointerType: "mouse" });
    expect(screen.getByText("Chapter two · 1:30")).toBeInTheDocument();
    expect(seek).not.toHaveBeenCalled();
    fireEvent.pointerUp(slider, { clientX: 90, pointerId: 1, pointerType: "mouse" });
    expect(seek.mock.calls).toEqual([[90]]);
  });
});
