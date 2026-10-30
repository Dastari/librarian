#!/usr/bin/env node
// Serialize entity generation and type generation. Watching the CLI alone skips
// entity-document regeneration after an SDL change.
import { watch } from "node:fs";
import { spawn } from "node:child_process";

let child;
let dirty = false;
let stopped = false;
let timer;
async function run() {
  if (stopped) return;
  if (child) { dirty = true; return; }
  dirty = false;
  child = spawn("pnpm", ["run", "codegen"], { stdio: "inherit" });
  await new Promise((resolve) => { child.once("exit", resolve); child.once("error", (error) => { console.error(error); resolve(); }); });
  child = undefined;
  if (dirty) void run();
}
function changed() { clearTimeout(timer); timer = setTimeout(() => void run(), 150); }
const watchers = ["schema.graphql", "codegen.ts", "src/graphql/documents"].map((path) => watch(path, { recursive: path.endsWith("documents") }, changed));
function stop() {
  stopped = true; clearTimeout(timer); watchers.forEach((watcher) => watcher.close()); child?.kill("SIGTERM");
}
process.on("SIGINT", stop); process.on("SIGTERM", stop);
void run();
