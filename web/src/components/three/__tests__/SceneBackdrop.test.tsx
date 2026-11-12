import { act, cleanup, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const ambientRendered = vi.fn();
vi.mock("@/components/three/AmbientScene", () => ({
  default: () => {
    ambientRendered();
    return <div data-testid="ambient-scene" />;
  },
}));
const constellationRendered = vi.fn();
vi.mock("@/components/constellation/FilmConstellation", () => ({
  FilmConstellation: () => {
    constellationRendered();
    return <div data-testid="film-constellation" />;
  },
}));

import { ConstellationBackdrop } from "@/components/constellation/ConstellationBackdrop";
import { SceneBackdrop } from "@/components/three/SceneBackdrop";
import { setAnimatedBackgrounds } from "@/lib/glass/appearance";

const realAgent = navigator.userAgent;
const TV_AGENT = "Mozilla/5.0 (Linux; Android 11; wv) Chrome/156.0.0.0 Safari/537.36 LibrarianTV/1.0.0";

function device({ agent = realAgent, memory, width = 1920, reduced = false }: { agent?: string; memory?: number; width?: number; reduced?: boolean }) {
  Object.defineProperty(navigator, "userAgent", { configurable: true, get: () => agent });
  Object.defineProperty(navigator, "deviceMemory", { configurable: true, get: () => memory });
  Object.defineProperty(window, "innerWidth", { configurable: true, get: () => width });
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      matches: reduced && query.includes("reduce"),
      media: query,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
    }),
  });
}

async function settle() {
  // Effects, then the lazy chunk.
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

beforeEach(() => {
  ambientRendered.mockClear();
  constellationRendered.mockClear();
  window.localStorage.clear();
});
afterEach(() => {
  cleanup();
  device({});
});

describe("SceneBackdrop", () => {
  it("runs the WebGL scene on a capable desktop", async () => {
    device({ memory: 8 });
    render(<SceneBackdrop />);
    expect(await screen.findByTestId("ambient-scene")).toBeInTheDocument();
  });

  it.each([
    ["the TV app", { agent: TV_AGENT, memory: 8 }],
    ["a 2 GB device", { memory: 2 }],
    ["reduced motion", { memory: 8, reduced: true }],
    ["a small screen", { memory: 8, width: 500 }],
  ])("never mounts it for %s", async (_name, options) => {
    device(options);
    const { container } = render(<SceneBackdrop />);
    await settle();
    expect(ambientRendered).not.toHaveBeenCalled();
    expect(screen.queryByTestId("ambient-scene")).toBeNull();
    // The static gradient is still there.
    expect(container.querySelector(".canvas-ambient")).not.toBeNull();
  });

  it("follows this device's switch", async () => {
    device({ memory: 8 });
    setAnimatedBackgrounds(false);
    render(<SceneBackdrop />);
    await settle();
    expect(ambientRendered).not.toHaveBeenCalled();
  });
});

describe("ConstellationBackdrop (sign-in)", () => {
  it("draws the constellation on phones and desktops", async () => {
    device({ memory: 8, width: 390 });
    render(<ConstellationBackdrop />);
    expect(await screen.findByTestId("film-constellation")).toBeInTheDocument();
  });

  it("keeps only the gradient in the TV app and on 2 GB devices", async () => {
    for (const options of [{ agent: TV_AGENT }, { memory: 2 }]) {
      device(options);
      const { container, unmount } = render(<ConstellationBackdrop />);
      await settle();
      expect(container.querySelector(".canvas-ambient")).not.toBeNull();
      unmount();
    }
    expect(constellationRendered).not.toHaveBeenCalled();
  });
});
