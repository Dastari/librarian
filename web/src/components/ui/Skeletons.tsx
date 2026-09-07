import { cn } from "@/lib/utils";

import type { ArtworkAspect } from "./Artwork";

const ASPECT: Record<ArtworkAspect, string> = {
  poster: "aspect-[2/3]",
  square: "aspect-square",
  backdrop: "aspect-video",
  banner: "aspect-[5/1]",
};

export function SkeletonBlock({ className }: { className?: string }) {
  return <div aria-hidden className={cn("skeleton", className)} />;
}

export function SkeletonText({ lines = 2, className }: { lines?: number; className?: string }) {
  return (
    <div aria-hidden className={cn("flex flex-col gap-2", className)}>
      {Array.from({ length: lines }, (_, index) => (
        <div key={index} className="skeleton h-3.5" style={{ width: `${index === lines - 1 ? 60 : 100 - index * 8}%` }} />
      ))}
    </div>
  );
}

export function SkeletonCard({ aspect = "poster", className }: { aspect?: ArtworkAspect; className?: string }) {
  return (
    <div aria-hidden className={cn("flex flex-col gap-2", className)}>
      <div className={cn("skeleton rounded-poster", ASPECT[aspect])} />
      <div className="skeleton h-3.5 w-4/5" />
      <div className="skeleton h-3 w-1/2" />
    </div>
  );
}

export function SkeletonRow({ count = 8, aspect = "poster" }: { count?: number; aspect?: ArtworkAspect }) {
  return (
    <div className="snap-row page-gutter">
      {Array.from({ length: count }, (_, index) => (
        <SkeletonCard key={index} aspect={aspect} className="w-(--poster-min)" />
      ))}
    </div>
  );
}

export function SkeletonGrid({ count = 12, aspect = "poster" }: { count?: number; aspect?: ArtworkAspect }) {
  return (
    <div className="poster-grid">
      {Array.from({ length: count }, (_, index) => (
        <SkeletonCard key={index} aspect={aspect} />
      ))}
    </div>
  );
}

export function SkeletonHero() {
  return <div aria-hidden className="skeleton h-[52svh] min-h-80 w-full rounded-none" />;
}
