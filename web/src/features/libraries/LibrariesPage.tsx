import { Link } from "@tanstack/react-router";
import { ActionContextMenu } from "glass-ux/actions";
import { IconStack2 } from "@tabler/icons-react";

import { Button, EmptyState, ErrorState, PageHeader, SkeletonBlock } from "@/components/ui";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { LibrariesOverviewDocument } from "@/graphql/generated/graphql";

import { LibraryCard, type LibraryOverview } from "./LibraryCard";
import { LibrarySettingsDialog, type LibrarySettingsTab } from "./LibrarySettingsDialog";
import { useLibraryActions } from "./useLibraryActions";
import { useCompleteConnection } from "./browser/useCompleteConnection";
import { useLibraryPreferences } from "@/features/personal/usePersonalLibrary";
import { IconPlus, IconStar } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { errorMessage } from "@/lib/graphql/errors";
import { useCallback, useState } from "react";

export function LibrariesPage() {
  const { rows: libraries, loading, error, refetch } = useCompleteConnection(LibrariesOverviewDocument, {}, (data) => data.libraries);
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const preferences = useLibraryPreferences();
  const [pending, setPending] = useState<string>();
  const favouriteIds = new Set(preferences.rows.filter((row) => row.favourite).map((row) => row.libraryId));
  const ordered = [...libraries].sort((a, b) => Number(favouriteIds.has(b.id)) - Number(favouriteIds.has(a.id)));

  const [settings, setSettings] = useState<{ library: LibraryOverview; tab: LibrarySettingsTab } | null>(null);
  const openSettings = useCallback((library: LibraryOverview, tab: LibrarySettingsTab = "general") => setSettings({ library, tab }), []);
  const actions = useLibraryActions<LibraryOverview>({ settings: openSettings, onChanged: () => void refetch() });

  return (
    <div className="page-gutter flex flex-col gap-8 py-8">
      <PageHeader
        title="Libraries"
        actions={
          isAdmin ? (
            <Link to="/settings/libraries">
              <Button variant="primary">
                <IconPlus size="md" /> Add library
              </Button>
            </Link>
          ) : null
        }
      />
      {preferences.error ? <ErrorState error={preferences.error} onRetry={() => void preferences.refetch()} /> : null}
      {error && libraries.length === 0 ? (
        <ErrorState error={error} onRetry={() => void refetch()} />
      ) : loading && libraries.length === 0 ? (
        <div className="library-grid">
          {Array.from({ length: 3 }, (_, index) => (
            <SkeletonBlock key={index} className="min-h-[21rem] rounded-card" />
          ))}
        </div>
      ) : libraries.length === 0 ? (
        <EmptyState icon={IconStack2} title="No libraries yet" description="Add a library to start cataloguing your media." />
      ) : (
        <div className="library-grid">
          {ordered.map((library) => (
            <div key={library.id} className="flex min-w-0 flex-col gap-1"><ActionContextMenu set={actions} targets={[library]} label={`Actions for ${library.name}`}><div className="min-w-0"><LibraryCard library={library} size="lg" /></div></ActionContextMenu>
              {library.userId === user?.id ? <Button variant="ghost" aria-pressed={favouriteIds.has(library.id)} aria-label={`${favouriteIds.has(library.id) ? "Unfavourite" : "Favourite"} ${library.name}`} isPending={pending === library.id} isDisabled={Boolean(pending) || !preferences.complete} onPress={async () => {
                setPending(library.id); try { await preferences.toggle(library.id); } catch (error) { toast.danger(errorMessage(error)); } finally { setPending(undefined); }
              }}><IconStar />{favouriteIds.has(library.id) ? "Favourite" : "Add to favourites"}</Button> : null}
            </div>
          ))}
        </div>
      )}
      {settings ? <LibrarySettingsDialog library={libraries.find((row) => row.id === settings.library.id) ?? settings.library} open onOpenChange={(open) => { if (!open) setSettings(null); }} tab={settings.tab} onTabChange={(tab) => setSettings({ ...settings, tab })} /> : null}
    </div>
  );
}
