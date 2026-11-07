import { ActionBar, usePaletteActions, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { IconPlay } from "glass-ux/icons";
import { useLayoutEffect, useMemo, useRef, useState } from "react";

/**
 * A title's own actions in its page hero. Playback leads the row as the large primary button; the
 * rest are the title's one action list, which folds into More instead of wrapping.
 */
export function MediaActions<T>({ target, label, actions, play }: {
  target: T;
  label: string;
  actions: ActionSet<T>;
  play?: { label: string; run: () => void; disabledReason?: string };
}) {
  const targets = useMemo(() => [target], [target]);
  usePaletteActions(actions, targets, label);
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
    <div ref={row} className="media-actions flex min-w-0 items-center gap-3">
      {play ? (
        <div ref={lead} className="shrink-0">
          <Button variant="primary" size="lg" icon={<IconPlay />} onClick={play.disabledReason ? undefined : play.run} disabledReason={play.disabledReason} data-tv-autofocus data-focusable className="media-play">{play.label}</Button>
        </div>
      ) : null}
      <div className="flex min-w-0 flex-[0_1_auto]">
        <ActionBar set={actions} targets={targets} label={label} maxButtons={3} available={room} className="min-w-0" />
      </div>
    </div>
  );
}
