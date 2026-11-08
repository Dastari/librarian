import { Link, useNavigate, type LinkProps } from "@tanstack/react-router";
import { useMutation } from "@apollo/client/react";
import { ActionContextMenu, defineActions, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { IconChevronLeft, IconChevronRight, IconClose, IconFilter, IconInfo, IconRefresh } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { Tooltip } from "glass-ux/tooltip";
import { useMemo, type ReactNode } from "react";

import { Artwork } from "@/components/ui";
import { useAddToPlaylist } from "@/features/personal/useAddToPlaylist";
import { RefreshShowDocument } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPES } from "@/lib/library-types";

export interface LibraryShowTarget {
  id: string;
  name: string;
}

/** What right-clicking one of your own shows offers, and the dialog one of those opens. */
export function useLibraryShowActions() {
  const navigate = useNavigate();
  const isAdmin = useIsAdmin();
  const [refreshShow] = useMutation(RefreshShowDocument);
  const playlist = useAddToPlaylist<LibraryShowTarget>({
    targetsOf: (rows) => rows.map((show) => ({ kind: "show" as const, id: show.id })),
    titleOf: (rows) => rows.length === 1 ? rows[0]!.name : `${rows.length} shows`,
    priority: 2,
  });
  const actions = useMemo(() => defineActions<LibraryShowTarget>("guide-show", [
    { id: "open", label: "Open show", icon: IconInfo, group: "open", priority: 1, run: ([show]) => void navigate({ to: "/shows/$showId", params: { showId: show!.id } }) },
    playlist.action,
    ...(isAdmin ? [{
      id: "refresh", label: "Refresh metadata", icon: IconRefresh, group: "manage" as const, priority: 3,
      run: ([show]: readonly LibraryShowTarget[]) => void refreshShow({ variables: { id: show!.id } }).then(({ data }) => {
        assertSuccess(data?.refreshShow, "Could not refresh metadata");
        toast.success(`Refreshed ${show!.name}`);
      }).catch((error: unknown) => toast.danger(errorMessage(error))),
    }] : []),
  ]), [navigate, isAdmin, refreshShow, playlist.action]);
  const dialog = playlist.dialog;
  return { actions, dialog };
}

export interface ScheduledShowTarget {
  tvmazeShowId: number;
  name: string;
  network?: string | null;
}

/** What right-clicking a listed show offers. Filtering by its service only applies to the grid. */
export function scheduledShowActions(network: string | null | undefined, open: (show: ScheduledShowTarget) => void, service?: { only: (name: string) => void; hide: (name: string) => void }) {
  return defineActions<ScheduledShowTarget>("guide-listing", [
    { id: "open", label: "Show details", icon: IconInfo, group: "open", priority: 1, run: ([show]) => open(show!) },
    ...(network && service ? [
      { id: "only", label: `Only show ${network}`, icon: IconFilter, group: "manage" as const, priority: 2, run: () => service.only(network) },
      { id: "hide", label: `Hide ${network}`, icon: IconClose, group: "manage" as const, priority: 3, run: () => service.hide(network) },
    ] : []),
  ]);
}

interface GuidePosterProps<T> {
  name: string;
  image: string | null | undefined;
  /** `day` fits a calendar day, `tile` a grid cell, `row` a list line and `panel` the selected day. */
  size: "day" | "tile" | "row" | "panel";
  /** A few words over the artwork: "New", "S2", "E03". */
  badge?: ReactNode;
  tooltip?: string;
  ariaLabel?: string;
  to?: LinkProps["to"];
  params?: LinkProps["params"];
  onPress?: () => void;
  actions: ActionSet<T>;
  target: T;
}

/**
 * A show's portrait wherever the guide draws one. Only its size changes: the shape, shadow, badge,
 * hover and right-click menu are the same, so a show reads as one object across its views. With no
 * `to` or `onPress` it sits inside something else that's pressed, like a calendar day.
 */
export function GuidePoster<T>({ name, image, size, badge, tooltip, ariaLabel, to, params, onPress, actions, target }: GuidePosterProps<T>) {
  const art = (
    <>
      <Artwork src={image} alt="" aspect="poster" displayWidth={size === "panel" ? 120 : 80} tint={LIBRARY_TYPES.tv.tintVar} className="rounded-[inherit]" fallback={<span className="px-1 text-center text-[0.625rem] leading-tight text-foreground/80">{name}</span>} />
      {badge ? <span className="guide-poster-badge">{badge}</span> : null}
    </>
  );
  const label = ariaLabel ?? name;
  const face = to ? <Link to={to} params={params} aria-label={label} data-focusable data-pressable className="guide-poster nav-focus tv-lift" data-size={size}>{art}</Link>
    : onPress ? <button type="button" onClick={onPress} aria-label={label} data-focusable data-pressable className="guide-poster nav-focus tv-lift text-start" data-size={size}>{art}</button>
    : <span className="guide-poster" data-size={size}>{art}</span>;
  return (
    <ActionContextMenu set={actions} targets={[target]} label={`Actions for ${name}`}>
      <span className="guide-poster-slot" data-size={size}>{tooltip ? <Tooltip content={tooltip}>{face}</Tooltip> : face}</span>
    </ActionContextMenu>
  );
}

/** The line above both guide views: which period this is, what changes it, and the view switch. */
export function GuideToolbar({ title, noun, onPrevious, onToday, onNext, previousDisabled, nextDisabled, meta, children, switcher }: {
  title: string;
  noun: string;
  onPrevious: () => void;
  onToday: () => void;
  onNext: () => void;
  previousDisabled?: boolean;
  nextDisabled?: boolean;
  meta?: ReactNode;
  children?: ReactNode;
  switcher: ReactNode;
}) {
  return (
    <div data-nav-section className="page-gutter flex min-w-0 shrink-0 flex-wrap items-center gap-2">
      <Button variant="ghost" shape="circle" size="sm" icon={<IconChevronLeft className="rtl:rotate-180" />} aria-label={`Previous ${noun}`} onClick={onPrevious} disabled={previousDisabled} data-focusable />
      <Button variant="ghost" size="sm" onClick={onToday} data-focusable>Today</Button>
      <Button variant="ghost" shape="circle" size="sm" icon={<IconChevronRight className="rtl:rotate-180" />} aria-label={`Next ${noun}`} onClick={onNext} disabled={nextDisabled} data-focusable />
      <h2 className="ms-1 me-2 truncate text-title-lg text-foreground" aria-live="polite">{title}</h2>
      {meta}
      <div className="ms-auto flex min-w-0 flex-wrap items-center justify-end gap-2">
        {children}
        {switcher}
      </div>
    </div>
  );
}
