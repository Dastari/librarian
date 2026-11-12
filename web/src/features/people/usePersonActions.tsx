import { useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconExternalLink, IconRefresh, IconSearch } from "glass-ux/icons";
import { useMemo } from "react";

import { RefreshPersonDetailsDocument } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { assertSuccess } from "@/lib/graphql/errors";

import type { PersonEntity } from "./PersonIdentity";

export interface PersonActionTarget extends PersonEntity {
  imdbId?: string | null;
}

const openExternal = (url: string) => window.open(url, "_blank", "noopener,noreferrer");

/** Everything that can be done with a person, wherever they are shown. */
export function usePersonActions<T extends PersonActionTarget>({ open = true }: { open?: boolean } = {}): ActionSet<T> {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const [refresh, { loading: refreshing }] = useMutation(RefreshPersonDetailsDocument);

  return useMemo(() => {
    const one = (rows: readonly T[]) => rows.length === 1;
    const onTmdb = (rows: readonly T[]) => one(rows) && (rows[0]!.tmdbPersonId ?? 0) > 0;
    return defineActions<T>("person", [
      ...(open ? [{
        id: "open", label: "Open person", icon: IconExternalLink, group: "open" as const, priority: 0, primary: true,
        visible: one,
        run: (rows: readonly T[]) => void navigate({ to: "/people/$personId", params: { personId: rows[0]!.id } }),
      }] : []),
      {
        id: "search", label: "Search the library", icon: IconSearch, group: "open", priority: 1,
        visible: one,
        run: (rows) => void navigate({ to: "/search", search: { q: rows[0]!.name } }),
      },
      {
        id: "tmdb", label: "Open on TMDB", icon: IconExternalLink, group: "open", priority: 2,
        visible: onTmdb,
        run: (rows) => openExternal(`https://www.themoviedb.org/person/${rows[0]!.tmdbPersonId}`),
      },
      {
        id: "imdb", label: "Open on IMDb", icon: IconExternalLink, group: "open", priority: 3,
        visible: (rows) => one(rows) && Boolean(rows[0]!.imdbId),
        run: (rows) => openExternal(`https://www.imdb.com/name/${rows[0]!.imdbId}`),
      },
      ...(isAdmin ? [{
        id: "refresh", label: "Refresh details", icon: IconRefresh, group: "manage" as const, priority: 4,
        visible: onTmdb,
        disabledReason: () => (refreshing ? "Already refreshing" : false),
        run: async (rows: readonly T[]) => {
          assertSuccess((await refresh({ variables: { id: rows[0]!.id } })).data?.refreshPersonDetails, "Could not refresh details");
        },
        successMessage: () => "Details refreshed",
      }] : []),
    ]);
  }, [isAdmin, navigate, open, refresh, refreshing]);
}
