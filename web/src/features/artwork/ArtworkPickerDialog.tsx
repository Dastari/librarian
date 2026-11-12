import { useMutation, useQuery } from "@apollo/client/react";
import { FileDrop } from "glass-ux/file-drop";
import { IconImage, IconLock, IconRestore, IconUnlock, IconUpload } from "glass-ux/icons";
import { Notice } from "glass-ux/notice";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";

import { Button, Dialog, EmptyState, ErrorState, PosterCard, Spinner } from "@/components/ui";
import { SegmentTabs } from "@/components/ui/Tabs";
import {
  ArtworkOptionsDocument,
  ChooseArtworkDocument,
  RevertArtworkDocument,
  type ArtworkKind,
  type ArtworkOptionsQuery,
} from "@/graphql/generated/graphql";
import { artworkUrl } from "@/lib/api/urls";
import { bumpArtwork } from "@/lib/artworkRevisions";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { artworkEntityPath, slotInfo, SLOTS, UPLOAD_ACCEPT, UPLOAD_MAX_BYTES, type ArtworkTarget, type SlotInfo } from "./slots";
import { uploadArtwork } from "./upload";
import { useUnlockArtwork } from "./useUnlockArtwork";

type Option = NonNullable<ArtworkOptionsQuery["artworkOptions"]>["options"][number];
type Mode = "provider" | "upload";

const REFETCH = ["ArtworkOptions", "ArtworkSlots"];

const languages = typeof Intl !== "undefined" && "DisplayNames" in Intl ? new Intl.DisplayNames(undefined, { type: "language" }) : null;

/** "English", or "No text" for textless art. */
export function languageName(code: string | null | undefined): string {
  if (!code) return "No text";
  try {
    return languages?.of(code) ?? code.toUpperCase();
  } catch {
    return code.toUpperCase();
  }
}

/** The facts under a picture: language, size and votes. */
export function optionFacts(option: Pick<Option, "language" | "width" | "height" | "voteAverage" | "voteCount" | "source" | "label">, showsLanguage: boolean): string {
  return [
    option.label ? option.source : null,
    showsLanguage ? languageName(option.language) : null,
    option.width && option.height ? `${option.width}×${option.height}` : null,
    option.voteCount ? `★ ${(option.voteAverage ?? 0).toFixed(1)} (${option.voteCount})` : null,
  ].filter(Boolean).join(" · ");
}

/**
 * Change a title's pictures: one of the provider's (posters, backdrops, logos, covers, stills,
 * with their language, size and votes), or a file of your own. Whatever is chosen is locked, so a
 * metadata refresh keeps it; Unlock lets refreshes change it again and Revert puts back the
 * provider's choice now. Tiles are buttons in a remote-navigable grid; nothing is hover-only.
 */
export function ArtworkPickerDialog({ target, open, onOpenChange, initialKind }: {
  target: ArtworkTarget;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialKind?: ArtworkKind;
}) {
  const slots = SLOTS[target.entityType];
  const [kind, setKind] = useState<ArtworkKind>(initialKind ?? slots[0]!.kind);
  useEffect(() => { if (open) setKind(initialKind ?? slots[0]!.kind); }, [open, initialKind, slots]);
  const info = slotInfo(target.entityType, kind);

  return (
    <Dialog isOpen={open} onOpenChange={onOpenChange} size="xl" title={`Artwork for ${target.title}`}
      description="What you choose is locked, so a metadata refresh keeps it.">
      <div className="flex min-w-0 flex-col gap-4">
        {slots.length > 1 ? (
          <SegmentTabs ariaLabel="Picture" items={slots.map((slot) => ({ key: slot.kind, label: slot.label }))} selected={kind} onSelect={(next) => setKind(next as ArtworkKind)} />
        ) : null}
        {open ? <SlotPicker key={`${target.entityId}:${kind}`} target={target} info={info} onDone={() => onOpenChange(false)} /> : null}
      </div>
    </Dialog>
  );
}

function SlotPicker({ target, info, onDone }: { target: ArtworkTarget; info: SlotInfo; onDone: () => void }) {
  const variables = { entityType: target.entityType, entityId: target.entityId, kind: info.kind };
  const query = useQuery(ArtworkOptionsDocument, { variables, fetchPolicy: "cache-and-network", notifyOnNetworkStatusChange: true });
  const result = query.data?.artworkOptions;
  const slot = result?.slot;
  const options = useMemo(() => result?.options ?? [], [result]);
  const hasProvider = slot?.hasProvider ?? target.entityType !== "PLAYLIST";
  const [mode, setMode] = useState<Mode>(hasProvider ? "provider" : "upload");
  useEffect(() => { if (slot && !slot.hasProvider) setMode("upload"); }, [slot]);

  const current = options.find((option) => option.isCurrent)?.url ?? null;
  const [selected, setSelected] = useState<string | null>(null);
  const chosen = selected ?? current;
  const [file, setFile] = useState<File | null>(null);
  const [fileError, setFileError] = useState<string>();
  const preview = useMemo(() => (file && typeof URL.createObjectURL === "function" ? URL.createObjectURL(file) : null), [file]);
  useEffect(() => () => { if (preview) URL.revokeObjectURL(preview); }, [preview]);

  const [choose, choosing] = useMutation(ChooseArtworkDocument, { refetchQueries: REFETCH });
  const [revert, reverting] = useMutation(RevertArtworkDocument, { refetchQueries: REFETCH });
  const { unlock, unlocking } = useUnlockArtwork(target);
  const [uploading, setUploading] = useState(false);
  const busy = choosing.loading || reverting.loading || unlocking || uploading;

  const changed = (message: string) => {
    bumpArtwork(artworkEntityPath(target.entityType), target.entityId);
    toast.success(message);
  };
  const run = async (work: () => Promise<void>) => {
    try { await work(); } catch (error) { toast.danger(errorMessage(error)); }
  };
  const useChosen = () => run(async () => {
    if (!chosen) return;
    const { data } = await choose({ variables: { ...variables, url: chosen } });
    assertSuccess(data?.chooseArtwork, `Could not use that ${info.noun}`);
    changed(`New ${info.noun} chosen and locked`);
    onDone();
  });
  // The server's reason (too small, not a picture) belongs beside the file, not in a toast.
  const upload = async () => {
    if (!file) return;
    setUploading(true);
    setFileError(undefined);
    try {
      await uploadArtwork(target, info.kind, file);
    } catch (error) {
      setFileError(errorMessage(error));
      setUploading(false);
      return;
    }
    setUploading(false);
    void query.refetch().catch(() => {});
    changed(`Your ${info.noun} is uploaded and locked`);
    onDone();
  };
  const revertSlot = () => run(async () => {
    const { data } = await revert({ variables });
    assertSuccess(data?.revertArtwork, `Could not put the provider's ${info.noun} back`);
    changed(hasProvider && info.kind !== "LOGO" ? `The provider's ${info.noun} is back` : `${info.label} removed`);
  });
  const unlockSlot = () => run(async () => {
    if (!slot) return;
    await unlock(info.kind, slot.lockedFields);
    await query.refetch();
    toast.success(`${info.label} unlocked: a metadata refresh may change it`);
  });

  // A logo and a playlist's cover have no lock of their own: only a choice ever fills them.
  const lockable = hasProvider && info.kind !== "LOGO";
  const canRevert = Boolean(slot && (slot.locked || slot.uploaded));
  const textOnArt = info.kind === "POSTER" || info.kind === "LOGO" || info.kind === "BACKDROP";

  return (
    <div className="flex min-w-0 flex-col gap-4">
      <div className="flex min-w-0 flex-wrap items-center gap-3">
        {hasProvider ? (
          <SegmentTabs ariaLabel="Where the picture comes from" items={[{ key: "provider", label: "From the provider", icon: IconImage }, { key: "upload", label: "Upload your own", icon: IconUpload }]} selected={mode} onSelect={(next) => setMode(next as Mode)} />
        ) : null}
        <LockState slot={slot} info={info} lockable={lockable} />
        <span className="ms-auto flex flex-wrap gap-2">
          {lockable && slot?.locked && !slot.uploaded ? <Button size="sm" variant="ghost" onPress={() => void unlockSlot()} isDisabled={busy} isPending={unlocking}><IconUnlock size="md" /> Unlock</Button> : null}
          {canRevert ? (
            <Button size="sm" variant="secondary" onPress={() => void revertSlot()} isDisabled={busy} isPending={reverting.loading}>
              <IconRestore size="md" /> {lockable ? "Revert to provider" : `Remove ${info.noun}`}
            </Button>
          ) : null}
        </span>
      </div>

      {mode === "provider" ? (
        <ProviderGrid query={query} options={options} unavailable={result?.unavailable ?? []} info={info} chosen={chosen} onChoose={setSelected} showsLanguage={textOnArt} onUpload={() => setMode("upload")} />
      ) : (
        <div className="grid min-w-0 gap-4 sm:grid-cols-[minmax(0,1fr)_12rem]">
          <FileDrop accept={UPLOAD_ACCEPT} maxSize={UPLOAD_MAX_BYTES} multiple={false} aria-label={`Choose a ${info.noun} to upload`}
            hint="JPEG, PNG or WebP, up to 20 MB, at least 100 pixels on each side"
            invalid={Boolean(fileError)} disabled={busy}
            onFiles={(accepted, rejected) => {
              setFile(accepted[0] ?? null);
              setFileError(rejected[0] ? `${rejected[0].file.name}: ${rejected[0].reason === "size" ? "larger than 20 MB" : "not a JPEG, PNG or WebP picture"}` : undefined);
            }} />
          <div className="flex min-w-0 flex-col gap-2">
            <PosterCard width="fill" aspect={info.aspect} contain={info.contain} image={preview ?? (slot?.hasArtwork ? artworkUrl(info.path[0], target.entityId, info.path[1]) : null)}
              title={file ? file.name : slot?.hasArtwork ? "Shown now" : `No ${info.noun} yet`} meta={file ? `${Math.max(1, Math.round(file.size / 1024))} KB` : undefined} />
          </div>
          {fileError ? <p role="alert" className="text-body-sm text-danger sm:col-span-2">{fileError}</p> : null}
        </div>
      )}

      <div className="flex justify-end gap-2">
        <Button variant="ghost" onPress={onDone} isDisabled={busy}>Close</Button>
        {mode === "provider" ? (
          <Button variant="primary" onPress={() => void useChosen()} isPending={choosing.loading} isDisabled={busy || !chosen || chosen === current}>
            <IconLock size="md" /> Use this {info.noun}
          </Button>
        ) : (
          <Button variant="primary" onPress={() => void upload()} isPending={uploading} isDisabled={busy || !file}>
            <IconUpload size="md" /> Upload and use
          </Button>
        )}
      </div>
    </div>
  );
}

function LockState({ slot, info, lockable }: { slot: NonNullable<ArtworkOptionsQuery["artworkOptions"]>["slot"] | undefined; info: SlotInfo; lockable: boolean }) {
  if (!slot) return null;
  const text = !slot.hasArtwork ? `No ${info.noun} yet`
    : slot.uploaded ? `Your upload, locked`
    : slot.locked ? (lockable ? "Locked: a metadata refresh keeps it" : "Chosen by you")
    : "Follows the provider";
  return (
    <span className="flex items-center gap-1.5 text-label-sm text-muted" data-testid="artwork-lock-state">
      {slot.locked || slot.uploaded ? <IconLock size="sm" aria-hidden /> : <IconUnlock size="sm" aria-hidden />}
      {text}
    </span>
  );
}

function ProviderGrid({ query, options, unavailable, info, chosen, onChoose, showsLanguage, onUpload }: {
  query: { loading: boolean; error?: unknown; data?: unknown; refetch: () => Promise<unknown> };
  options: readonly Option[];
  unavailable: readonly string[];
  info: SlotInfo;
  chosen: string | null;
  onChoose: (url: string) => void;
  showsLanguage: boolean;
  onUpload: () => void;
}) {
  if (query.loading && !query.data) {
    return <div className="grid h-48 place-items-center" role="status" aria-label="Asking the provider"><Spinner size={28} /></div>;
  }
  if (query.error && !query.data) return <ErrorState error={query.error} onRetry={() => void query.refetch()} />;
  const notSetUp = unavailable.filter((name) => name.endsWith("isn't set up"));
  const failed = unavailable.filter((name) => !name.endsWith("isn't set up"));
  return (
    <div className="flex min-w-0 flex-col gap-3">
      {failed.length ? (
        <Notice tone="warning" action={<Button size="sm" variant="ghost" onPress={() => void query.refetch()}>Try again</Button>}>
          Couldn't reach {failed.join(", ")}, so its pictures aren't listed.
        </Notice>
      ) : null}
      {notSetUp.length ? <Notice tone="info">{notSetUp.join(", ")}: add its API key under Settings › Metadata for more pictures.</Notice> : null}
      {options.length === 0 ? (
        <EmptyState icon={IconImage} title={`The provider has no ${info.noun}s for this`} description="You can upload your own instead."
          action={<Button size="sm" variant="secondary" onPress={onUpload}><IconUpload size="md" /> Upload your own</Button>} className="border-none" />
      ) : (
        <div data-nav-group role="group" aria-label={`${info.label}s from the provider`}
          className="grid gap-x-3 gap-y-5"
          style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${info.aspect === "poster" ? "8.5rem" : info.aspect === "square" ? "9.5rem" : "13rem"}, 1fr))` }}>
          {options.map((option) => (
            <PosterCard key={option.url} width="fill" aspect={info.aspect} contain={info.contain} image={option.previewUrl}
              title={option.label ?? option.source} meta={optionFacts(option, showsLanguage) || undefined}
              badge={option.isCurrent ? "Shown now" : option.isDefault ? "Provider's pick" : undefined}
              selected={chosen === option.url} onPress={() => onChoose(option.url)} />
          ))}
        </div>
      )}
    </div>
  );
}
