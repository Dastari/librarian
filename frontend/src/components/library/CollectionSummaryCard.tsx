import { Card, CardBody } from "@heroui/card";
import { Image } from "@heroui/image";
import { CollectionPoster } from "./CollectionCardParts";

export interface CollectionSummaryCardProps {
  name: string;
  posterUrl: string | null;
  backdropUrl: string | null;
  hasFileCount: number;
  totalMovieCount: number;
  onPress: () => void;
}

export function CollectionSummaryCard({
  name,
  posterUrl,
  backdropUrl,
  hasFileCount,
  totalMovieCount,
  onPress,
}: CollectionSummaryCardProps) {
  return (
    <Card
      isPressable
      className="relative overflow-hidden border border-default-200 hover:border-primary/40 transition-colors w-full"
      classNames={{
        body: "p-0",
      }}
      onPress={onPress}
    >
      {backdropUrl ? (
        <Image
          src={backdropUrl}
          alt={name}
          className="absolute inset-0 h-full w-full object-cover"
          removeWrapper
        />
      ) : (
        <div className="absolute inset-0 bg-gradient-to-r from-content2 to-content3" />
      )}

      <div className="absolute inset-0 bg-black/45 backdrop-blur-md" />

      <CardBody className="relative z-10">
        <div className="max-w-full flex items-center gap-3 rounded-lg border border-white/15 bg-black/35 px-3 py-2">
          <CollectionPoster
            posterUrl={posterUrl}
            name={name}
            imageClassName="w-12 h-16 object-cover rounded-md shrink-0"
            fallbackClassName="w-12 h-16 bg-black/25 rounded-md flex items-center justify-center shrink-0"
          />
          <div className="min-w-0">
            <p className="font-semibold text-white truncate text-shadow-sm">
              {name}
            </p>
            <p className="text-xs text-white/85 text-shadow-sm">
              {hasFileCount}/{totalMovieCount} in collection
            </p>
          </div>
        </div>
      </CardBody>
    </Card>
  );
}
