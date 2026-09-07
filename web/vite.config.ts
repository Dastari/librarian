import { fileURLToPath, URL } from "node:url";

import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, loadEnv } from "vite";
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
  if (/node_modules\/(react-aria|react-aria-components|react-stately|@react-aria|@react-stately|@react-types|@internationalized|@heroui|@radix-ui|tailwind-variants|tailwind-merge)\//.test(id)) {
    return "vendor-ui";
  }
  if (/node_modules\/motion/.test(id) || /node_modules\/framer-motion/.test(id)) return "vendor-motion";
  if (/node_modules\/@tabler\/icons-react/.test(id)) return "vendor-icons";
  return "vendor";
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

  return {
    define: {
      __APP_VERSION__: JSON.stringify(process.env.npm_package_version ?? "0.0.0"),
      __BUILD_TIME__: JSON.stringify(new Date().toISOString()),
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
      VitePWA({
        registerType: "prompt",
        injectRegister: false,
        includeAssets: ["favicon.ico", "apple-touch-icon.png", "icons/*.png", "icons/*.svg"],
        manifest: {
          id: "/",
          name: "Librarian",
          short_name: "Librarian",
          description: "Your media library, beautifully organised.",
          start_url: "/",
          scope: "/",
          display: "standalone",
          display_override: ["window-controls-overlay", "standalone", "fullscreen"],
          orientation: "any",
          background_color: "#0a0a0f",
          theme_color: "#0a0a0f",
          categories: ["entertainment", "video", "music"],
          icons: [
            { src: "/icons/icon-192.png", sizes: "192x192", type: "image/png" },
            { src: "/icons/icon-512.png", sizes: "512x512", type: "image/png" },
            { src: "/icons/icon-maskable-192.png", sizes: "192x192", type: "image/png", purpose: "maskable" },
            { src: "/icons/icon-maskable-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
            { src: "/icons/icon.svg", sizes: "any", type: "image/svg+xml" },
          ],
          shortcuts: [
            { name: "Search", url: "/search", icons: [{ src: "/icons/shortcut-search.png", sizes: "192x192" }] },
            { name: "Downloads", url: "/downloads", icons: [{ src: "/icons/shortcut-downloads.png", sizes: "192x192" }] },
          ],
        },
        workbox: {
          globPatterns: ["**/*.{js,css,html,svg,png,ico,woff2}"],
          navigateFallback: "/index.html",
          navigateFallbackDenylist: [/^\/api\//, /^\/graphql/],
          maximumFileSizeToCacheInBytes: 4 * 1024 * 1024,
          runtimeCaching: [
            {
              // Artwork is immutable per entity/artwork key; cache it aggressively.
              urlPattern: /\/api\/artwork\//,
              handler: "CacheFirst",
              options: {
                cacheName: "artwork",
                expiration: { maxEntries: 800, maxAgeSeconds: 60 * 60 * 24 * 30 },
                cacheableResponse: { statuses: [0, 200] },
              },
            },
          ],
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
      environment: "jsdom",
      globals: false,
      setupFiles: ["./vitest.setup.ts"],
      include: ["src/**/*.test.{ts,tsx}"],
      css: false,
    },
  };
});
