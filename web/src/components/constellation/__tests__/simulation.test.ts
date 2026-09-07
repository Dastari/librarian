// @vitest-environment node
import { describe, expect, it } from "vitest";

import type { Adjacency, FilmGraph } from "../graph";
import { Constellation, type SimNode } from "../simulation";

/** Deterministic PRNG so growth, retirement and layout are reproducible. */
function seeded(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** A small, densely connected dataset: 12 titles, 8 people, every person in every third title. */
function dataset(): Adjacency {
  const graph: FilmGraph = {
    generatedAt: "2026-01-01",
    source: "test",
    titles: Array.from({ length: 12 }, (_, index) => [`Title ${index}`, 1990 + index, (index % 2) as 0 | 1, index % 3 === 0 ? "" : `en:0/0${index % 10}:250:T${index}.jpg`]),
    people: Array.from({ length: 8 }, (_, index) => [`Person ${index}`, `commons:1/1${index}:250:P${index}.jpg`]),
    edges: [],
  };
  for (let title = 0; title < graph.titles.length; title += 1) {
    for (let person = 0; person < graph.people.length; person += 1) {
      if ((title + person) % 2 === 0) graph.edges.push([title, person]);
    }
  }
  const cast: number[][] = graph.titles.map(() => []);
  const credits: number[][] = graph.people.map(() => []);
  for (const [title, person] of graph.edges) {
    cast[title]!.push(person);
    credits[person]!.push(title);
  }
  return { graph, cast, credits };
}

const live = (constellation: Constellation): SimNode[] => constellation.nodes.filter((node) => node.died === null);
const run = (constellation: Constellation, seconds: number, dt = 1 / 30) => {
  for (let elapsed = 0; elapsed < seconds; elapsed += dt) constellation.step(dt);
};

/** The same superellipse the simulation uses for its keep-out zone. */
const clearDistance = (node: SimNode, size: { width: number; height: number }, clear: { rx: number; ry: number }) =>
  Math.pow(Math.pow(Math.abs(node.x - size.width / 2) / clear.rx, 4) + Math.pow(Math.abs(node.y - size.height / 2) / clear.ry, 4), 0.25);

describe("constellation simulation", () => {
  it("is fully determined by the injected random source", () => {
    const a = new Constellation(dataset(), { width: 1200, height: 800, random: seeded(7) });
    const b = new Constellation(dataset(), { width: 1200, height: 800, random: seeded(7) });
    run(a, 20);
    run(b, 20);
    expect(a.nodes.map((node) => node.key)).toEqual(b.nodes.map((node) => node.key));
    expect(a.nodes.map((node) => [node.x, node.y])).toEqual(b.nodes.map((node) => [node.x, node.y]));
    expect(a.edges.length).toBe(b.edges.length);

    const c = new Constellation(dataset(), { width: 1200, height: 800, random: seeded(99) });
    run(c, 20);
    expect(c.nodes.map((node) => node.key)).not.toEqual(a.nodes.map((node) => node.key));
  });

  it("seeds a title and then alternates titles and people along every branch", () => {
    const constellation = new Constellation(dataset(), { width: 1200, height: 800, random: seeded(3) });
    // The first step seeds twice: once because there is nothing to grow from, once on the reseed timer.
    constellation.step(0.1);
    expect(constellation.nodes).toHaveLength(2);
    for (const node of constellation.nodes) {
      expect(node.key).toMatch(/^t\d+$/);
      expect(node.parent).toBeNull();
    }

    run(constellation, 10);
    expect(constellation.nodes.length).toBeGreaterThan(4);
    for (const node of constellation.nodes) {
      expect(node.kind === "person" ? node.key[0] : "t").toBe(node.key[0]);
      if (node.parent) expect(node.parent.kind === "person").toBe(node.kind !== "person");
    }
    for (const edge of constellation.edges) expect(edge.a.kind === "person").not.toBe(edge.b.kind === "person");
  });

  it("labels films and series from the dataset and resolves their images", () => {
    const constellation = new Constellation(dataset(), { width: 1200, height: 800, random: seeded(11) });
    run(constellation, 12);
    const titles = constellation.nodes.filter((node) => node.kind !== "person");
    expect(titles.length).toBeGreaterThan(0);
    for (const node of titles) {
      expect(node.label).toMatch(/^Title \d+$/);
      expect(node.year).toBeGreaterThanOrEqual(1990);
      expect(node.kind).toBe(node.ref % 2 === 0 ? "film" : "series");
      expect(node.image === null || node.image!.startsWith("https://upload.wikimedia.org/")).toBe(true);
    }
    for (const node of constellation.nodes.filter((item) => item.kind === "person")) expect(node.label).toMatch(/^Person \d+$/);
  });

  it("cross-links to a node already on screen instead of duplicating it", () => {
    const constellation = new Constellation(dataset(), { width: 900, height: 700, random: seeded(5) });
    run(constellation, 60);
    const keys = constellation.nodes.map((node) => node.key);
    expect(new Set(keys).size).toBe(keys.length);
    // A dense dataset with few nodes has to reuse people, which only happens through extra edges.
    expect(constellation.edges.length).toBeGreaterThanOrEqual(constellation.nodes.length - 1);
  });

  it("never exceeds the node budget for the canvas", () => {
    const size = { width: 1600, height: 900 };
    const budget = Math.round(Math.min(84, Math.max(18, (size.width * size.height) / 26000)));
    const constellation = new Constellation(dataset(), { ...size, random: seeded(13) });
    let peak = 0;
    for (let elapsed = 0; elapsed < 200; elapsed += 1 / 30) {
      constellation.step(1 / 30);
      peak = Math.max(peak, live(constellation).length);
    }
    expect(budget).toBe(55);
    expect(peak).toBeLessThanOrEqual(budget);
    expect(peak).toBeGreaterThan(10);
  });

  it("scales the budget with the density option", () => {
    const size = { width: 1600, height: 900 };
    const dense = new Constellation(dataset(), { ...size, density: 0.4, random: seeded(13) });
    run(dense, 200);
    expect(live(dense).length).toBeLessThanOrEqual(Math.round(Math.min(84, Math.max(18, (size.width * size.height) / 26000)) * 0.4));
  });

  it("retires old branches and drops them once they have faded", () => {
    const constellation = new Constellation(dataset(), { width: 2000, height: 1200, random: seeded(17) });
    run(constellation, 40);
    const oldest = Math.min(...constellation.nodes.map((node) => node.born));
    expect(constellation.now - oldest).toBeLessThanOrEqual(34 + 1.6 + 1);
    // Everything still present is either alive or within the fade window.
    for (const node of constellation.nodes) {
      if (node.died !== null) expect(constellation.now - node.died).toBeLessThanOrEqual(1.6);
    }
    for (const edge of constellation.edges) {
      if (edge.a.died !== null || edge.b.died !== null) expect(edge.died).not.toBeNull();
    }
  });

  it("keeps the middle clear for the sign-in card", () => {
    const size = { width: 1400, height: 900 };
    const clear = { rx: 320, ry: 260 };
    const guarded = new Constellation(dataset(), { ...size, clear, random: seeded(23) });
    run(guarded, 80);
    for (const node of live(guarded)) expect(clearDistance(node, size, clear)).toBeGreaterThan(0.85);

    const open = new Constellation(dataset(), { ...size, random: seeded(23) });
    run(open, 80);
    expect(live(open).some((node) => clearDistance(node, size, clear) < 0.9)).toBe(true);
  });

  it("can pick up a keep-out zone after it has started", () => {
    const size = { width: 1400, height: 900 };
    const clear = { rx: 300, ry: 240 };
    const constellation = new Constellation(dataset(), { ...size, random: seeded(29) });
    run(constellation, 40);
    constellation.setClear(clear);
    run(constellation, 40);
    for (const node of live(constellation)) expect(clearDistance(node, size, clear)).toBeGreaterThan(0.85);
  });

  it("rescales positions and the budget on resize", () => {
    const constellation = new Constellation(dataset(), { width: 1000, height: 800, random: seeded(31) });
    run(constellation, 12);
    const before = constellation.nodes.map((node) => ({ x: node.x, y: node.y }));
    constellation.resize(2000, 400);
    constellation.nodes.forEach((node, index) => {
      expect(node.x).toBeCloseTo(before[index]!.x * 2, 6);
      expect(node.y).toBeCloseTo(before[index]!.y * 0.5, 6);
    });
    // 2000x400 is 800000px², the same budget band as before; growth keeps working after a resize.
    const nodesBefore = constellation.nodes.length;
    run(constellation, 10);
    expect(constellation.nodes.length).toBeGreaterThanOrEqual(nodesBefore);
    expect(live(constellation).length).toBeLessThanOrEqual(Math.round(Math.min(84, Math.max(18, 800000 / 26000))));
  });

  it("keeps nodes inside the canvas", () => {
    const size = { width: 1200, height: 800 };
    const constellation = new Constellation(dataset(), { ...size, random: seeded(37) });
    run(constellation, 120);
    for (const node of live(constellation)) {
      expect(node.x).toBeGreaterThan(-200);
      expect(node.x).toBeLessThan(size.width + 200);
      expect(node.y).toBeGreaterThan(-200);
      expect(node.y).toBeLessThan(size.height + 200);
    }
  });

  it("advances its clock by the step size", () => {
    const constellation = new Constellation(dataset(), { width: 800, height: 600, random: seeded(41) });
    constellation.step(0.5);
    constellation.step(0.25);
    expect(constellation.now).toBeCloseTo(0.75, 10);
  });
});
