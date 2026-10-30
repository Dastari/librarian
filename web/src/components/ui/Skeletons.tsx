import { Skeleton } from "glass-ux/skeleton";

import { cn } from "@/lib/utils";

import type { ArtworkAspect } from "./Artwork";

const ASPECT: Record<ArtworkAspect, string> = {
  poster: "aspect-[210/297]",
  square: "aspect-square",
  backdrop: "aspect-video",
  banner: "aspect-[5/1]",
};

export function SkeletonBlock({ className }: { className?: string }) {
  return <Skeleton.Block className={className} />;
}

export function SkeletonText({ lines = 2, className }: { lines?: number; className?: string }) {
  return <Skeleton.Text type="body" lines={lines} width="60%" className={className} />;
}

export function SkeletonCard({ aspect = "poster", className }: { aspect?: ArtworkAspect; className?: string }) {
  if (aspect === "poster") {
    return <Skeleton.Block aria-hidden radius="tile" className={cn(ASPECT.poster, className)} />;
  }
  return (
    <div aria-hidden className={cn("flex flex-col gap-2", className)}>
      <Skeleton.Block radius="tile" className={ASPECT[aspect]} />
      <Skeleton.Text type="label" width="80%" />
      <Skeleton.Text type="caption" width="50%" />
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
  return <Skeleton.Block radius="surface" className="h-[52svh] min-h-80 w-full" />;
}
