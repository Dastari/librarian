import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { sizedArtwork } from "@/lib/api/urls";
import { hlsBufferConfig } from "@/features/player/useMediaSource";
import { blurDefault, readBlur, readAnimatedBackgrounds, animatedBackgroundsUnavailable, BLUR_KEY } from "@/lib/glass/appearance";
import {
  TV_VOLUME_EVENT,
  bindTvVolume,
  deviceMemory,
  displaySizeStore,
  handleBack,
  holdScreenOn,
  installTvApp,
  isConstrainedDevice,
  isLowMemoryDevice,
  isTvApp,
  openMenu,
  quantizeVolume,
  readDisplaySize,
  resetTvVolumeForTests,
  setDisplaySize,
  tvAppVersion,
  tvVolumeStore,
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

  it("lets Home take Back first (clearing a search field) before the app exits", () => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "GoBack") event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    expect(handleBack(deps("/"))).toBe("handled");
    window.removeEventListener("keydown", onKey);
    expect(handleBack(deps("/"))).toBe("exit");
  });

  it("aims Escape at the topmost overlay", () => {
    document.body.innerHTML = '<div role="dialog" id="below"></div><div role="menu" id="top"></div>';
    const hits: string[] = [];
    for (const id of ["below", "top"]) document.getElementById(id)!.addEventListener("keydown", (event) => { if (event.key === "Escape" && event.currentTarget === event.target) hits.push(id); });
    expect(handleBack(deps("/movies/1"))).toBe("handled");
    expect(hits).toEqual(["top"]);
    document.body.innerHTML = "";
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

describe("display size", () => {
  it("is the shell's, and changing it goes to the shell", () => {
    expect(readDisplaySize()).toBe(100);
    const setShellSize = vi.fn();
    (window as Window & { LibrarianTV?: unknown }).LibrarianTV = { setKeepScreenOn: () => {}, version: () => "1.0.0", openServerSettings: () => {}, serverUrl: () => "", displaySize: () => 110, setDisplaySize: setShellSize };
    expect(readDisplaySize()).toBe(110);
    setDisplaySize(120);
    expect(setShellSize).toHaveBeenCalledWith(120);
    expect(displaySizeStore.getSnapshot()).toBe(120);
  });

  it("never touches the viewport meta itself", () => {
    document.head.innerHTML = '<meta name="viewport" content="width=1920, viewport-fit=cover">';
    setAgent(TV_AGENT);
    installTvApp({ pathname: () => "/" });
    setDisplaySize(90);
    expect(document.querySelector<HTMLMetaElement>('meta[name="viewport"]')!.content).toBe("width=1920, viewport-fit=cover");
  });
});

describe("the TV's volume", () => {
  let shell: { step: number; max: number; muted: boolean; fixed: boolean; canMute: boolean };
  const setVolume = vi.fn();
  const setMuted = vi.fn();

  beforeEach(() => {
    shell = { step: 6, max: 15, muted: false, fixed: false, canMute: true };
    setVolume.mockReset().mockImplementation((level: number) => { shell.step = Math.round(level * shell.max); });
    setMuted.mockReset().mockImplementation((muted: boolean) => {
      if (shell.canMute) shell.muted = muted;
      return shell.canMute;
    });
    (window as Window & { LibrarianTV?: unknown }).LibrarianTV = {
      setKeepScreenOn: () => {}, version: () => "1.0.0", openServerSettings: () => {}, serverUrl: () => "",
      volume: () => JSON.stringify({ ...shell, level: shell.step / shell.max }),
      setVolume,
      setMuted,
    };
    resetTvVolumeForTests();
  });
  afterEach(() => resetTvVolumeForTests());

  const tvEvent = (patch: Partial<typeof shell>) => {
    Object.assign(shell, patch);
    window.dispatchEvent(new CustomEvent(TV_VOLUME_EVENT, { detail: { ...shell, level: shell.step / shell.max } }));
  };

  it("is null in a browser", () => {
    delete (window as Window & { LibrarianTV?: unknown }).LibrarianTV;
    resetTvVolumeForTests();
    expect(tvVolumeStore.getSnapshot()).toBeNull();
  });

  it("rounds levels to the TV's steps without drift", () => {
    expect(quantizeVolume(0.37, 15)).toBeCloseTo(6 / 15);
    expect(quantizeVolume(1.3, 15)).toBe(1);
    for (let step = 0; step <= 15; step += 1) expect(quantizeVolume(step / 15, 15)).toBe(step / 15);
  });

  it("keeps the element at full volume while its volume and mute drive the TV", () => {
    const video = document.createElement("video");
    const changes = vi.fn();
    video.addEventListener("volumechange", changes);
    const unbind = bindTvVolume(video);
    expect(video.volume).toBeCloseTo(6 / 15);
    video.volume = 0.5;
    expect(setVolume).toHaveBeenCalledWith(8 / 15);
    expect(video.volume).toBeCloseTo(8 / 15);
    video.muted = true;
    expect(setMuted).toHaveBeenCalledWith(true);
    expect(video.muted).toBe(true);
    // The element itself never left full volume.
    const realVolume = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "volume")!.get!.call(video);
    const realMuted = Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "muted")!.get!.call(video);
    expect(realVolume).toBe(1);
    expect(realMuted).toBe(false);
    changes.mockClear();
    tvEvent({ step: 12, muted: false });
    expect(video.volume).toBeCloseTo(12 / 15);
    expect(video.muted).toBe(false);
    expect(changes).toHaveBeenCalled();
    unbind();
    expect(Object.getOwnPropertyDescriptor(video, "volume")).toBeUndefined();
  });

  it("mutes the element when the TV will not mute", () => {
    shell.canMute = false;
    const video = document.createElement("video");
    bindTvVolume(video);
    video.muted = true;
    expect(video.muted).toBe(true);
    expect(Object.getOwnPropertyDescriptor(HTMLMediaElement.prototype, "muted")!.get!.call(video)).toBe(true);
  });

  it("leaves a fixed volume alone", () => {
    shell.fixed = true;
    resetTvVolumeForTests();
    const video = document.createElement("video");
    bindTvVolume(video);
    expect(video.volume).toBe(1);
    video.volume = 0.2;
    expect(setVolume).not.toHaveBeenCalled();
  });
});
