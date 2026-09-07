import { useQuery } from "@apollo/client/react";

import { Link } from "@tanstack/react-router";
import { IconPlus, IconStack2 } from "@tabler/icons-react";

import { Button, EmptyState, ErrorState, PageHeader, SkeletonBlock } from "@/components/ui";
import { useIsAdmin } from "@/lib/auth/useSession";
import { LibrariesOverviewDocument } from "@/graphql/generated/graphql";

import { LibraryCard } from "./LibraryCard";

export function LibrariesPage() {
  const { data, previousData, loading, error, refetch } = useQuery(LibrariesOverviewDocument);
  const isAdmin = useIsAdmin();
  const libraries = (data ?? previousData)?.libraries.edges.map((edge) => edge.node) ?? [];

  return (
    <div className="page-gutter flex flex-col gap-8 py-8">
      <PageHeader
        title="Libraries"
        actions={
          isAdmin ? (
            <Link to="/settings/libraries">
              <Button variant="primary">
                <IconPlus size={16} /> Add library
              </Button>
            </Link>
          ) : null
        }
      />
      {error && libraries.length === 0 ? (
        <ErrorState error={error} onRetry={() => void refetch()} />
      ) : loading && libraries.length === 0 ? (
        <div className="library-grid">
          {Array.from({ length: 3 }, (_, index) => (
            <SkeletonBlock key={index} className="min-h-44 rounded-card" />
          ))}
        </div>
      ) : libraries.length === 0 ? (
        <EmptyState icon={IconStack2} title="No libraries yet" description="Add a library to start cataloguing your media." />
      ) : (
        <div className="library-grid">
          {libraries.map((library) => (
            <LibraryCard key={library.id} library={library} size="lg" />
          ))}
        </div>
      )}
    </div>
  );
}
