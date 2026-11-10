import { screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { renderWithProviders } from "@/test";
import { FreeSpaceCell } from "../FreeSpaceCell";
import { DEFAULT_SPACE_THRESHOLDS, formatDiskBytes, spaceThresholds, volumeSpace, type VolumeReading } from "../volumeSpace";

const TB = 1e12;
const GB = 1e9;
const volume = (availableBytes: number, level = "normal", extra: Partial<VolumeReading> = {}): VolumeReading => ({ status: "ok", level, totalBytes: 8 * TB, availableBytes, label: "media on 10.0.1.190", ...extra });

describe("volumeSpace", () => {
  it("reads used against total, with free of total as disks are sold", () => {
    const space = volumeSpace(volume(1.2 * TB));
    expect(space.state).toBe("measured");
    if (space.state !== "measured") return;
    expect(space.valueText).toBe("1.2 TB free of 8 TB");
    expect(space.value).toBe(6.8 * TB);
    expect(space.max).toBe(8 * TB);
    expect(space.level).toBe("normal");
    // 8 TB: warning below 800 GB free (90 % used), critical below 400 GB (95 % used).
    expect(space.warningAt).toBeCloseTo(0.9);
    expect(space.dangerAt).toBeCloseTo(0.95);
    expect(space.description).toContain("(15%) on media on 10.0.1.190");
  });

  it("colours by the level the server settled, inside the hysteresis band too", () => {
    // 820 GB free is above the 800 GB line, but the server keeps the warning until 880 GB.
    const held = volumeSpace(volume(820 * GB, "warning"));
    if (held.state !== "measured") throw new Error("measured");
    expect(held.value / held.max).toBeGreaterThanOrEqual(held.warningAt);
    expect(held.value / held.max).toBeLessThan(held.dangerAt);
    const critical = volumeSpace(volume(410 * GB, "critical"));
    if (critical.state !== "measured") throw new Error("measured");
    expect(critical.value / critical.max).toBeGreaterThanOrEqual(critical.dangerAt);
    expect(critical.description).toContain("Critically low on space");
    // A normal level never colours, even past a threshold changed a moment ago.
    const normal = volumeSpace(volume(100 * GB, "normal"));
    if (normal.state !== "measured") throw new Error("measured");
    expect(normal.value / normal.max).toBeLessThan(normal.warningAt);
  });

  it("says unavailable, not 0, for a share that cannot be read", () => {
    const space = volumeSpace({ ...volume(0), status: "unavailable", detail: "The share is not connected" });
    expect(space).toMatchObject({ state: "unavailable", text: "Unavailable" });
    expect(space.description).toContain("The share is not connected");
    expect(volumeSpace(null).state).toBe("unknown");
    expect(volumeSpace({ status: "ok", level: "normal", totalBytes: null, availableBytes: null }).state).toBe("unknown");
  });

  it("formats sizes in powers of 1000 and reads the thresholds from settings", () => {
    expect(formatDiskBytes(48 * GB)).toBe("48 GB");
    expect(formatDiskBytes(8 * TB)).toBe("8 TB");
    expect(formatDiskBytes(512)).toBe("512 B");
    expect(spaceThresholds(new Map())).toEqual(DEFAULT_SPACE_THRESHOLDS);
    const custom = spaceThresholds(new Map([["storage.warning_free_fraction", "0.2"], ["storage.critical_free_fraction", "0.5"], ["storage.warning_free_bytes", "nonsense"]]));
    expect(custom).toMatchObject({ warningFraction: 0.2, criticalFraction: 0.2, warningBytes: 50 * GB });
  });
});

const levelOf = (element: Element) => element.closest("[data-level]")?.getAttribute("data-level");

describe("FreeSpaceCell", () => {
  it("shows the meter with its reading", async () => {
    await renderWithProviders(<FreeSpaceCell volume={volume(1.2 * TB)} />, { router: false });
    const meter = screen.getByRole("meter", { name: "Free space" });
    expect(meter).toHaveAttribute("aria-valuetext", "1.2 TB free of 8 TB");
    expect(levelOf(meter)).toBe("normal");
  });

  it("turns warning and danger with the level", async () => {
    const { unmount } = await renderWithProviders(<FreeSpaceCell volume={volume(700 * GB, "warning")} />, { router: false });
    expect(levelOf(screen.getByRole("meter"))).toBe("warning");
    unmount();
    await renderWithProviders(<FreeSpaceCell volume={volume(100 * GB, "critical")} />, { router: false });
    expect(levelOf(screen.getByRole("meter"))).toBe("danger");
  });

  it("shows Unavailable for an unreachable share and a placeholder before the first reading", async () => {
    const { unmount } = await renderWithProviders(<FreeSpaceCell volume={{ ...volume(0), status: "unavailable" }} />, { router: false });
    expect(screen.getByText("Unavailable")).toBeInTheDocument();
    expect(screen.queryByRole("meter")).toBeNull();
    unmount();
    await renderWithProviders(<FreeSpaceCell volume={null} />, { router: false });
    expect(screen.getByText("Not measured yet")).toBeInTheDocument();
  });
});
