import base from "../vite.config";

/**
 * The sandbox's dev server (e2e/sandbox.sh): the app's own config with a dependency cache of its
 * own. Sharing `node_modules/.vite` with the live dev server on :3000 overwrote the live server's
 * pre-bundled dependencies and broke every page with two copies of React (2026-11-06).
 */
export default async (env: { command: "serve" | "build"; mode: string; isSsrBuild?: boolean; isPreview?: boolean }) => {
  const config = typeof base === "function" ? await base(env) : await base;
  return { ...config, cacheDir: "node_modules/.vite-sandbox" };
};
