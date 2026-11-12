import { useMutation } from "@apollo/client/react";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconExternalLink, IconFile, IconFolder, IconPlus, IconRefresh, IconSettings, IconStar, IconTrash, IconWarning } from "glass-ux/icons";
import { parseAsString, useQueryState } from "nuqs";
import { useCallback, useMemo } from "react";

import { ScanLibraryDocument } from "@/graphql/generated/graphql";
import { addHref, addScopeFor } from "@/features/search/addScopes";
import { useIsAdmin } from "@/lib/auth/useSession";
import { libraryType } from "@/lib/library-types";

import type { LibraryEntity } from "./LibraryIdentity";
import { isLibrarySettingsTab, type LibrarySettingsTab } from "./LibrarySettingsDialog";

/**
 * Everything that can be done to a library, wherever it's shown: its header, a table row's menu,
 * a card's right-click. `settings` opens its dialog at a section. `open` is left out on its page.
 */
export function useLibraryActions<T extends LibraryEntity>({ settings, open = true, onChanged, favourites }: {
  settings: (library: T, tab?: LibrarySettingsTab) => void;
  open?: boolean;
  onChanged?: () => void;
  /**
   * Where a person's favourite libraries are known (the Libraries page): adds Add to / Remove from
   * favourites, which is how a card's star is toggled from a remote or the keyboard.
   */
  favourites?: { isFavourite: (library: T) => boolean; canChange: (library: T) => boolean; toggle: (library: T) => Promise<unknown> };
}): ActionSet<T> {
  const isAdmin = useIsAdmin();
  const [scan] = useMutation(ScanLibraryDocument);
  return useMemo(() => defineActions<T>("library", [
    ...(open ? [{
      id: "open", label: "Open library", icon: IconExternalLink, group: "open" as const, priority: 1,
      visible: (rows: readonly T[]) => rows.length === 1,
      run: (rows: readonly T[], context: { navigate: (href: string) => void }) => context.navigate(`/libraries/${rows[0]!.id}`),
    }] : []),
    ...(isAdmin ? [{
      id: "scan", label: (rows: readonly T[]) => (rows.length === 1 ? "Scan now" : `Scan ${rows.length} libraries`), icon: IconRefresh, group: "manage" as const, priority: 1, bulk: true, primary: !open,
      disabledReason: (rows: readonly T[]) => (rows.every((row) => row.scanning) ? "Already scanning" : false),
      run: async (rows: readonly T[]) => {
        for (const row of rows.filter((item) => !item.scanning)) {
          const result = (await scan({ variables: { id: row.id } })).data?.scanLibrary;
          if (!result?.success) throw new Error(result?.message ?? `${row.name}: the scan could not start`);
        }
        onChanged?.();
      },
      successMessage: (rows: readonly T[]) => (rows.length === 1 ? "Scan started" : `Started ${rows.length} scans`),
    }] : []),
    ...(isAdmin ? [{
      // Where new titles come from: Search, on the tab that adds this library's kind.
      id: "add", label: (rows: readonly T[]) => addScopeFor(libraryType(rows[0]?.libraryType).type)?.label ?? "Add", icon: IconPlus, group: "open" as const, priority: 0,
      visible: (rows: readonly T[]) => rows.length === 1 && addHref(libraryType(rows[0]!.libraryType).type) !== null,
      run: (rows: readonly T[], context: { navigate: (href: string) => void }) => context.navigate(addHref(libraryType(rows[0]!.libraryType).type)!),
    }] : []),
    ...(favourites ? [{
      id: "favourite", label: (rows: readonly T[]) => (favourites.isFavourite(rows[0]!) ? "Remove from favourites" : "Add to favourites"), icon: IconStar, group: "open" as const, priority: 4,
      visible: (rows: readonly T[]) => rows.length === 1 && favourites.canChange(rows[0]!),
      run: (rows: readonly T[]) => favourites.toggle(rows[0]!),
    }] : []),
    {
      id: "files", label: "Files", icon: IconFile, group: "open", priority: 2,
      visible: (rows) => rows.length === 1,
      run: (rows) => settings(rows[0]!, "files"),
    },
    {
      id: "issues", label: "Scan issues", icon: IconWarning, group: "open", priority: 3,
      visible: (rows) => rows.length === 1,
      run: (rows) => settings(rows[0]!, "issues"),
    },
    {
      id: "settings", label: "Library settings", icon: IconSettings, group: "manage", priority: 2,
      visible: (rows) => rows.length === 1 && isAdmin,
      run: (rows) => settings(rows[0]!),
    },
    {
      // Changes where the library reads its files, keeping every record. The files are copied first.
      id: "move", label: "Move library…", icon: IconFolder, group: "manage", priority: 3,
      visible: (rows) => rows.length === 1 && isAdmin,
      run: (rows) => settings(rows[0]!, "move"),
    },
    {
      id: "remove", label: "Remove library…", icon: IconTrash, group: "danger", priority: 9,
      visible: (rows) => rows.length === 1 && isAdmin,
      run: (rows) => settings(rows[0]!, "remove"),
    },
  ]), [isAdmin, open, scan, settings, onChanged, favourites]);
}

/** The settings dialog's open section, in the URL so it can be linked. */
export function useLibrarySettingsParam() {
  const [param, setParam] = useQueryState("settings", parseAsString.withOptions({ history: "replace" }));
  const tab: LibrarySettingsTab | null = param === null ? null : isLibrarySettingsTab(param) ? param : "general";
  const open = useCallback((_: unknown, next?: LibrarySettingsTab) => void setParam(next ?? "general"), [setParam]);
  const close = useCallback(() => void setParam(null), [setParam]);
  return { tab, open, close };
}
