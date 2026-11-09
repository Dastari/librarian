/**
 * This browser as a Librarian device (design.md Q133). It keeps a stable id of its own in local
 * storage and checks in with what it can tell about itself: a suggested name ("Safari on iPad",
 * "TV"), the platform, the browser and the app version. The server adds the address and user agent
 * it saw. Only the id is stored here; the device row and its name live on the server.
 */

const STORAGE_KEY = "librarian.device.v1";

export type FormFactor = "tv" | "phone" | "tablet" | "desktop" | "unknown";

export interface ClientFacts {
  product: string | null;
  platform: string | null;
  formFactor: FormFactor;
}

const TV_PLATFORMS: Array<[RegExp, string]> = [
  [/Web0S|webOS/, "webOS"],
  [/Tizen/, "Tizen"],
  [/\bAFT[A-Z0-9]*\b/, "Fire TV"],
  [/CrKey/, "Chromecast"],
  [/Android TV/, "Android TV"],
  [/GoogleTV/, "Google TV"],
  [/AppleTV/, "tvOS"],
  [/SMART-TV|SmartTV/, "Smart TV"],
  [/BRAVIA/, "Android TV"],
  [/PlayStation/, "PlayStation"],
  [/Xbox/, "Xbox"],
];

/** TV platforms worth naming; anything else in TV mode (a laptop on the television) is just "TV". */
const TV_NAMES = new Set(["webOS", "Tizen", "Fire TV", "Android TV", "Google TV"]);

/**
 * Browser, platform and form factor from a user agent. Edge and Opera also say Chrome, Chrome also
 * says Safari, and iPadOS says Macintosh unless the screen takes touch (`touchPoints`).
 */
export function describeClient(ua: string, touchPoints = 0): ClientFacts {
  const tv = TV_PLATFORMS.find(([pattern]) => pattern.test(ua))?.[1] ?? null;
  const ipadAsMac = /Macintosh/.test(ua) && touchPoints > 1;
  const platform = tv
    ?? (/iPad/.test(ua) || ipadAsMac ? "iPadOS"
      : /iPhone|iPod/.test(ua) ? "iOS"
        : /Android/.test(ua) ? "Android"
          : /CrOS/.test(ua) ? "ChromeOS"
            : /Mac OS X|Macintosh/.test(ua) ? "macOS"
              : /Windows/.test(ua) ? "Windows"
                : /Linux/.test(ua) ? "Linux" : null);
  const product = /Edg(A|iOS)?\//.test(ua) ? "Edge"
    : /OPR\/|Opera/.test(ua) ? "Opera"
      : /SamsungBrowser/.test(ua) ? "Samsung Internet"
        : /Firefox\/|FxiOS\//.test(ua) ? "Firefox"
          : /Chrome\/|CriOS\/|Chromium\//.test(ua) ? "Chrome"
            : /Safari\/|AppleWebKit\//.test(ua) ? "Safari" : null;
  const formFactor: FormFactor = tv ? "tv"
    : platform === "iPadOS" || (/Android/.test(ua) && !/Mobile/.test(ua)) || /Tablet/.test(ua) ? "tablet"
      : /Mobi|iPhone|iPod/.test(ua) ? "phone"
        : platform ? "desktop" : "unknown";
  return { product, platform, formFactor };
}

/** "Safari on iPad", "Chrome on Windows", "webOS TV", "TV". */
export function defaultDeviceName({ product, platform, formFactor }: ClientFacts): string {
  if (formFactor === "tv") return platform && TV_NAMES.has(platform) ? (platform.endsWith("TV") ? platform : `${platform} TV`) : "TV";
  const place = platform === "iPadOS" ? "iPad"
    : platform === "iOS" ? "iPhone"
      : platform === "macOS" ? "Mac"
        : platform ?? (formFactor === "phone" ? "phone" : formFactor === "tablet" ? "tablet" : "this device");
  if (product) return `${product} on ${place}`;
  return platform ? place : "Unknown device";
}

function randomId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") return crypto.randomUUID();
  const bytes = new Uint8Array(16);
  if (typeof crypto !== "undefined") crypto.getRandomValues(bytes);
  else for (let i = 0; i < bytes.length; i += 1) bytes[i] = Math.floor(Math.random() * 256);
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

const VALID_ID = /^[A-Za-z0-9_-]{1,128}$/;

/** This browser's device id, made once and kept. Falls back to a per-load id when storage is off. */
let memoryId: string | null = null;
export function clientDeviceId(): string {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    if (stored && VALID_ID.test(stored)) return stored;
    const id = randomId();
    window.localStorage.setItem(STORAGE_KEY, id);
    return id;
  } catch {
    memoryId ??= randomId();
    return memoryId;
  }
}

/** What this client says about itself when it checks in. TV mode makes it a TV. */
export function clientCheckIn(options: { tvMode: boolean; appVersion?: string }) {
  const facts = describeClient(navigator.userAgent, navigator.maxTouchPoints ?? 0);
  const resolved: ClientFacts = options.tvMode && facts.formFactor !== "tv" ? { ...facts, formFactor: "tv" } : facts;
  return {
    clientId: clientDeviceId(),
    name: defaultDeviceName(resolved),
    formFactor: resolved.formFactor,
    platform: resolved.platform,
    product: resolved.product,
    appVersion: options.appVersion ?? null,
  };
}

/** The device row this browser checked in as, for the player's sessions. */
let currentDeviceId: string | null = null;
const listeners = new Set<() => void>();
export const currentDevice = {
  get: () => currentDeviceId,
  set(id: string | null) {
    if (id === currentDeviceId) return;
    currentDeviceId = id;
    for (const listener of listeners) listener();
  },
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => { listeners.delete(listener); };
  },
};
