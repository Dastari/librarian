import { ActionBar, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { TextCell } from "glass-ux/data-table";
import { IconBookmark, IconPause, IconPlay } from "glass-ux/icons";
import { MeterCell } from "glass-ux/meter-cell";
import { toast } from "glass-ux/toast";

import { errorMessage } from "@/lib/graphql/errors";

export type RowPlayState = "playing" | "resume" | "finished" | "ready";

/** The animated bars that mean this is what's playing. */
export function PlayingBars() {
  return (
    <span className="playing-bars" aria-hidden="true">
      <span className="equalizer-bar" />
      <span className="equalizer-bar" style={{ animationDelay: "-0.3s" }} />
      <span className="equalizer-bar" style={{ animationDelay: "-0.6s" }} />
    </span>
  );
}

const LABEL: Record<RowPlayState, string> = { playing: "Pause", resume: "Resume", finished: "Play again", ready: "Play" };

interface RowActionsProps<T> {
  set: ActionSet<T>;
  target: T;
  label: string;
  /** How this row plays, when it has a file. */
  play?: { state: RowPlayState; onPress: () => void; disabledReason?: string };
  /** Marks a row with no file as wanted. */
  want?: { wanted: boolean; onPress: () => Promise<unknown> };
}

/**
 * A row's actions: a ghost icon for the main thing, then the menu. The icon's slot is always there,
 * so every row's menu lines up, whatever state the row is in.
 */
export function RowActions<T>({ set, target, label, play, want }: RowActionsProps<T>) {
  return (
    <div className="row-actions table-row-actions">
      <span className="row-actions-slot">
        {play ? (
          <Button
            variant="ghost"
            shape="circle"
            size="sm"
            aria-label={LABEL[play.state]}
            tooltip={LABEL[play.state]}
            disabledReason={play.disabledReason}
            onClick={play.disabledReason ? undefined : play.onPress}
            className="row-play"
            data-state={play.state}
            data-focusable
            icon={
              play.state === "playing" ? (
                <>
                  <PlayingBars />
                  <IconPause className="row-play-pause" />
                </>
              ) : (
                // Finished rows keep the play icon: a tick doesn't read as "play again" on touch,
                // and the progress column already says it's done.
                <IconPlay />
              )
            }
          />
        ) : want ? (
          <Button variant="ghost" shape="circle" size="sm" aria-label={want.wanted ? "Wanted" : "Want"} tooltip={want.wanted ? "Wanted" : "Want"} aria-pressed={want.wanted} onClick={() => { want.onPress().catch((error: unknown) => toast.danger(errorMessage(error))); }} className={want.wanted ? "text-brand" : undefined} data-focusable icon={<IconBookmark />} />
        ) : null}
      </span>
      <ActionBar set={set} targets={[target]} label={label} maxButtons={0} />
    </div>
  );
}

/** For a table using `RowActions`: without this its own ⋯ appears beside ours when a row is hovered. */
export const noRowMenu = () => null;

/** How far through a row's media this person is. */
export function PlayProgressCell({ fraction, label, available }: { fraction: number; label: string; available: boolean }) {
  if (!available) return <TextCell muted>—</TextCell>;
  if (fraction <= 0) return <TextCell muted>Not started</TextCell>;
  const finished = fraction >= 0.98;
  return <MeterCell label={label} value={fraction * 100} max={100} valueText={finished ? "Done" : `${Math.round(fraction * 100)}%`} warningAt={2} dangerAt={2} tone="data" />;
}
