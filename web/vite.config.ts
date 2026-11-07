import { execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath, URL } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { glassUx } from "glass-ux/vite";
import { defineConfig, loadEnv, type Plugin } from "vite";
import { VitePWA } from "vite-plugin-pwa";

/**
 * Vendor chunking keeps the first paint small: the 3D scene and the HLS engine only load
 * with the routes that use them.
 */
function vendorChunk(id: string): string | undefined {
  if (!id.includes("node_modules/")) return undefined;
  if (/node_modules\/(three|@react-three)\//.test(id)) return "vendor-three";
  if (/node_modules\/hls\.js\//.test(id)) return "vendor-hls";
  if (/node_modules\/(@apollo|graphql|graphql-ws|rxjs|@wry|optimism|zen-observable)/.test(id)) return "vendor-graphql";
  if (/node_modules\/(glass-ux|@base-ui|@floating-ui|clsx|tailwind-merge)\//.test(id)) {
    return "vendor-ui";
  }
  if (/node_modules\/motion/.test(id) || /node_modules\/framer-motion/.test(id)) return "vendor-motion";
  if (/node_modules\/@tabler\/icons-react/.test(id)) return "vendor-icons";
  return "vendor";
}

/** Cinema's dark canvas and brand amber, written by scripts/generate-pwa-assets.mjs. */
const pwaColors = JSON.parse(readFileSync(new URL("./src/assets/brand/pwa-colors.json", import.meta.url), "utf8")) as { brand: string; canvasDark: string };
const THEME_COLOR = pwaColors.canvasDark;
const BRAND_COLOR = pwaColors.brand;

/** A new id for every build: the package version, the commit when there is one, and the time. */
function buildId(version: string, time: string): string {
  let commit = "";
  try {
    commit = execSync("git rev-parse --short HEAD", { stdio: ["ignore", "pipe", "ignore"] }).toString().trim();
  } catch {
    // Not a git checkout (a release tarball): the timestamp alone is unique.
  }
  return [version, commit, time.replace(/[^0-9]/g, "").slice(0, 14)].filter(Boolean).join("-");
}

/**
 * Fills the head's placeholders and, in production builds, writes `version.json`, which open tabs
 * poll to notice a new deployment even where no service worker runs.
 */
function librarianHtml({ publicUrl, version, id, time }: { publicUrl: string; version: string; id: string; time: string }): Plugin {
  return {
    name: "librarian-html",
    transformIndexHtml: {
      order: "pre",
      handler(html) {
        // Without a known public origin og:url means nothing, and the image paths stay relative.
        const withUrl = publicUrl ? html : html.replace(/^\s*<meta property="og:url"[^>]*>\n/m, "");
        return withUrl
          .replaceAll("%LIBRARIAN_PUBLIC_URL%", publicUrl)
          .replaceAll("%LIBRARIAN_THEME_COLOR%", THEME_COLOR)
          .replaceAll("%LIBRARIAN_BRAND_COLOR%", BRAND_COLOR);
      },
    },
    generateBundle() {
      this.emitFile({ type: "asset", fileName: "version.json", source: `${JSON.stringify({ version, buildId: id, builtAt: time })}\n` });
    },
  };
}

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  const backendTarget = env.BACKEND_PROXY_TARGET || "http://127.0.0.1:3001";
  const publicDevUrl = env.DEV_SERVER_PUBLIC_URL ? new URL(env.DEV_SERVER_PUBLIC_URL) : undefined;
  // :3000 is the port the public site has always pointed at; screenshot runs pass their own.
  const port = Number(env.PORT || 3000);
  // The Host header is preserved on purpose: the backend's cookie origin guard
  // accepts requests whose Origin matches the Host it received.
  const proxy = {
    "/api": { target: backendTarget },
    "/graphql": { target: backendTarget, ws: true },
  };
  const allowedHosts = publicDevUrl ? [publicDevUrl.hostname] : undefined;
  // Absolute Open Graph links need the site's public origin; the dev server's serves the live site.
  const publicUrl = (env.LIBRARIAN_PUBLIC_URL || env.DEV_SERVER_PUBLIC_URL || "").replace(/\/+$/, "");
  const version = (JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as { version: string }).version;
  const builtAt = new Date().toISOString();
  const id = buildId(version, builtAt);

  return {
    define: {
      __APP_VERSION__: JSON.stringify(version),
      __BUILD_TIME__: JSON.stringify(builtAt),
      __BUILD_ID__: JSON.stringify(id),
    },
    server: {
      host: "0.0.0.0",
      port,
      strictPort: true,
      allowedHosts,
      hmr: publicDevUrl
        ? {
            protocol: publicDevUrl.protocol === "https:" ? "wss" : "ws",
            host: publicDevUrl.hostname,
            clientPort: Number(publicDevUrl.port || (publicDevUrl.protocol === "https:" ? 443 : 80)),
          }
        : undefined,
      proxy,
    },
    // `vite preview` serves the production build with the same proxy, for running the built app live.
    preview: { host: "0.0.0.0", port, strictPort: true, allowedHosts, proxy },
    plugins: [
      tanstackRouter({
        target: "react",
        autoCodeSplitting: true,
        routesDirectory: "./src/routes",
        generatedRouteTree: "./src/routeTree.gen.ts",
        quoteStyle: "double",
        semicolons: true,
      }),
      react(),
      tailwindcss(),
      glassUx(),
      librarianHtml({ publicUrl, version, id, time: builtAt }),
      // Production builds only: the dev server gets no worker, manifest link or caching.
      VitePWA({
        registerType: "prompt",
        injectRegister: false,
        includeAssets: ["favicon.ico", "favicon.svg", "apple-touch-icon.png"],
        includeManifestIcons: false,
        manifest: {
          id: "/",
          name: "Librarian",
          short_name: "Librarian",
          description: "Your media library, beautifully organised.",
          lang: "en",
          dir: "ltr",
          start_url: "/",
          scope: "/",
          display: "standalone",
          display_override: ["window-controls-overlay", "standalone"],
          orientation: "any",
          background_color: THEME_COLOR,
          theme_color: THEME_COLOR,
          categories: ["entertainment", "video", "music", "books"],
          prefer_related_applications: false,
          // "any" icons are rounded tiles (Android draws its launch splash from one, centred on
          // background_color); maskable ones are full bleed with the mark inside the safe zone.
          icons: [
            ...[48, 72, 96, 128, 144, 192, 256, 384, 512].map((size) => ({ src: `/icons/icon-${size}.png`, sizes: `${size}x${size}`, type: "image/png", purpose: "any" })),
            { src: "/favicon.svg", sizes: "any", type: "image/svg+xml", purpose: "any" },
            ...[192, 512].map((size) => ({ src: `/icons/icon-maskable-${size}.png`, sizes: `${size}x${size}`, type: "image/png", purpose: "maskable" })),
            ...[192, 512].map((size) => ({ src: `/icons/icon-monochrome-${size}.png`, sizes: `${size}x${size}`, type: "image/png", purpose: "monochrome" })),
          ],
          shortcuts: [
            { name: "Search", short_name: "Search", description: "Find something to watch, read or hear", url: "/search", icons: [{ src: "/icons/shortcut-search.png", sizes: "192x192", type: "image/png" }] },
            { name: "Downloads", short_name: "Downloads", description: "What is downloading now", url: "/downloads", icons: [{ src: "/icons/shortcut-downloads.png", sizes: "192x192", type: "image/png" }] },
          ],
        },
        workbox: {
          // The app shell only. Splash screens, the social card and the other icons load on demand.
          // (includeAssets adds the favicons and the iOS icon.)
          globPatterns: ["**/*.{js,css,html,woff2}", "icons/icon-{192,512}.png", "icons/favicon-*.png"],
          globIgnores: ["splash/**", "og-image.png", "version.json"],
          cleanupOutdatedCaches: true,
          navigateFallback: "/index.html",
          // No runtime caching: the API, GraphQL and its websocket, media streams and artwork
          // always go to the network. Offline gets the app shell, nothing more.
          navigateFallbackDenylist: [/^\/api(\/|$)/, /^\/graphql(\/|$)/, /^\/version\.json$/],
          maximumFileSizeToCacheInBytes: 4 * 1024 * 1024,
        },
        devOptions: { enabled: false },
      }),
    ],
    resolve: {
      alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
    },
    build: {
      target: "es2022",
      sourcemap: false,
      chunkSizeWarningLimit: 900,
      rollupOptions: { output: { manualChunks: vendorChunk } },
    },
    test: {
      server: { deps: { inline: [/glass-ux/] } },
      environment: "jsdom",
      // Dates in the UI are formatted in the local zone; pin it so snapshots of them are stable.
      env: { TZ: "UTC" },
      globals: false,
      setupFiles: ["./vitest.setup.ts", "./src/test/glass-setup.ts"],
      include: ["src/**/*.test.{ts,tsx}"],
      css: false,
      coverage: {
        provider: "v8",
        reporter: ["text-summary", "html"],
        reportsDirectory: "./coverage",
        include: ["src/**/*.{ts,tsx}"],
        exclude: [
          "src/**/*.test.{ts,tsx}",
          "src/test/**",
          "src/graphql/generated/**",
          "src/routeTree.gen.ts",
          "src/main.tsx",
          "src/vite-env.d.ts",
        ],
      },
    },
  };
});
