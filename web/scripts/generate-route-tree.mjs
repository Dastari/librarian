#!/usr/bin/env node
/**
 * Writes src/routeTree.gen.ts from the file routes, using the same options as the Vite
 * plugin in vite.config.ts. The Vite plugin does this during `dev` and `build`; this script
 * exists so type-checking and tests work on a fresh checkout (CI) where the file is absent.
 */
import { Generator, getConfig } from "@tanstack/router-generator";

const root = process.cwd();
const config = getConfig(
  {
    target: "react",
    autoCodeSplitting: true,
    routesDirectory: "./src/routes",
    generatedRouteTree: "./src/routeTree.gen.ts",
    quoteStyle: "double",
    semicolons: true,
  },
  root,
);
await new Generator({ config, root }).run();
console.log(`Route tree written to ${config.generatedRouteTree}`);
