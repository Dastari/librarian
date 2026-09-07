import { useEffect, useRef, type RefObject } from "react";

import { loadFilmGraph } from "./graph";
import { Constellation, type SimEdge, type SimNode } from "./simulation";

interface FilmConstellationProps {
  className?: string;
  /** Overall strength of the drawing; keep it low so it reads as background. */
  opacity?: number;
  density?: number;
  /** Element to keep clear of nodes (the sign-in card); measured on resize. */
  clearRef?: RefObject<HTMLElement | null>;
}

interface Palette {
  title: string;
  series: string;
  person: string;
  text: string;
  muted: string;
  rim: string;
  fontSans: string;
  fontDisplay: string;
}

const FADE = 1.6;
const GROW = 0.9;
const POSTER = { w: 46, h: 68, r: 6 };
const PORTRAIT_RADIUS = 21;
const easeOut = (t: number) => 1 - Math.pow(1 - Math.min(1, Math.max(0, t)), 3);

function readPalette(): Palette {
  const style = getComputedStyle(document.documentElement);
  const get = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
  return {
    title: get("--brand", "#e0b458"),
    series: get("--ambient-b", "#6ea8ff"),
    person: get("--foreground", "#f5f5f7"),
    text: get("--foreground", "#f5f5f7"),
    muted: get("--muted", "#a1a1aa"),
    rim: get("--glass-rim-top", "rgba(255,255,255,0.35)"),
    fontSans: get("--font-sans", "system-ui, sans-serif"),
    fontDisplay: get("--font-display", "system-ui, sans-serif"),
  };
}

function life(item: { born: number; died: number | null }, now: number): number {
  const fadeIn = easeOut((now - item.born) / GROW);
  const fadeOut = item.died === null ? 1 : 1 - easeOut((now - item.died) / FADE);
  return fadeIn * fadeOut;
}

/** Lazily loaded thumbnails; a node draws its fallback until its picture is ready. */
class ImageCache {
  private readonly images = new Map<string, HTMLImageElement | null>();
  private readonly loadedAt = new Map<string, number>();

  get(url: string, now: number): { image: HTMLImageElement; since: number } | null {
    if (!this.images.has(url)) {
      const image = new Image();
      image.crossOrigin = "anonymous";
      image.referrerPolicy = "no-referrer";
      image.decoding = "async";
      image.onload = () => this.loadedAt.set(url, -1);
      image.onerror = () => this.images.set(url, null);
      image.src = url;
      this.images.set(url, image);
    }
    const image = this.images.get(url);
    if (!image || !image.complete || image.naturalWidth === 0) return null;
    let since = this.loadedAt.get(url) ?? -1;
    if (since < 0) {
      since = now;
      this.loadedAt.set(url, now);
    }
    return { image, since };
  }
}

/** Draws `image` cover-fitted into the current clip rectangle. */
function drawCover(context: CanvasRenderingContext2D, image: HTMLImageElement, x: number, y: number, w: number, h: number) {
  const scale = Math.max(w / image.naturalWidth, h / image.naturalHeight);
  const sw = w / scale;
  const sh = h / scale;
  const sx = (image.naturalWidth - sw) / 2;
  const sy = (image.naturalHeight - sh) * (h > w ? 0.25 : 0.4);
  context.drawImage(image, sx, sy, sw, sh, x, y, w, h);
}

function roundedRect(context: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  context.beginPath();
  context.roundRect(x, y, w, h, r);
}

/**
 * Full-screen, unobtrusive family chart of film drawn on a 2D canvas: posters for titles,
 * portraits for the people in them, and lines that reach out as each one spawns the next.
 * Sits under the sign-in card; it never handles input and stops drawing when the tab is hidden.
 */
export function FilmConstellation({ className, opacity = 0.9, density = 1, clearRef }: FilmConstellationProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const context = canvas.getContext("2d");
    if (!context) return;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    let cancelled = false;
    let frame = 0;
    let palette = readPalette();
    let simulation: Constellation | null = null;
    let last = performance.now();
    const pointer = { x: 0, y: 0, tx: 0, ty: 0 };
    const images = new ImageCache();

    const clearZone = (rect: DOMRect) => {
      const target = clearRef?.current?.getBoundingClientRect();
      if (!target || target.width >= rect.width * 0.9) return null;
      return { rx: target.width / 2 + 64, ry: target.height / 2 + 52 };
    };

    const size = () => {
      const rect = canvas.getBoundingClientRect();
      const dpr = Math.min(2, window.devicePixelRatio || 1);
      canvas.width = Math.round(rect.width * dpr);
      canvas.height = Math.round(rect.height * dpr);
      context.setTransform(dpr, 0, 0, dpr, 0, 0);
      simulation?.resize(rect.width, rect.height);
      simulation?.setClear(clearZone(rect));
      return rect;
    };

    const drawEdge = (edge: SimEdge, now: number) => {
      const alpha = life(edge, now);
      if (alpha <= 0) return;
      const progress = edge.died === null ? easeOut((now - edge.born) / GROW) : 1;
      const x = edge.a.x + (edge.b.x - edge.a.x) * progress;
      const y = edge.a.y + (edge.b.y - edge.a.y) * progress;
      context.strokeStyle = palette.title;
      context.lineCap = "round";
      context.globalAlpha = alpha * 0.1;
      context.lineWidth = 3;
      context.beginPath();
      context.moveTo(edge.a.x, edge.a.y);
      context.lineTo(x, y);
      context.stroke();
      context.globalAlpha = alpha * 0.45;
      context.lineWidth = 1;
      context.stroke();
    };

    const drawNode = (node: SimNode, now: number, width: number) => {
      const alpha = life(node, now);
      if (alpha <= 0) return;
      const person = node.kind === "person";
      const accent = person ? palette.person : node.kind === "series" ? palette.series : palette.title;
      const picture = node.image ? images.get(node.image, now) : null;
      const pictureAlpha = picture ? easeOut((now - picture.since) / 0.6) : 0;

      // Frame: a rounded poster for titles, a circle for people.
      const x = person ? node.x - PORTRAIT_RADIUS : node.x - POSTER.w / 2;
      const y = person ? node.y - PORTRAIT_RADIUS : node.y - POSTER.h / 2;
      const w = person ? PORTRAIT_RADIUS * 2 : POSTER.w;
      const h = person ? PORTRAIT_RADIUS * 2 : POSTER.h;
      const shape = () => {
        if (person) {
          context.beginPath();
          context.arc(node.x, node.y, PORTRAIT_RADIUS, 0, Math.PI * 2);
        } else {
          roundedRect(context, x, y, w, h, POSTER.r);
        }
      };

      // Soft halo behind the frame so it lifts off the gradient.
      const halo = context.createRadialGradient(node.x, node.y, 0, node.x, node.y, Math.max(w, h) * 0.9);
      halo.addColorStop(0, accent);
      halo.addColorStop(1, "transparent");
      context.globalAlpha = alpha * 0.18;
      context.fillStyle = halo;
      context.beginPath();
      context.arc(node.x, node.y, Math.max(w, h) * 0.9, 0, Math.PI * 2);
      context.fill();

      // Fallback fill: a tinted gradient with the initial letter.
      context.save();
      shape();
      context.clip();
      const fill = context.createLinearGradient(x, y, x + w, y + h);
      fill.addColorStop(0, accent);
      fill.addColorStop(1, "rgba(0,0,0,0.6)");
      context.globalAlpha = alpha * (person ? 0.55 : 0.6);
      context.fillStyle = fill;
      context.fillRect(x, y, w, h);
      if (pictureAlpha < 1) {
        context.globalAlpha = alpha * 0.9 * (1 - pictureAlpha);
        context.fillStyle = palette.text;
        context.font = `600 ${person ? 15 : 18}px ${palette.fontDisplay}`;
        context.textAlign = "center";
        context.textBaseline = "middle";
        context.fillText(node.label.charAt(0), node.x, node.y + 1);
      }
      if (picture) {
        context.globalAlpha = alpha * pictureAlpha;
        drawCover(context, picture.image, x, y, w, h);
      }
      context.restore();

      // Rim.
      shape();
      context.globalAlpha = alpha * 0.9;
      context.lineWidth = 1;
      context.strokeStyle = palette.rim;
      context.stroke();

      // Labels: title and year under the poster, name under the portrait.
      const flip = node.x > width - 120;
      context.textAlign = "center";
      context.textBaseline = "top";
      const labelX = flip ? node.x - 10 : node.x;
      const labelY = node.y + h / 2 + 5;
      const label = node.label.length > 28 ? `${node.label.slice(0, 27)}…` : node.label;
      if (person) {
        context.font = `500 11px ${palette.fontSans}`;
        context.fillStyle = palette.text;
        context.globalAlpha = alpha * 0.82;
        context.fillText(label, labelX, labelY);
      } else {
        context.font = `600 11.5px ${palette.fontDisplay}`;
        context.fillStyle = palette.text;
        context.globalAlpha = alpha * 0.9;
        context.fillText(label, labelX, labelY);
        if (node.year) {
          context.font = `400 10px ${palette.fontSans}`;
          context.fillStyle = palette.muted;
          context.globalAlpha = alpha * 0.75;
          context.fillText(String(node.year), labelX, labelY + 14);
        }
      }
    };

    const render = (rect: DOMRect) => {
      if (!simulation) return;
      const now = simulation.now;
      context.clearRect(0, 0, rect.width, rect.height);
      context.save();
      context.translate(pointer.x, pointer.y);
      for (const edge of simulation.edges) drawEdge(edge, now);
      for (const node of simulation.nodes) drawNode(node, now, rect.width);
      context.restore();
      context.globalAlpha = 1;
    };

    const tick = (time: number) => {
      if (cancelled || !simulation) return;
      const dt = Math.min(0.05, (time - last) / 1000);
      last = time;
      simulation.step(dt);
      pointer.x += (pointer.tx - pointer.x) * 0.04;
      pointer.y += (pointer.ty - pointer.y) * 0.04;
      render(canvas.getBoundingClientRect());
      frame = requestAnimationFrame(tick);
    };

    const onPointer = (event: PointerEvent) => {
      pointer.tx = (event.clientX / window.innerWidth - 0.5) * -18;
      pointer.ty = (event.clientY / window.innerHeight - 0.5) * -18;
    };

    // Static mode (reduced motion) still needs to repaint as pictures arrive.
    let staticRepaint = 0;
    const repaintStatic = () => {
      if (!reduced || cancelled) return;
      render(canvas.getBoundingClientRect());
      staticRepaint = window.setTimeout(repaintStatic, 400);
    };

    const observer = new ResizeObserver(() => {
      const rect = size();
      if (reduced) render(rect);
    });
    observer.observe(canvas);
    const themeObserver = new MutationObserver(() => {
      palette = readPalette();
      if (reduced) render(canvas.getBoundingClientRect());
    });
    themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme", "class"] });

    void loadFilmGraph().then((data) => {
      if (cancelled) return;
      const rect = size();
      simulation = new Constellation(data, { width: rect.width, height: rect.height, density, clear: clearZone(rect) });
      // Start with a picture already forming rather than an empty screen.
      for (let index = 0; index < (reduced ? 900 : 300); index += 1) simulation.step(1 / 30);
      if (reduced) {
        repaintStatic();
        window.setTimeout(() => window.clearTimeout(staticRepaint), 8000);
        return;
      }
      last = performance.now();
      frame = requestAnimationFrame(tick);
      window.addEventListener("pointermove", onPointer, { passive: true });
    });

    return () => {
      cancelled = true;
      cancelAnimationFrame(frame);
      window.clearTimeout(staticRepaint);
      observer.disconnect();
      themeObserver.disconnect();
      window.removeEventListener("pointermove", onPointer);
    };
  }, [density, clearRef]);

  return <canvas ref={canvasRef} className={className} style={{ opacity }} aria-hidden />;
}
