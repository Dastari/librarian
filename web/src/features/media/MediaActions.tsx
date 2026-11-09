import { ActionBar, usePaletteActions, type ActionContext, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { IconPlay } from "glass-ux/icons";
import { useLayoutEffect, useMemo, useRef, useState } from "react";

import { FindReleasesButton } from "@/features/recommendations/FindReleasesButton";

/** What an action needs from its surface when the hero runs one itself (Find releases only reads its targets). */
const HERO_CONTEXT = {
  surface: "bar",
  navigate: () => {},
  toast: () => {},
  openWindow: () => {},
} as unknown as ActionContext;

/**
 * A title's own actions in its page hero. Playback leads the row as the large primary button; the
 * rest are the title's one action list, which folds into More instead of wrapping. With
 * `recommended`, the title's "Find releases" action gets its own button carrying a badge with the
 * number of releases Librarian recommends for it (the action stays in menus and the palette).
 */
export function MediaActions<T>({ target, label, actions, play, recommended }: {
  target: T;
  label: string;
  actions: ActionSet<T>;
  play?: { label: string; run: () => void; disabledReason?: string };
  recommended?: number;
}) {
  const targets = useMemo(() => [target], [target]);
  usePaletteActions(actions, targets, label);
  const find = recommended === undefined ? undefined : actions.actions.find((action) => action.id === "find" && (action.visible?.(targets) ?? true));
  const bar = useMemo<ActionSet<T>>(() => (find ? { entity: actions.entity, actions: actions.actions.filter((action) => action.id !== "find") } : actions), [actions, find]);
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
  }, [play, find]);
  return (
    <div ref={row} className="media-actions flex min-w-0 items-center gap-3">
      {play || find ? (
        <div ref={lead} className="flex shrink-0 items-center gap-3">
          {play ? <Button variant="primary" size="lg" icon={<IconPlay />} onClick={play.disabledReason ? undefined : play.run} disabledReason={play.disabledReason} data-tv-autofocus data-focusable className="media-play">{play.label}</Button> : null}
          {find ? <FindReleasesButton count={recommended ?? 0} variant={play ? "secondary" : "primary"} onPress={() => void find.run?.(targets, HERO_CONTEXT)} /> : null}
        </div>
      ) : null}
      <div className="flex min-w-0 flex-[0_1_auto]">
        <ActionBar set={bar} targets={targets} label={label} maxButtons={find ? 2 : 3} available={room} className="min-w-0" />
      </div>
    </div>
  );
}
