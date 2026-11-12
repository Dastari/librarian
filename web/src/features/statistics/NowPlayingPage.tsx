import { Link } from "@tanstack/react-router";
import { Badge } from "glass-ux/badge";
import { IconPlay, IconStop, IconWarning } from "glass-ux/icons";
import { Progress } from "glass-ux/progress";
import { Surface } from "glass-ux/surface";
import { toast } from "glass-ux/toast";
import { useMutation } from "@apollo/client/react";
import { useEffect, useMemo, useState } from "react";

import { Artwork, Button, EmptyState } from "@/components/ui";
import { deviceGlyph } from "@/features/devices/DeviceIdentity";
import { EntityUserListDocument, NowPlayingDocument, StopPlaybackSessionDocument } from "@/graphql/generated/graphql";
import { useIsAdmin, useSession } from "@/lib/auth/useSession";
import { formatClock } from "@/lib/format";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { cn } from "@/lib/utils";

import { describeSession, isLive, methodLabel, playState, qualityLabel, sessionProgress, type NowPlayingSession } from "./nowPlaying";

/** Re-render this often so a player that went quiet drops off without asking the server again. */
const TICK_MS = 15_000;

function useNow(interval: number) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), interval);
    return () => clearInterval(timer);
  }, [interval]);
  return now;
}

function SessionCard({ session, owner, canStop }: { session: NowPlayingSession; owner: string; canStop: boolean }) {
  const view = describeSession(session);
  const state = playState(session);
  const progress = sessionProgress(session);
  const quality = qualityLabel(session);
  const DeviceIcon = deviceGlyph(session.device?.formFactor);
  const [stop, { loading }] = useMutation(StopPlaybackSessionDocument);
  const onStop = async () => {
    try {
      assertSuccess((await stop({ variables: { id: session.id, at: new Date().toISOString() } })).data?.updatePlaybackSession, "The player could not be asked to stop");
      toast.success(`Asked ${session.device?.name ?? "the player"} to stop`);
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  return (
    <Surface as="article" material="card" radius="tile" padding="tile" className="flex min-w-0 gap-3" aria-label={`${view.title}, ${state.label.toLowerCase()} on ${session.device?.name ?? "an unknown device"}`} data-state={state.id}>
      <Link to={view.href ?? "."} disabled={!view.href} className="w-20 shrink-0 sm:w-24" tabIndex={-1} data-nav-skip>
        <Artwork src={view.artwork} alt="" aspect={view.square ? "square" : "poster"} displayWidth={96} className="rounded-[var(--radius-control)]" />
      </Link>
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div className="flex min-w-0 items-start gap-2">
          <div className="min-w-0 flex-1">
            <h3 className="truncate text-title-sm text-foreground">{view.title}</h3>
            {view.subtitle ? <p className="truncate text-body-sm text-muted">{view.subtitle}</p> : null}
          </div>
          <Badge size="sm" marker="dot" tone={state.tone}>{state.label}</Badge>
        </div>
        <p className="flex min-w-0 items-center gap-1.5 text-body-sm text-muted">
          <DeviceIcon size="sm" />
          <span className="truncate">{[owner, session.device?.name ?? "Unknown device"].join(" · ")}</span>
        </p>
        <p className="truncate text-label-sm text-muted">{[methodLabel(session.playMethod), quality].filter(Boolean).join(" · ")}</p>
        <div className="mt-auto flex min-w-0 items-end gap-3 pt-1">
          <Progress
            className="min-w-0 flex-1"
            size="sm"
            value={progress === null ? null : Math.round(progress * 100)}
            aria-label={`${view.title} progress`}
            meta={session.duration ? `${formatClock(session.currentPosition)} of ${formatClock(session.duration)}` : formatClock(session.currentPosition)}
            showValue={false}
          />
          {canStop && !session.stopRequestedAt ? (
            <Button variant="ghost" size="sm" isPending={loading} onPress={() => void onStop()} aria-label={`Stop ${view.title} on ${session.device?.name ?? "this player"}`}>
              <IconStop /> Stop
            </Button>
          ) : null}
        </div>
      </div>
    </Surface>
  );
}

/**
 * What is playing right now, Plex's "Now Playing": members see their own players, administrators
 * everyone's. It updates as the players report; a player that stops reporting drops off.
 */
export function NowPlayingPage() {
  const isAdmin = useIsAdmin();
  const { user } = useSession();
  // A day back is plenty: anything quieter than a few minutes is hidden below.
  const [since] = useState(() => new Date(Date.now() - 24 * 3600_000).toISOString());
  const sessions = useQuery(NowPlayingDocument, { variables: { since } });
  const users = useQuery(EntityUserListDocument, { variables: { page: { limit: 100, offset: 0 } }, skip: !isAdmin });
  const now = useNow(TICK_MS);
  const names = useMemo(() => new Map((users.data?.users.edges ?? []).map(({ node }) => [node.id, node.displayName || node.username])), [users.data]);
  const live = useMemo(() => (sessions.data ?? sessions.previousData)?.playbackSessions.edges.map((edge) => edge.node).filter((session) => isLive(session, now)) ?? [], [sessions.data, sessions.previousData, now]);
  const ownerName = (session: NowPlayingSession) => (session.userId === user?.id ? "You" : names.get(session.userId) ?? "Someone");

  if (sessions.error && live.length === 0) {
    return <EmptyState icon={IconWarning} title="Could not load what is playing" description={errorMessage(sessions.error)} action={<Button onPress={() => void sessions.refetch()}>Retry</Button>} />;
  }
  if (!sessions.data && sessions.loading) {
    return <div className="grid min-w-0 gap-3 [grid-template-columns:repeat(auto-fill,minmax(min(100%,22rem),1fr))]" aria-busy="true">{[0, 1].map((key) => <Surface key={key} material="card" radius="tile" padding="tile" className="h-36 animate-pulse" />)}</div>;
  }
  if (live.length === 0) {
    return <EmptyState icon={IconPlay} title="Nothing is playing" description={isAdmin ? "Whatever anyone plays on this server shows up here as it starts." : "What you play on any of your devices shows up here as it starts."} />;
  }
  return (
    <section aria-label="Now playing" data-nav-section className="min-w-0">
      <p className="sr-only" aria-live="polite">{live.length === 1 ? "1 player" : `${live.length} players`}</p>
      <div data-nav-group className={cn("grid min-w-0 gap-3 [grid-template-columns:repeat(auto-fill,minmax(min(100%,22rem),1fr))]")}>
        {live.map((session) => (
          <SessionCard key={session.id} session={session} owner={ownerName(session)} canStop={isAdmin || session.userId === user?.id} />
        ))}
      </div>
    </section>
  );
}
