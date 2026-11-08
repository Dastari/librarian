import { useLazyQuery, useMutation } from "@apollo/client/react";
import { useNavigate } from "@tanstack/react-router";
import { defineActions, type ActionSet } from "glass-ux/actions";
import { IconBookmark, IconDownload, IconExternalLink, IconPlay, IconRefresh, IconSettings, IconTrash } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { useMemo, useState, type ReactNode } from "react";

import { ChooseListingDialog, type Listing } from "@/features/downloads/ChooseListingDialog";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import { AudiobookEditionsDocument, ChangeAudiobookEditionDocument, ListenToAudiobookChaptersDocument, RestoreAudiobookChaptersDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import type { AudiobookEntity } from "./AudiobookIdentity";
import { resetProgressAction, useResetProgress, type ProgressReset } from "./progressReset";

export const AUDIOBOOK_SETTINGS_TABS = ["general", "downloads", "files", "info", "remove"] as const;
export type AudiobookSettingsTab = (typeof AUDIOBOOK_SETTINGS_TABS)[number];

/** Everything that can be done to a book, wherever it's shown. `play` is where its page would resume. */
export function useAudiobookActions<T extends AudiobookEntity>({ settings, open = true, play, onChanged, reset }: {
  settings: (book: T, tab?: AudiobookSettingsTab) => void;
  open?: boolean;
  play?: { label: string; run: () => void; disabledReason?: string };
  onChanged?: () => void;
  /** Where this person's progress is known (its page), which adds "Mark as unplayed". */
  reset?: ProgressReset<T>;
}): { actions: ActionSet<T>; dialogs: ReactNode } {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  const [restore] = useMutation(RestoreAudiobookChaptersDocument);
  const [listen] = useMutation(ListenToAudiobookChaptersDocument);
  const [change] = useMutation(ChangeAudiobookEditionDocument);
  const [searchEditions, editions] = useLazyQuery(AudiobookEditionsDocument, { fetchPolicy: "network-only" });
  const resetProgress = useResetProgress();
  const [saving, setSaving] = useState<T | null>(null);
  const [finding, setFinding] = useState<T | null>(null);
  const [choosing, setChoosing] = useState<T | null>(null);

  const actions = useMemo(() => {
    const one = (rows: readonly T[]) => rows.length === 1;
    return defineActions<T>("audiobook", [
      ...(play ? [{
        id: "play", label: play.label, icon: IconPlay, group: "open" as const, priority: 0, surfaces: ["menu" as const, "context" as const, "palette" as const],
        visible: one,
        disabledReason: () => play.disabledReason ?? false,
        run: () => play.run(),
      }] : []),
      ...(open ? [{
        id: "open", label: "Open audiobook", icon: IconExternalLink, group: "open" as const, priority: 1,
        visible: one,
        run: (rows: readonly T[]) => void navigate({ to: "/audiobooks/$audiobookId", params: { audiobookId: rows[0]!.id } }),
      }] : []),
      {
        id: "save", label: "Save to list", icon: IconBookmark, group: "open", priority: 2,
        visible: (rows) => one(rows) && rows[0]!.userId === user?.id,
        run: (rows) => setSaving(rows[0]!),
      },
      ...resetProgressAction("audio", reset, resetProgress),
      ...(isAdmin ? [
        {
          id: "find", label: "Find releases", icon: IconDownload, group: "manage" as const, priority: 2, primary: !play,
          visible: one,
          run: (rows: readonly T[]) => setFinding(rows[0]!),
        },
        {
          id: "change-edition", label: "Change edition", icon: IconRefresh, group: "manage" as const, priority: 3,
          visible: one,
          run: (rows: readonly T[]) => setChoosing(rows[0]!),
        },
        {
          id: "check-chapters", label: "Check chapters by listening", icon: IconRefresh, group: "manage" as const, priority: 4,
          visible: (rows: readonly T[]) => one(rows) && rows[0]!.hasFiles,
          disabledReason: (rows: readonly T[]) => (rows[0]!.chapterCheckState === "listening" ? "Already listening" : false),
          run: async (rows: readonly T[]) => {
            assertSuccess((await listen({ variables: { audiobookId: rows[0]!.id } })).data?.listenToAudiobookChapters, "Could not start listening");
          },
          successMessage: () => "Listening for chapters. This takes a few minutes.",
        },
        {
          id: "restore-chapters", label: "Use the listed chapters", icon: IconRefresh, group: "manage" as const, priority: 5,
          visible: (rows: readonly T[]) => one(rows) && Boolean(rows[0]!.providerChapters),
          run: async (rows: readonly T[]) => {
            assertSuccess((await restore({ variables: { audiobookId: rows[0]!.id } })).data?.restoreAudiobookChapters, "Could not restore the chapters");
            onChanged?.();
          },
          successMessage: () => "Chapters restored",
        },
      ] : []),
      {
        id: "settings", label: isAdmin ? "Audiobook settings" : "Details", icon: IconSettings, group: "manage", priority: 6,
        visible: one,
        run: (rows) => settings(rows[0]!),
      },
      ...(isAdmin ? [{
        id: "remove", label: "Remove from library…", icon: IconTrash, group: "danger" as const, priority: 9,
        visible: one,
        run: (rows: readonly T[]) => settings(rows[0]!, "remove"),
      }] : []),
    ]);
  }, [isAdmin, listen, navigate, onChanged, open, play, reset, resetProgress, restore, settings, user?.id]);

  const useEdition = async (listing: Listing) => {
    if (!listing.providerId || !choosing) return;
    try {
      assertSuccess((await change({ variables: { audiobookId: choosing.id, providerId: listing.providerId } })).data?.changeAudiobookEdition, "Could not change the edition");
      toast.success("Edition changed");
      setChoosing(null);
      onChanged?.();
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const dialogs = (
    <>
      {saving ? <SaveToListDialog open onOpenChange={(next) => { if (!next) setSaving(null); }} target={{ kind: "audiobook", id: saving.id }} title={saving.title} /> : null}
      {finding ? <ReleaseSearchDialog isOpen onOpenChange={(next) => { if (!next) setFinding(null); }} query={[finding.authorName, finding.title].filter(Boolean).join(" ")} libraryId={finding.libraryId} author={finding.authorName} target={{ audiobookId: finding.id }} /> : null}
      {choosing ? (
        <ChooseListingDialog
          isOpen
          onOpenChange={(next) => { if (!next) setChoosing(null); }}
          title="Change edition"
          initialQuery={[choosing.title.split(":")[0], choosing.authorName].filter(Boolean).join(" ")}
          listings={editions.data?.audiobookEditions ?? []}
          loading={editions.loading}
          error={editions.error ? errorMessage(editions.error) : null}
          search={(query) => void searchEditions({ variables: { audiobookId: choosing.id, query } })}
          choose={useEdition}
          currentProviderId={choosing.audibleId}
        />
      ) : null}
    </>
  );
  return { actions, dialogs };
}
