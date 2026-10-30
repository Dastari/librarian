import { ActionBar, defineActions, usePaletteActions, type ActionDefinition } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { IconBookmark, IconDownload, IconPlay, IconPlus, IconRefresh, IconSettings, IconTrash } from "glass-ux/icons";
import { useLayoutEffect, useMemo, useRef, useState } from "react";
import { SaveToListDialog } from "@/features/personal/SaveToListDialog";
import type { PersonalTarget } from "@/features/personal/types";

interface MediaActionsProps {
  id: string;
  label: string;
  personal?: PersonalTarget;
  play?: { label: string; run: () => void; disabledReason?: string };
  wanted?: { value: boolean; run: () => Promise<void> };
  enqueue?: () => void;
  find?: () => void;
  settings?: () => void;
  refresh?: { pending: boolean; run: () => Promise<void> };
  remove?: () => void;
}

/**
 * Media hero actions. Playback leads the row as the large primary button, the way media
 * players present it; every other action, and Play itself in menus and the palette, reads
 * the same action set. The bar after Play is told how much room is left so it folds into
 * More instead of wrapping, and stays left-aligned beside the button.
 */
export function MediaActions({ id, label, personal, play, wanted, enqueue, find, settings, refresh, remove }: MediaActionsProps) {
  const [saving, setSaving] = useState(false);
  const targets = useMemo(() => [id], [id]);
  const set = useMemo(() => {
    const actions: ActionDefinition<string>[] = [];
    if (play) actions.push({ id: "play", label: play.label, icon: IconPlay, group: "open", surfaces: ["menu", "context", "palette"], disabledReason: () => play.disabledReason, run: play.run });
    if (wanted) actions.push({ id: "wanted", label: wanted.value ? "Wanted" : "Want", icon: IconBookmark, group: "manage", priority: 1, run: wanted.run, successMessage: () => wanted.value ? "No longer wanted" : "Marked as wanted" });
    if (enqueue) actions.push({ id: "enqueue", label: "Add to queue", icon: IconPlus, group: "open", priority: 1, run: enqueue });
    if (personal) actions.push({ id: "save", label: "Save to list", icon: IconBookmark, group: "open", priority: 1, run: () => setSaving(true) });
    if (find) actions.push({ id: "find", label: "Find releases", icon: IconDownload, group: "manage", priority: 2, primary: !play, run: find });
    if (settings) actions.push({ id: "settings", label: "Download settings", icon: IconSettings, group: "manage", run: settings });
    if (refresh) actions.push({ id: "refresh", label: "Refresh metadata", icon: IconRefresh, group: "manage", disabledReason: () => refresh.pending ? "Metadata refresh is in progress" : false, run: refresh.run, successMessage: () => "Metadata refreshed" });
    // Existing page confirmation owns removal and pending state.
    if (remove) actions.push({ id: "remove", label: "Remove from library", icon: IconTrash, group: "danger", run: remove });
    return defineActions("media", actions);
  }, [play, wanted, personal, enqueue, find, settings, refresh, remove]);
  usePaletteActions(set, targets, label);

  const row = useRef<HTMLDivElement>(null);
  const lead = useRef<HTMLDivElement>(null);
  const [room, setRoom] = useState<number>();
  useLayoutEffect(() => {
    const element = row.current;
    if (!element) return;
    const measure = () => {
      const gap = parseFloat(getComputedStyle(element).columnGap) || 0;
      const used = lead.current ? lead.current.getBoundingClientRect().width + gap : 0;
      setRoom(Math.max(0, Math.floor(element.getBoundingClientRect().width - used)));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    if (lead.current) observer.observe(lead.current);
    return () => observer.disconnect();
  }, [play]);

  return (
    <>
      <div ref={row} className="media-actions flex min-w-0 items-center gap-3">
        {play ? (
          <div ref={lead} className="shrink-0">
            <Button variant="primary" size="lg" icon={<IconPlay />} onClick={play.disabledReason ? undefined : play.run} disabledReason={play.disabledReason} data-tv-autofocus data-focusable className="media-play">
              {play.label}
            </Button>
          </div>
        ) : null}
        {/* The bar right-aligns itself; a shrink-to-fit box keeps it beside Play. */}
        <div className="flex min-w-0 flex-[0_1_auto]">
          <ActionBar set={set} targets={targets} label={label} maxButtons={3} available={room} className="min-w-0" />
        </div>
      </div>
      {personal && saving ? <SaveToListDialog open={saving} onOpenChange={setSaving} target={personal} title={label} /> : null}
    </>
  );
}
