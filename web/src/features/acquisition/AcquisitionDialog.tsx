import { useMutation, useQuery } from "@apollo/client/react";
import { Label, Select, toast } from "@heroui/react";
import { IconSearch } from "@tabler/icons-react";
import { useEffect, useState } from "react";

import { Button, Dialog, GlassSegmented, GlassSwitch, SelectList } from "@/components/ui";
import {
  EntityAlbumUpdateDocument,
  EntityAudiobookUpdateDocument,
  EntityLibraryGetDocument,
  EntityMovieUpdateDocument,
  EntityShowUpdateDocument,
  QualityProfilesListDocument,
  SearchMissingDocument,
  type AutoDownloadMode,
} from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";

import { MODE_HELP, MODE_SEGMENTS } from "./mode";

export type AcquisitionKind = "show" | "album" | "audiobook" | "movie";

export interface AcquisitionTarget {
  kind: AcquisitionKind;
  id: string;
  title: string;
  libraryId: string;
  /** Shows, albums and audiobooks. */
  autoDownloadMode?: AutoDownloadMode;
  /** Movies. */
  monitored?: boolean;
  wanted?: boolean;
  qualityProfileId?: string | null;
}

interface AcquisitionDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  target: AcquisitionTarget;
  onSaved?: () => void;
}

const INHERIT = "inherit";

/**
 * Per-title download settings: how much is fetched automatically and which quality profile
 * decides what is acceptable. Opened from the status chip in a detail page header.
 */
export function AcquisitionDialog({ isOpen, onOpenChange, target, onSaved }: AcquisitionDialogProps) {
  const [mode, setMode] = useState<AutoDownloadMode>(target.autoDownloadMode ?? "NONE");
  const [monitored, setMonitored] = useState(target.monitored ?? false);
  const [wanted, setWanted] = useState(target.wanted ?? false);
  const [profileKey, setProfileKey] = useState<string>(target.qualityProfileId ?? INHERIT);

  const library = useQuery(EntityLibraryGetDocument, { variables: { id: target.libraryId }, skip: !isOpen || !target.libraryId });
  const profiles = useQuery(QualityProfilesListDocument, { skip: !isOpen });
  const [updateShow, showState] = useMutation(EntityShowUpdateDocument);
  const [updateAlbum, albumState] = useMutation(EntityAlbumUpdateDocument);
  const [updateAudiobook, audiobookState] = useMutation(EntityAudiobookUpdateDocument);
  const [updateMovie, movieState] = useMutation(EntityMovieUpdateDocument);
  const [searchMissing, { loading: searching }] = useMutation(SearchMissingDocument);
  const saving = showState.loading || albumState.loading || audiobookState.loading || movieState.loading;

  useEffect(() => {
    if (!isOpen) return;
    setMode(target.autoDownloadMode ?? "NONE");
    setMonitored(target.monitored ?? false);
    setWanted(target.wanted ?? false);
    setProfileKey(target.qualityProfileId ?? INHERIT);
  }, [isOpen, target.autoDownloadMode, target.monitored, target.wanted, target.qualityProfileId]);

  const profileList = profiles.data?.qualityProfiles.edges.map((edge) => edge.node) ?? [];
  const libraryProfileId = library.data?.library?.qualityProfileId ?? null;
  const libraryProfileName = profileList.find((profile) => profile.id === libraryProfileId)?.name ?? profileList.find((profile) => profile.isDefault)?.name ?? "default profile";
  const options = [
    { key: INHERIT, label: `Inherit from library (${libraryProfileName})` },
    ...profileList.map((profile) => ({ key: profile.id, label: profile.name, description: profile.mediaKind === "VIDEO" ? profile.allowedResolutions.join(" / ") || "Any resolution" : profile.allowedAudioFormats.join(" / ") || "Any format" })),
  ];

  const save = async () => {
    const qualityProfileId = profileKey === INHERIT ? null : profileKey;
    try {
      if (target.kind === "movie") {
        assertSuccess((await updateMovie({ variables: { id: target.id, input: { monitored, wanted, qualityProfileId } } })).data?.updateMovie, "Could not save");
      } else if (target.kind === "show") {
        assertSuccess((await updateShow({ variables: { id: target.id, input: { autoDownload: mode !== "NONE", autoDownloadMode: mode, qualityProfileId } } })).data?.updateShow, "Could not save");
      } else if (target.kind === "album") {
        assertSuccess((await updateAlbum({ variables: { id: target.id, input: { autoDownload: mode !== "NONE", autoDownloadMode: mode, qualityProfileId } } })).data?.updateAlbum, "Could not save");
      } else {
        assertSuccess((await updateAudiobook({ variables: { id: target.id, input: { autoDownload: mode !== "NONE", autoDownloadMode: mode, qualityProfileId } } })).data?.updateAudiobook, "Could not save");
      }
      toast.success("Download settings saved");
      onSaved?.();
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  const searchNow = async () => {
    const scope =
      target.kind === "show" ? { showId: target.id } : target.kind === "movie" ? { movieId: target.id } : target.kind === "album" ? { albumId: target.id } : { audiobookId: target.id };
    try {
      const { data } = await searchMissing({ variables: { input: scope } });
      const result = data?.searchMissing;
      if (result?.success) toast.success(result.searched === 0 ? "Nothing is missing here" : `Searched ${result.searched}, grabbed ${result.queued}`);
      else toast.warning(result?.error ?? "The search did not run");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title="Downloads"
      description={target.title}
      size="sm"
      footer={
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="primary" onPress={() => void save()} isPending={saving}>
            Save
          </Button>
        </>
      }
    >
      <div className="flex flex-col gap-5">
        {target.kind === "movie" ? (
          <div className="flex flex-col gap-3">
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <p className="text-body-sm text-foreground">Monitored</p>
                <p className="text-label-sm text-muted">Keep watching for a better release once the movie is in the library.</p>
              </div>
              <GlassSwitch checked={monitored} onChange={setMonitored} ariaLabel="Monitored" />
            </div>
            <div className="flex items-start justify-between gap-4">
              <div className="min-w-0">
                <p className="text-body-sm text-foreground">Wanted</p>
                <p className="text-label-sm text-muted">Search sources for this movie until a file arrives.</p>
              </div>
              <GlassSwitch checked={wanted} onChange={(value) => { setWanted(value); if (value) setMonitored(true); }} ariaLabel="Wanted" />
            </div>
          </div>
        ) : (
          <div className="flex flex-col gap-2">
            <p className="text-body-sm text-foreground">Automatic downloads</p>
            <GlassSegmented<AutoDownloadMode> ariaLabel="Automatic downloads" value={mode} onChange={setMode} items={MODE_SEGMENTS} className="max-w-full" />
            <p className="text-label-sm text-muted">{MODE_HELP[mode]}</p>
          </div>
        )}

        <Select aria-label="Quality profile" selectedKey={profileKey} onSelectionChange={(key) => key !== null && setProfileKey(String(key))} fullWidth>
          <Label>Quality profile</Label>
          <Select.Trigger>
            <Select.Value />
            <Select.Indicator />
          </Select.Trigger>
          <Select.Popover>
            <SelectList options={options} />
          </Select.Popover>
        </Select>

        <div className="flex flex-col gap-2 border-t border-separator pt-4">
          <Button variant="secondary" fullWidth onPress={() => void searchNow()} isPending={searching}>
            <IconSearch size={16} /> Search now
          </Button>
          <p className="text-label-sm text-muted">Searches your sources for everything still missing here, ignoring the usual retry delay.</p>
        </div>
      </div>
    </Dialog>
  );
}
