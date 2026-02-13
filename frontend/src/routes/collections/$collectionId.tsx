import { createFileRoute, Link, redirect } from "@tanstack/react-router";
import { useMemo } from "react";
import { BreadcrumbItem, Breadcrumbs } from "@heroui/breadcrumbs";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Image } from "@heroui/image";
import { Spinner } from "@heroui/spinner";
import { IconMovie, IconStack } from "@tabler/icons-react";
import { RouteError } from "../../components/RouteError";
import { DataTable, type DataTableColumn } from "../../components/data-table";
import { gql, useQuery } from "../../lib/graphql/client";

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

interface CollectionNode {
  Id: string;
  LibraryId: string;
  TmdbCollectionId: number;
  Name: string;
  Overview: string | null;
  PosterUrl: string | null;
  BackdropUrl: string | null;
  MovieCount: number;
}

interface CollectionMovieNode {
  TmdbId: number;
  Title: string;
  Year: number | null;
  PosterUrl: string | null;
  LibraryMovieId: string | null;
  MediaFileId: string | null;
  Wanted: boolean;
}

interface CollectionQueryData {
  Collection: CollectionNode | null;
}

interface CollectionResolveByTmdbQueryData {
  Collections: {
    Edges: Array<{
      Node: CollectionNode;
    }>;
  };
}

interface LibraryQueryData {
  Library: {
    Id: string;
    Name: string;
  } | null;
}

interface CollectionDetailsQueryData {
  MovieCollectionDetails: {
    CollectionId: number;
    Name: string;
    Movies: CollectionMovieNode[];
  } | null;
}

const COLLECTION_QUERY = gql`
  query CollectionDetailRoute($Id: String!) {
    Collection(Id: $Id) {
      Id
      LibraryId
      TmdbCollectionId
      Name
      Overview
      PosterUrl
      BackdropUrl
      MovieCount
    }
  }
`;

const COLLECTION_RESOLVE_BY_TMDB_QUERY = gql`
  query CollectionDetailResolveByTmdbRoute($Where: CollectionWhereInput, $Page: PageInput) {
    Collections(Where: $Where, Page: $Page) {
      Edges {
        Node {
          Id
          LibraryId
          TmdbCollectionId
          Name
          Overview
          PosterUrl
          BackdropUrl
          MovieCount
        }
      }
    }
  }
`;

const LIBRARY_QUERY = gql`
  query CollectionDetailLibraryRoute($Id: String!) {
    Library(Id: $Id) {
      Id
      Name
    }
  }
`;

const COLLECTION_DETAILS_QUERY = gql`
  query CollectionDetailMoviesRoute($LibraryId: String!, $CollectionId: Int!) {
    MovieCollectionDetails(LibraryId: $LibraryId, CollectionId: $CollectionId) {
      CollectionId
      Name
      Movies {
        TmdbId
        Title
        Year
        PosterUrl
        LibraryMovieId
        MediaFileId
        Wanted
      }
    }
  }
`;

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
  } = useQuery<CollectionQueryData>(COLLECTION_QUERY, {
    variables: { Id: collectionId },
    fetchPolicy: "cache-and-network",
  });

  const directCollection =
    collectionData?.Collection ?? previousCollectionData?.Collection ?? null;

  const {
    data: resolvedCollectionData,
    previousData: previousResolvedCollectionData,
    loading: resolveCollectionLoading,
  } = useQuery<CollectionResolveByTmdbQueryData>(COLLECTION_RESOLVE_BY_TMDB_QUERY, {
    variables: {
      Where: {
        TmdbCollectionId: { Eq: parsedCollectionTmdbId },
      },
      Page: { Limit: 1, Offset: 0 },
    },
    skip: !hasNumericCollectionParam || directCollection != null,
    fetchPolicy: "cache-and-network",
  });

  const resolvedCollection =
    resolvedCollectionData?.Collections?.Edges?.[0]?.Node ??
    previousResolvedCollectionData?.Collections?.Edges?.[0]?.Node ??
    null;
  const collection = directCollection ?? resolvedCollection;

  const { data: libraryData } = useQuery<LibraryQueryData>(LIBRARY_QUERY, {
    variables: { Id: collection?.LibraryId ?? "" },
    skip: !collection?.LibraryId,
    fetchPolicy: "cache-and-network",
  });

  const {
    data: detailsData,
    previousData: previousDetailsData,
    loading: detailsLoading,
  } = useQuery<CollectionDetailsQueryData>(COLLECTION_DETAILS_QUERY, {
    variables: {
      LibraryId: collection?.LibraryId ?? "",
      CollectionId: collection?.TmdbCollectionId ?? -1,
    },
    skip: !collection?.LibraryId || collection?.TmdbCollectionId == null,
    fetchPolicy: "cache-and-network",
  });

  const movies =
    detailsData?.MovieCollectionDetails?.Movies ??
    previousDetailsData?.MovieCollectionDetails?.Movies ??
    [];

  const downloadedCount = useMemo(
    () => movies.filter((movie) => movie.MediaFileId != null).length,
    [movies],
  );
  const wantedCount = useMemo(
    () => movies.filter((movie) => movie.MediaFileId == null && movie.Wanted).length,
    [movies],
  );
  const missingCount = useMemo(
    () => movies.filter((movie) => movie.MediaFileId == null && !movie.Wanted).length,
    [movies],
  );

  const columns: DataTableColumn<CollectionMovieNode>[] = [
    {
      key: "title",
      label: "Movie",
      sortable: true,
      render: (movie) => (
        <div className="flex items-center gap-3">
          {movie.PosterUrl ? (
            <Image
              src={movie.PosterUrl}
              alt={movie.Title}
              className="w-10 h-14 object-cover rounded"
              loading="lazy"
            />
          ) : (
            <div className="w-10 h-14 bg-default-200 rounded flex items-center justify-center">
              <IconMovie size={18} className="text-purple-400" />
            </div>
          )}
          {movie.LibraryMovieId ? (
            <Link
              to="/movies/$movieId"
              params={{ movieId: movie.LibraryMovieId }}
              className="font-medium hover:opacity-80"
            >
              {movie.Title}
            </Link>
          ) : (
            <span className="font-medium">{movie.Title}</span>
          )}
        </div>
      ),
    },
    {
      key: "year",
      label: "Year",
      width: 90,
      sortable: true,
      render: (movie) => <span>{movie.Year ?? "—"}</span>,
    },
    {
      key: "status",
      label: "Status",
      width: 130,
      render: (movie) => (
        <Chip
          size="sm"
          variant="flat"
          color={movie.MediaFileId ? "success" : movie.Wanted ? "warning" : "danger"}
        >
          {movie.MediaFileId ? "Downloaded" : movie.Wanted ? "Wanted" : "Missing"}
        </Chip>
      ),
    },
  ];

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
            <p className="text-default-500">The collection may have been removed.</p>
          </CardBody>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto px-4 sm:px-6 lg:px-8 py-8 mb-20 space-y-6">
      <Breadcrumbs>
        <BreadcrumbItem href="/libraries">Libraries</BreadcrumbItem>
        {collection.LibraryId ? (
          <BreadcrumbItem href={`/libraries/${collection.LibraryId}`}>
            {libraryData?.Library?.Name || "Library"}
          </BreadcrumbItem>
        ) : null}
        {collection.LibraryId ? (
          <BreadcrumbItem href={`/libraries/${collection.LibraryId}/collections`}>
            Collections
          </BreadcrumbItem>
        ) : null}
        <BreadcrumbItem isCurrent>{collection.Name}</BreadcrumbItem>
      </Breadcrumbs>

      <Card className="overflow-hidden border-default-200">
        <div className="relative">
          {collection.BackdropUrl ? (
            <img
              src={collection.BackdropUrl}
              alt={collection.Name}
              className="h-64 w-full object-cover"
            />
          ) : (
            <div className="h-64 w-full bg-gradient-to-r from-content2 to-content3" />
          )}
          <div className="absolute inset-0 bg-gradient-to-r from-black/75 via-black/55 to-black/30" />
          <div className="absolute inset-0 p-6 sm:p-8 flex items-end">
            <div className="flex items-end gap-4 sm:gap-6 w-full">
              {collection.PosterUrl ? (
                <Image
                  src={collection.PosterUrl}
                  alt={collection.Name}
                  className="w-24 h-36 sm:w-32 sm:h-48 object-cover rounded-lg shrink-0 border border-white/15"
                />
              ) : (
                <div className="w-24 h-36 sm:w-32 sm:h-48 bg-black/30 rounded-lg flex items-center justify-center shrink-0 border border-white/15">
                  <IconStack size={28} className="text-white/70" />
                </div>
              )}
              <div className="text-white space-y-2 min-w-0">
                <h1 className="text-2xl sm:text-3xl font-bold">{collection.Name}</h1>
                <p className="text-sm text-white/80">TMDB #{collection.TmdbCollectionId}</p>
                {collection.Overview ? (
                  <p className="text-sm sm:text-base text-white/85 line-clamp-2 sm:line-clamp-3 max-w-3xl">
                    {collection.Overview}
                  </p>
                ) : null}
                <div className="flex flex-wrap gap-2 pt-1">
                  <Chip size="sm" color="success" variant="flat">
                    {downloadedCount} Downloaded
                  </Chip>
                  <Chip size="sm" color="warning" variant="flat">
                    {wantedCount} Wanted
                  </Chip>
                  <Chip size="sm" color="danger" variant="flat">
                    {missingCount} Missing
                  </Chip>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Card>

      <DataTable
        stateKey={`collection-detail-movies-${collection.Id}`}
        data={movies}
        columns={columns}
        getRowKey={(movie) => String(movie.TmdbId)}
        ariaLabel="Collection movies table"
        searchPlaceholder={`Search "${collection.Name}"...`}
        showItemCount
        isLoading={detailsLoading && movies.length === 0}
        showViewModeToggle
        cardGridClassName="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3"
        cardRenderer={({ item }) => (
          <Card className="bg-content1 border border-default-200">
            <CardBody className="p-3">
              <div className="flex gap-3">
                {item.PosterUrl ? (
                  <Image
                    src={item.PosterUrl}
                    alt={item.Title}
                    className="w-14 h-20 object-cover rounded-md shrink-0"
                    loading="lazy"
                  />
                ) : (
                  <div className="w-14 h-20 bg-default-200 rounded-md shrink-0 flex items-center justify-center">
                    <IconMovie size={16} className="text-purple-400" />
                  </div>
                )}
                <div className="min-w-0 flex-1 space-y-2">
                  {item.LibraryMovieId ? (
                    <Link
                      to="/movies/$movieId"
                      params={{ movieId: item.LibraryMovieId }}
                      className="block font-medium truncate hover:opacity-80"
                    >
                      {item.Title}
                    </Link>
                  ) : (
                    <p className="font-medium truncate">{item.Title}</p>
                  )}
                  <p className="text-xs text-default-500">{item.Year ?? "Unknown year"}</p>
                  <Chip
                    size="sm"
                    variant="flat"
                    color={item.MediaFileId ? "success" : item.Wanted ? "warning" : "danger"}
                  >
                    {item.MediaFileId ? "Downloaded" : item.Wanted ? "Wanted" : "Missing"}
                  </Chip>
                </div>
              </div>
            </CardBody>
          </Card>
        )}
      />
    </div>
  );
}
