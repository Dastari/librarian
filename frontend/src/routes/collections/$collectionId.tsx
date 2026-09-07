import { createFileRoute, Link, redirect } from "@tanstack/react-router";
import { BreadcrumbItem, Breadcrumbs } from "@heroui/breadcrumbs";
import { Card, CardBody } from "@heroui/card";
import { Image } from "@heroui/image";
import { Spinner } from "@heroui/spinner";
import { RouteError } from "../../components/RouteError";
import { CollectionPoster } from "../../components/library/CollectionCardParts";
import { CollectionMoviesTable } from "../../components/library/CollectionMoviesTable";
import { useQuery } from "../../lib/graphql/client";
import {
  CollectionDetailLibraryRouteDocument,
  CollectionDetailMoviesRouteDocument,
  CollectionDetailResolveByTmdbRouteDocument,
  CollectionDetailRouteDocument,
  type CollectionDetailMoviesRouteQuery,
  type CollectionDetailResolveByTmdbRouteQuery,
  type CollectionDetailRouteQuery,
} from "../../lib/graphql/generated/graphql";

export const Route = createFileRoute("/collections/$collectionId")({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: "/",
        search: {
          signin: true,
          redirect: location.href,
        },
      });
    }
  },
  component: CollectionDetailPage,
  errorComponent: RouteError,
});

function CollectionDetailPage() {
  const { collectionId } = Route.useParams();
  const parsedCollectionTmdbId = Number.parseInt(collectionId, 10);
  const hasNumericCollectionParam =
    Number.isFinite(parsedCollectionTmdbId) &&
    parsedCollectionTmdbId > 0 &&
    String(parsedCollectionTmdbId) === collectionId;

  const {
    data: collectionData,
    previousData: previousCollectionData,
    loading: collectionLoading,
  } = useQuery<CollectionDetailRouteQuery>(CollectionDetailRouteDocument, {
    variables: { id: collectionId },
    fetchPolicy: "cache-and-network",
  });

  const directCollection =
    collectionData?.collection ?? previousCollectionData?.collection ?? null;
  const {
    data: resolvedCollectionData,
    previousData: previousResolvedCollectionData,
    loading: resolveCollectionLoading,
  } = useQuery<CollectionDetailResolveByTmdbRouteQuery>(
    CollectionDetailResolveByTmdbRouteDocument,
    {
      variables: {
        where: {
          tmdbCollectionId: { eq: parsedCollectionTmdbId },
        },
        page: { limit: 1, offset: 0 },
      },
      skip: !hasNumericCollectionParam || directCollection != null,
      fetchPolicy: "cache-and-network",
    },
  );
  const resolvedCollection =
    resolvedCollectionData?.collections?.edges?.[0]?.node ??
    previousResolvedCollectionData?.collections?.edges?.[0]?.node ??
    null;
  const collection = directCollection ?? resolvedCollection;

  const { data: libraryData } = useQuery(CollectionDetailLibraryRouteDocument, {
    variables: { id: collection?.libraryId ?? "" },
    skip: !collection?.libraryId,
    fetchPolicy: "cache-and-network",
  });

  const {
    data: detailsData,
    previousData: previousDetailsData,
    loading: detailsLoading,
  } = useQuery<CollectionDetailMoviesRouteQuery>(
    CollectionDetailMoviesRouteDocument,
    {
      variables: {
        libraryId: collection?.libraryId ?? "",
        collectionId: collection?.tmdbCollectionId ?? -1,
      },
      skip: !collection?.libraryId || collection?.tmdbCollectionId == null,
      fetchPolicy: "cache-and-network",
    },
  );

  const movies =
    detailsData?.movieCollectionDetails?.movies ??
    previousDetailsData?.movieCollectionDetails?.movies ??
    [];

  if ((collectionLoading || resolveCollectionLoading) && !collection) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 flex items-center justify-center min-h-[50vh]">
        <Spinner size="lg" />
      </div>
    );
  }

  if (!collection) {
    return (
      <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8">
        <Card className="bg-content1">
          <CardBody className="py-12 text-center">
            <h2 className="text-xl font-semibold mb-2">Collection not found</h2>
            <p className="text-default-500">
              The collection may have been removed.
            </p>
          </CardBody>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 mb-20 space-y-6">
      <Breadcrumbs>
        <BreadcrumbItem><Link to="/libraries">Libraries</Link></BreadcrumbItem>
        {collection.libraryId ? (
          <BreadcrumbItem>
            <Link to="/libraries/$libraryId" params={{ libraryId: collection.libraryId }}>{libraryData?.library?.name || "Library"}</Link>
          </BreadcrumbItem>
        ) : null}
        {collection.libraryId ? (
          <BreadcrumbItem>
            <Link to="/libraries/$libraryId/collections" params={{ libraryId: collection.libraryId }}>Collections</Link>
          </BreadcrumbItem>
        ) : null}
        <BreadcrumbItem isCurrent>{collection.name}</BreadcrumbItem>
      </Breadcrumbs>

      <Card className="overflow-hidden border-default-200">
        <div className="relative">
          {collection.backdropUrl ? (
            <Image
              src={collection.backdropUrl}
              alt={collection.name}
              className="h-64 w-full object-cover"
              removeWrapper
            />
          ) : (
            <div className="h-64 w-full bg-gradient-to-r from-content2 to-content3" />
          )}
          <div className="absolute inset-0 bg-gradient-to-r from-black/75 via-black/55 to-black/30" />
          <div className="absolute inset-0 p-6 sm:p-8 flex items-end z-20">
            <div className="flex items-end gap-4 sm:gap-6 w-full">
              <CollectionPoster
                posterUrl={collection.posterUrl}
                name={collection.name}
                imageClassName="w-24 h-36 sm:w-32 sm:h-48 object-cover rounded-lg shrink-0 border border-white/15"
                fallbackClassName="w-24 h-36 sm:w-32 sm:h-48 bg-black/30 rounded-lg flex items-center justify-center shrink-0 border border-white/15"
                iconSize={28}
                iconClassName="text-white/70"
              />
              <div className="text-white space-y-2 min-w-0">
                <h1 className="text-2xl sm:text-3xl font-bold text-shadow-sm">
                  {collection.name}
                </h1>
                {collection.overview ? (
                  <p className="text-sm sm:text-base text-white/85 line-clamp-2 sm:line-clamp-3 max-w-3xl text-shadow-sm">
                    {collection.overview}
                  </p>
                ) : null}
              </div>
            </div>
          </div>
        </div>
      </Card>

      <CollectionMoviesTable
        stateKey={`collection-detail-movies-${collection.id}`}
        ariaLabel="Collection movies table"
        toolbarQueryPlaceholder={`Search "${collection.name}"...`}
        isLoading={detailsLoading && movies.length === 0}
        movies={movies}
      />
    </div>
  );
}
