import { imageUrl, type Adjacency } from "./graph";

/**
 * A slowly growing family chart of film: a title spawns its cast, each actor spawns another of
 * their titles, and when an actor who is already on screen turns up again the chart cross-links
 * instead of duplicating them. Old branches fade as new ones grow so the picture keeps moving
 * without ever filling up.
 */

export type NodeKind = "film" | "series" | "person";

export interface SimNode {
  key: string;
  kind: NodeKind;
  label: string;
  year?: number;
  /** Index into the dataset's titles or people. */
  ref: number;
  /** Thumbnail URL, when the dataset has one. */
  image: string | null;
  x: number;
  y: number;
  vx: number;
  vy: number;
  born: number;
  died: number | null;
  parent: SimNode | null;
  /** Neighbours already spawned from this node. */
  used: Set<number>;
  degree: number;
}

export interface SimEdge {
  a: SimNode;
  b: SimNode;
  born: number;
  died: number | null;
}

export interface SimulationOptions {
  width: number;
  height: number;
  /** Multiplies the node budget; 1 is the default density. */
  density?: number;
  /** Half-extents of an elliptical keep-out zone in the middle, e.g. behind a sign-in card. */
  clear?: { rx: number; ry: number } | null;
  random?: () => number;
}

const FADE = 1.6;
const LIFETIME = 34;
const SPAWN_EVERY = 0.42;
const RESEED_EVERY = 5;
const LINK_LENGTH = 132;
const REPULSE_RADIUS = 190;

export class Constellation {
  readonly nodes: SimNode[] = [];
  readonly edges: SimEdge[] = [];
  private readonly byKey = new Map<string, SimNode>();
  private time = 0;
  private nextSpawn = 0;
  private nextSeed = 0;
  private width: number;
  private height: number;
  private budget: number;
  private readonly random: () => number;

  constructor(private readonly data: Adjacency, options: SimulationOptions) {
    this.width = options.width;
    this.height = options.height;
    this.random = options.random ?? Math.random;
    this.budget = this.computeBudget(options.density ?? 1);
    this.density = options.density ?? 1;
    this.clear = options.clear ?? null;
  }

  private density: number;
  private clear: { rx: number; ry: number } | null;

  setClear(clear: { rx: number; ry: number } | null): void {
    this.clear = clear;
  }

  resize(width: number, height: number): void {
    const sx = width / this.width;
    const sy = height / this.height;
    for (const node of this.nodes) {
      node.x *= sx;
      node.y *= sy;
    }
    this.width = width;
    this.height = height;
    this.budget = this.computeBudget(this.density);
  }

  private computeBudget(density: number): number {
    const area = this.width * this.height;
    return Math.round(Math.min(84, Math.max(18, area / 26000)) * density);
  }

  /** Advances the chart by `dt` seconds: growth, retirement and the layout forces. */
  step(dt: number): void {
    this.time += dt;
    if (this.time >= this.nextSpawn) {
      this.grow();
      const live = this.liveCount();
      this.nextSpawn = this.time + SPAWN_EVERY * (live > this.budget * 0.8 ? 1.8 : 1);
    }
    if (this.time >= this.nextSeed) {
      this.seed();
      this.nextSeed = this.time + RESEED_EVERY;
    }
    this.retire();
    this.integrate(dt);
  }

  get now(): number {
    return this.time;
  }

  private liveCount(): number {
    let count = 0;
    for (const node of this.nodes) if (node.died === null) count += 1;
    return count;
  }

  private pick<T>(list: T[]): T {
    return list[Math.floor(this.random() * list.length)]!;
  }

  private seed(): void {
    const { titles } = this.data.graph;
    // Prefer well-connected titles so the first branch has somewhere to go.
    let index = 0;
    for (let attempt = 0; attempt < 6; attempt += 1) {
      index = Math.floor(this.random() * titles.length);
      if (this.data.cast[index]!.length >= 3) break;
    }
    if (this.byKey.has(`t${index}`)) return;
    const spot = this.sparsePoint();
    this.addTitle(index, spot.x, spot.y, null);
  }

  /** Picks the emptiest of a handful of random points, keeping the middle clear-ish. */
  private sparsePoint(): { x: number; y: number } {
    let best = { x: this.width * 0.5, y: this.height * 0.5 };
    let bestDistance = -1;
    for (let attempt = 0; attempt < 10; attempt += 1) {
      const x = this.width * (0.08 + this.random() * 0.84);
      const y = this.height * (0.1 + this.random() * 0.8);
      if (this.inClearZone(x, y)) continue;
      let nearest = Infinity;
      for (const node of this.nodes) {
        if (node.died !== null) continue;
        const d = Math.hypot(node.x - x, node.y - y);
        if (d < nearest) nearest = d;
      }
      if (nearest > bestDistance) {
        bestDistance = nearest;
        best = { x, y };
      }
    }
    return best;
  }

  /** Superellipse distance from the centre: < 1 is inside the (rounded-rectangle) keep-out zone. */
  private clearDistance(x: number, y: number): number {
    if (!this.clear) return Infinity;
    const dx = Math.abs(x - this.width / 2) / this.clear.rx;
    const dy = Math.abs(y - this.height / 2) / this.clear.ry;
    return Math.pow(Math.pow(dx, 4) + Math.pow(dy, 4), 0.25);
  }

  private inClearZone(x: number, y: number): boolean {
    return this.clearDistance(x, y) < 1;
  }

  private grow(): void {
    const frontier = this.nodes.filter((node) => node.died === null && this.time - node.born < LIFETIME * 0.6 && node.used.size < this.neighbours(node).length && node.used.size < (node.kind === "person" ? 3 : 5));
    if (frontier.length === 0) {
      this.seed();
      return;
    }
    // Recent nodes grow first so branches keep extending outward.
    frontier.sort((a, b) => b.born - a.born);
    const parent = frontier[Math.min(frontier.length - 1, Math.floor(Math.pow(this.random(), 2) * frontier.length))]!;
    const options = this.neighbours(parent).filter((index) => !parent.used.has(index));
    if (options.length === 0) return;

    // Favour neighbours that lead somewhere: actors with several titles, titles with a cast.
    const weighted = options.filter((index) => (parent.kind === "person" ? this.data.cast[index]!.length : this.data.credits[index]!.length) > 1);
    const choice = weighted.length > 0 && this.random() < 0.75 ? this.pick(weighted) : this.pick(options);
    parent.used.add(choice);

    const key = parent.kind === "person" ? `t${choice}` : `p${choice}`;
    const existing = this.byKey.get(key);
    if (existing && existing.died === null) {
      if (!this.edges.some((edge) => edge.died === null && ((edge.a === parent && edge.b === existing) || (edge.a === existing && edge.b === parent)))) {
        this.edges.push({ a: parent, b: existing, born: this.time, died: null });
        existing.used.add(parent.ref);
      }
      return;
    }

    // Grow away from the grandparent so chains stretch out instead of folding back.
    const away = parent.parent ? Math.atan2(parent.y - parent.parent.y, parent.x - parent.parent.x) : this.random() * Math.PI * 2;
    const angle = away + (this.random() - 0.5) * Math.PI * 1.1;
    const length = LINK_LENGTH * (0.75 + this.random() * 0.5);
    const x = parent.x + Math.cos(angle) * length;
    const y = parent.y + Math.sin(angle) * length;
    const child = parent.kind === "person" ? this.addTitle(choice, x, y, parent) : this.addPerson(choice, x, y, parent);
    child.used.add(parent.ref);
  }

  private neighbours(node: SimNode): number[] {
    return node.kind === "person" ? this.data.credits[node.ref]! : this.data.cast[node.ref]!;
  }

  private addTitle(index: number, x: number, y: number, parent: SimNode | null): SimNode {
    const [label, year, kind, image] = this.data.graph.titles[index]!;
    return this.add({ key: `t${index}`, kind: kind === 0 ? "film" : "series", label, year, ref: index, image: imageUrl(image), degree: this.data.cast[index]!.length }, x, y, parent);
  }

  private addPerson(index: number, x: number, y: number, parent: SimNode | null): SimNode {
    const [label, image] = this.data.graph.people[index]!;
    return this.add({ key: `p${index}`, kind: "person", label, ref: index, image: imageUrl(image), degree: this.data.credits[index]!.length }, x, y, parent);
  }

  private add(base: Pick<SimNode, "key" | "kind" | "label" | "year" | "ref" | "image" | "degree">, x: number, y: number, parent: SimNode | null): SimNode {
    const node: SimNode = { ...base, x, y, vx: 0, vy: 0, born: this.time, died: null, parent, used: new Set() };
    this.nodes.push(node);
    this.byKey.set(node.key, node);
    if (parent) this.edges.push({ a: parent, b: node, born: this.time, died: null });
    return node;
  }

  private retire(): void {
    const live = this.nodes.filter((node) => node.died === null);
    const over = live.length - this.budget;
    if (over > 0) {
      live.sort((a, b) => a.born - b.born);
      for (const node of live.slice(0, over)) node.died = this.time;
    }
    for (const node of live) {
      if (node.died === null && this.time - node.born > LIFETIME) node.died = this.time;
    }
    for (const edge of this.edges) {
      if (edge.died === null && (edge.a.died !== null || edge.b.died !== null)) edge.died = Math.min(edge.a.died ?? Infinity, edge.b.died ?? Infinity);
    }
    // Drop fully faded items.
    const gone = (died: number | null) => died !== null && this.time - died > FADE;
    for (let index = this.edges.length - 1; index >= 0; index -= 1) if (gone(this.edges[index]!.died)) this.edges.splice(index, 1);
    for (let index = this.nodes.length - 1; index >= 0; index -= 1) {
      const node = this.nodes[index]!;
      if (gone(node.died)) {
        this.nodes.splice(index, 1);
        if (this.byKey.get(node.key) === node) this.byKey.delete(node.key);
      }
    }
  }

  private integrate(dt: number): void {
    const nodes = this.nodes;
    const step = Math.min(dt, 1 / 30) * 60;
    // Pairwise repulsion keeps labels legible.
    for (let i = 0; i < nodes.length; i += 1) {
      const a = nodes[i]!;
      for (let j = i + 1; j < nodes.length; j += 1) {
        const b = nodes[j]!;
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const d2 = dx * dx + dy * dy;
        if (d2 > REPULSE_RADIUS * REPULSE_RADIUS || d2 === 0) continue;
        const d = Math.sqrt(d2);
        const push = ((REPULSE_RADIUS - d) / REPULSE_RADIUS) * 1.1;
        const fx = (dx / d) * push;
        const fy = (dy / d) * push;
        a.vx -= fx;
        a.vy -= fy;
        b.vx += fx;
        b.vy += fy;
      }
    }
    // Links behave like soft springs.
    for (const edge of this.edges) {
      const dx = edge.b.x - edge.a.x;
      const dy = edge.b.y - edge.a.y;
      const d = Math.hypot(dx, dy) || 1;
      const stretch = (d - LINK_LENGTH) * 0.02;
      const fx = (dx / d) * stretch;
      const fy = (dy / d) * stretch;
      edge.a.vx += fx;
      edge.a.vy += fy;
      edge.b.vx -= fx;
      edge.b.vy -= fy;
    }
    const margin = 48;
    const clear = this.clear;
    for (const node of nodes) {
      // Keep the middle clear so the sign-in card sits on calm ground; lines may still cross it.
      if (clear) {
        const distance = this.clearDistance(node.x, node.y);
        if (distance < 1.08) {
          const dx = (node.x - this.width / 2) / clear.rx;
          const dy = (node.y - this.height / 2) / clear.ry;
          const push = (1.08 - distance) * 4;
          const length = Math.hypot(dx, dy) || 1;
          node.vx += (dx / length) * push;
          node.vy += (dy / length) * push;
        }
      }
      // Soft walls and a breath of drift so nothing sits perfectly still.
      if (node.x < margin) node.vx += (margin - node.x) * 0.02;
      if (node.x > this.width - margin) node.vx -= (node.x - (this.width - margin)) * 0.02;
      if (node.y < margin) node.vy += (margin - node.y) * 0.02;
      if (node.y > this.height - margin) node.vy -= (node.y - (this.height - margin)) * 0.02;
      node.vx += Math.sin(this.time * 0.35 + node.ref) * 0.01;
      node.vy += Math.cos(this.time * 0.29 + node.ref * 0.7) * 0.01;
      node.vx *= 0.86;
      node.vy *= 0.86;
      node.x += node.vx * step;
      node.y += node.vy * step;
    }
  }
}
