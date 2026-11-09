import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { sizedArtwork } from "@/lib/api/urls";
import { hlsBufferConfig } from "@/features/player/useMediaSource";
import { blurDefault, readBlur, readAnimatedBackgrounds, animatedBackgroundsUnavailable, BLUR_KEY } from "@/lib/glass/appearance";
import {
  DISPLAY_SIZE_KEY,
  applyTvViewport,
  deviceMemory,
  handleBack,
  holdScreenOn,
  installTvApp,
  isConstrainedDevice,
  isLowMemoryDevice,
  isTvApp,
  openMenu,
  readDisplaySize,
  setDisplaySize,
  tvAppVersion,
  tvViewportContent,
} from "@/lib/tv-app";

const TV_AGENT = "Mozilla/5.0 (Linux; Android 11; V8-R51AT01-LF1V126 Build/RP1A; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/156.0.0.0 Safari/537.36 LibrarianTV/1.0.0";
const realAgent = navigator.userAgent;

function setAgent(agent: string) {
  Object.defineProperty(navigator, "userAgent", { configurable: true, get: () => agent });
}
function setMemory(memory: number | undefined) {
  Object.defineProperty(navigator, "deviceMemory", { configurable: true, get: () => memory });
}

afterEach(() => {
  setAgent(realAgent);
  setMemory(undefined);
  window.localStorage.clear();
  document.body.innerHTML = "";
  delete (window as Window & { LibrarianTV?: unknown }).LibrarianTV;
  delete (window as Window & { librarianTv?: unknown }).librarianTv;
  delete document.documentElement.dataset.tvApp;
});

describe("TV app detection", () => {
  it("reads the app and its version from the user agent", () => {
    expect(tvAppVersion(TV_AGENT)).toBe("1.0.0");
    expect(isTvApp(TV_AGENT)).toBe(true);
    expect(isTvApp(realAgent)).toBe(false);
    expect(isTvApp("Mozilla/5.0 (Linux; Android 11; BRAVIA 4K) Chrome/120")).toBe(false);
  });

  it("treats the TV app and 2 GB devices as constrained", () => {
    expect(isConstrainedDevice()).toBe(false);
    setMemory(4);
    expect(isLowMemoryDevice()).toBe(false);
    setMemory(2);
    expect(deviceMemory()).toBe(2);
    expect(isLowMemoryDevice()).toBe(true);
    expect(isConstrainedDevice()).toBe(true);
    setMemory(undefined);
    setAgent(TV_AGENT);
    expect(isConstrainedDevice()).toBe(true);
  });
});

describe("lighter defaults", () => {
  it("defaults blur off in the TV app, and still honours this device's choice", () => {
    expect(blurDefault()).toBe(true);
    setAgent(TV_AGENT);
    expect(blurDefault()).toBe(false);
    expect(readBlur(window.localStorage)).toBe(false);
    window.localStorage.setItem(BLUR_KEY, "on");
    expect(readBlur(window.localStorage)).toBe(true);
    window.localStorage.setItem(BLUR_KEY, "off");
    expect(readBlur(window.localStorage)).toBe(false);
  });

  it("keeps animated backgrounds off in the TV app and on 2 GB devices", () => {
    expect(readAnimatedBackgrounds()).toBe(true);
    setMemory(2);
    expect(readAnimatedBackgrounds()).toBe(false);
    expect(animatedBackgroundsUnavailable()).toMatch(/little memory/);
    setMemory(undefined);
    setAgent(TV_AGENT);
    expect(readAnimatedBackgrounds()).toBe(false);
    expect(animatedBackgroundsUnavailable()).toMatch(/TV app/);
  });

  it("fetches artwork at CSS size, capped, on constrained devices", () => {
    expect(sizedArtwork("/api/artwork/movie/1/poster", 200, true)).toBe("/api/artwork/movie/1/poster?w=200");
    expect(sizedArtwork("/api/artwork/movie/1/backdrop", undefined, true)).toBe("/api/artwork/movie/1/backdrop?w=1280");
    expect(sizedArtwork("/api/artwork/movie/1/backdrop", 1920, true)).toBe("/api/artwork/movie/1/backdrop?w=1280");
    expect(sizedArtwork("/api/artwork/movie/1/backdrop", undefined, false)).toBe("/api/artwork/movie/1/backdrop");
    expect(sizedArtwork("https://image.tmdb.org/x.jpg", 200, true)).toBe("https://image.tmdb.org/x.jpg");
  });

  it("caps hls.js buffers on constrained devices", () => {
    expect(hlsBufferConfig(true)).toEqual({ maxBufferLength: 30, maxMaxBufferLength: 60, backBufferLength: 10, maxBufferSize: 30_000_000 });
    expect(hlsBufferConfig(false)).toEqual({ maxBufferLength: 60, backBufferLength: 90 });
  });
});

describe("the remote's Back", () => {
  const deps = (path: string) => ({ pathname: () => path });

  it("closes an open dialog first", () => {
    document.body.innerHTML = '<div role="dialog"><button id="inside">x</button></div>';
    const inside = document.getElementById("inside")!;
    inside.focus();
    const keys: string[] = [];
    inside.addEventListener("keydown", (event) => keys.push(event.key));
    expect(handleBack(deps("/movies/1"))).toBe("handled");
    expect(keys).toEqual(["Escape"]);
  });

  it("leaves the app only from Home", () => {
    expect(handleBack(deps("/"))).toBe("exit");
  });

  it("goes back in the app when the page takes GoBack, and hands it back otherwise", () => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "GoBack") event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    expect(handleBack(deps("/movies/1"))).toBe("handled");
    window.removeEventListener("keydown", onKey);
    expect(handleBack(deps("/login"))).toBe("unhandled");
  });

  it("opens the focused item's context menu for Menu", () => {
    document.body.innerHTML = '<button id="tile">tile</button>';
    const tile = document.getElementById("tile")!;
    tile.focus();
    const menu = vi.fn((event: Event) => event.preventDefault());
    tile.addEventListener("contextmenu", menu);
    expect(openMenu()).toBe(true);
    expect(menu).toHaveBeenCalledOnce();
  });
});

describe("the shell bridge", () => {
  it("installs only in the TV app", () => {
    installTvApp({ pathname: () => "/" });
    expect((window as Window & { librarianTv?: unknown }).librarianTv).toBeUndefined();
    setAgent(TV_AGENT);
    installTvApp({ pathname: () => "/" });
    expect(document.documentElement).toHaveAttribute("data-tv-app");
    const api = (window as Window & { librarianTv?: { back: () => string } }).librarianTv;
    expect(api?.back()).toBe("exit");
  });

  it("asks the shell to keep the screen on while anything plays", () => {
    const setKeepScreenOn = vi.fn();
    (window as Window & { LibrarianTV?: unknown }).LibrarianTV = { setKeepScreenOn, version: () => "1.0.0", openServerSettings: () => {}, serverUrl: () => "" };
    const video = Symbol("video");
    const audio = Symbol("audio");
    holdScreenOn(video, true);
    holdScreenOn(audio, true);
    holdScreenOn(video, false);
    expect(setKeepScreenOn.mock.calls).toEqual([[true]]);
    holdScreenOn(audio, false);
    expect(setKeepScreenOn.mock.calls).toEqual([[true], [false]]);
  });
});

describe("the 10-foot canvas", () => {
  beforeEach(() => {
    document.head.innerHTML = '<meta name="viewport" content="width=device-width, initial-scale=1">';
  });

  it("lays out 1920 CSS px wide whatever density the TV reports", () => {
    // 1080p at density 2 (screen.width 960), 1080p at density 1, 4K at density 4.
    expect(tvViewportContent(100, 960)).toBe("width=1920, initial-scale=0.5, minimum-scale=0.5, maximum-scale=0.5, user-scalable=no");
    expect(tvViewportContent(100, 1920)).toBe("width=1920, initial-scale=1, minimum-scale=1, maximum-scale=1, user-scalable=no");
    expect(tvViewportContent(100, 0)).toBe("width=1920, user-scalable=no");
  });

  it("narrows the canvas for a larger display size", () => {
    expect(tvViewportContent(120, 960)).toBe("width=1600, initial-scale=0.6, minimum-scale=0.6, maximum-scale=0.6, user-scalable=no");
    expect(tvViewportContent(80, 960)).toBe("width=2400, initial-scale=0.4, minimum-scale=0.4, maximum-scale=0.4, user-scalable=no");
  });

  it("leaves the viewport alone when the shell's density already made the canvas", () => {
    Object.defineProperty(window.screen, "width", { configurable: true, get: () => 1920 });
    expect(applyTvViewport(100)).toBe(false);
    expect(document.querySelector<HTMLMetaElement>('meta[name="viewport"]')!.content).toBe("width=device-width, initial-scale=1");
    Object.defineProperty(window.screen, "width", { configurable: true, get: () => 1600 });
    expect(applyTvViewport(120)).toBe(false);
  });

  it("lets the shell keep the display size when it can", () => {
    const setShellSize = vi.fn();
    (window as Window & { LibrarianTV?: unknown }).LibrarianTV = { setKeepScreenOn: () => {}, version: () => "1.0.0", openServerSettings: () => {}, serverUrl: () => "", displaySize: () => 110, setDisplaySize: setShellSize };
    expect(readDisplaySize()).toBe(110);
    setDisplaySize(120);
    expect(setShellSize).toHaveBeenCalledWith(120);
    expect(window.localStorage.getItem(DISPLAY_SIZE_KEY)).toBeNull();
  });

  it("falls back to its own viewport and storage without the shell's density", () => {
    Object.defineProperty(window.screen, "width", { configurable: true, get: () => 960 });
    setAgent(TV_AGENT);
    expect(applyTvViewport()).toBe(true);
    const meta = () => document.querySelector<HTMLMetaElement>('meta[name="viewport"]')!.content;
    expect(meta()).toMatch(/^width=1920,/);
    setDisplaySize(110);
    expect(window.localStorage.getItem(DISPLAY_SIZE_KEY)).toBe("110");
    expect(meta()).toMatch(/^width=1745,/);
    expect(document.documentElement.style.getPropertyValue("--tv-display-size")).toBe("1.1");
    setDisplaySize(100);
    expect(window.localStorage.getItem(DISPLAY_SIZE_KEY)).toBeNull();
    window.localStorage.setItem(DISPLAY_SIZE_KEY, "300");
    expect(readDisplaySize()).toBe(100);
  });
});
