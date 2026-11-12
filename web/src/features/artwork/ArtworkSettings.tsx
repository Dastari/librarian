import { useQuery } from "@apollo/client/react";
import { IconImage, IconLock } from "glass-ux/icons";
import { SettingRow } from "glass-ux/setting-row";
import { useState } from "react";

import { Artwork, Button, ErrorState } from "@/components/ui";
import { ArtworkSlotsDocument, type ArtworkKind } from "@/graphql/generated/graphql";
import { artworkUrl } from "@/lib/api/urls";
import { useArtworkRevisions } from "@/lib/artworkRevisions";

import { ArtworkPickerDialog } from "./ArtworkPickerDialog";
import { SLOTS, type ArtworkTarget } from "./slots";

/** Another title whose pictures belong on this tab (a show's seasons), with how its rows read. */
export interface RelatedArtwork {
  target: ArtworkTarget;
  /** "Season 2": its rows read "Season 2 poster". */
  label: string;
}

/**
 * A settings dialog's Artwork tab: each picture the title has (and its related titles', such as a
 * show's seasons), what it shows now and whether it is locked, with Change… opening the picker.
 */
export function ArtworkSettings({ target, active, related = [] }: { target: ArtworkTarget; active: boolean; related?: readonly RelatedArtwork[] }) {
  useArtworkRevisions();
  const [picking, setPicking] = useState<{ target: ArtworkTarget; kind: ArtworkKind } | null>(null);
  return (
    <div className="flex flex-col gap-2">
      <SlotRows target={target} active={active} onChange={(kind) => setPicking({ target, kind })} />
      {related.map((item) => (
        <SlotRows key={item.target.entityId} target={item.target} label={item.label} active={active} onChange={(kind) => setPicking({ target: item.target, kind })} />
      ))}
      {picking ? <ArtworkPickerDialog open target={picking.target} initialKind={picking.kind} onOpenChange={(next) => { if (!next) setPicking(null); }} /> : null}
    </div>
  );
}

function SlotRows({ target, label, active, onChange }: { target: ArtworkTarget; label?: string; active: boolean; onChange: (kind: ArtworkKind) => void }) {
  const slots = useQuery(ArtworkSlotsDocument, { variables: { entityType: target.entityType, entityId: target.entityId }, skip: !active, fetchPolicy: "cache-and-network" });
  if (slots.error && !slots.data) return <ErrorState error={slots.error} onRetry={() => void slots.refetch()} />;
  const states = new Map((slots.data?.artworkSlots ?? []).map((slot) => [slot.kind, slot]));
  return (
    <>
      {SLOTS[target.entityType].map((info) => {
        const state = states.get(info.kind);
        const description = !state ? "Loading…" : !state.hasArtwork ? `No ${info.noun} yet`
          : state.uploaded ? "Your upload. A metadata refresh keeps it."
          : state.locked ? "Chosen by you. A metadata refresh keeps it."
          : "From the provider. A metadata refresh may change it.";
        const name = label ? `${label} ${info.noun}` : info.label;
        return (
          <SettingRow key={info.kind} label={name} description={description}
            badge={state?.locked || state?.uploaded ? <IconLock size="sm" aria-label="Locked" /> : undefined}
            control={(
              <span className="flex items-center gap-3">
                <Artwork src={state?.hasArtwork ? artworkUrl(info.path[0], target.entityId, info.path[1]) : null} alt="" aspect={info.aspect}
                  imageClassName={info.contain ? "object-contain" : undefined} displayWidth={112} fallback={<IconImage />}
                  className={info.aspect === "backdrop" ? "w-28 shrink-0 rounded-md" : "w-16 shrink-0 rounded-md"} />
                <Button size="sm" variant="secondary" onPress={() => onChange(info.kind)} aria-label={`Change ${name.toLowerCase()}`}>Change…</Button>
              </span>
            )} />
        );
      })}
    </>
  );
}
