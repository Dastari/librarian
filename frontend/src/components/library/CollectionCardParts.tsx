import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { IconStack } from "@tabler/icons-react";

type CollectionPosterProps = {
  posterUrl: string | null;
  name: string;
  imageClassName: string;
  fallbackClassName: string;
  iconSize?: number;
  iconClassName?: string;
};

export function CollectionPoster({
  posterUrl,
  name,
  imageClassName,
  fallbackClassName,
  iconSize = 18,
  iconClassName = "text-purple-400",
}: CollectionPosterProps) {
  if (posterUrl) {
    return (
      <Image
        src={posterUrl}
        alt={name}
        className={imageClassName}
        loading="lazy"
      />
    );
  }

  return (
    <div className={fallbackClassName}>
      <IconStack size={iconSize} className={iconClassName} />
    </div>
  );
}

type CollectionStatusChipsProps = {
  downloadedCount: number;
  wantedCount: number;
  missingCount: number;
  showLabels?: boolean;
  className?: string;
};

export function CollectionStatusChips({
  downloadedCount,
  wantedCount,
  missingCount,
  showLabels = true,
  className = "flex items-center gap-2 flex-wrap",
}: CollectionStatusChipsProps) {
  return (
    <div className={className}>
      <Chip size="sm" color="success" variant="flat">
        {downloadedCount}
        {showLabels ? " Downloaded" : ""}
      </Chip>
      <Chip size="sm" color="warning" variant="flat">
        {wantedCount}
        {showLabels ? " Wanted" : ""}
      </Chip>
      <Chip size="sm" color="danger" variant="flat">
        {missingCount}
        {showLabels ? " Missing" : ""}
      </Chip>
    </div>
  );
}
