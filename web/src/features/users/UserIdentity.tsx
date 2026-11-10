import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState } from "glass-ux/entity";
import type { ReactNode } from "react";

import { artworkUrl, sizedArtwork } from "@/lib/api/urls";
import { ENTITY_KINDS } from "@/lib/entities/kinds";
import { formatRelative } from "@/lib/format";

/** What the identity needs of a person (the `UserFields` fragment has it all). */
export interface UserEntity {
  id: string;
  username: string;
  displayName?: string | null;
  email?: string | null;
  role: string;
  avatarUrl?: string | null;
  isActive?: boolean;
  lastLoginAt?: string | null;
  twoFactorEnabledAt?: string | null;
  mustChangePassword?: boolean;
}

/** The name people see: the display name, or the sign-in name. */
export const userName = (user: Pick<UserEntity, "displayName" | "username">) => user.displayName?.trim() || user.username;

/** The uploaded photo, when there is one (`/api/artwork/user/{id}/avatar`). */
export function userAvatarSrc(user: Pick<UserEntity, "id" | "avatarUrl">, width = 160): string | undefined {
  return user.avatarUrl ? sizedArtwork(artworkUrl("user", user.id, "avatar"), width) : undefined;
}

export function userState(user: UserEntity, { self = false }: { self?: boolean } = {}): EntityState {
  if (user.isActive === false) return { id: "disabled", label: "Sign-in disabled", tone: "danger" };
  if (user.mustChangePassword) return { id: "temporary", label: "Temporary password", tone: "warning" };
  if (self) return { id: "self", label: "You", tone: "accent" };
  return user.role === "admin" ? { id: "admin", label: "Admin", tone: "accent" } : { id: "member", label: "Member", tone: "neutral" };
}

export function userIdentityProps<T extends UserEntity>(user: T, { actions, self }: { actions?: ActionSet<T>; self?: boolean } = {}): EntityIdentityProps<T> {
  const Icon = ENTITY_KINDS.user.icon;
  const name = userName(user);
  return {
    kind: "User",
    name,
    icon: <Icon />,
    portrait: { name, src: userAvatarSrc(user) },
    state: userState(user, { self }),
    facts: [
      user.username !== name ? user.username : "",
      user.email ?? "",
      user.role === "admin" ? "Admin" : "Member",
      user.twoFactorEnabledAt ? "Two-factor on" : "",
      user.lastLoginAt ? `Signed in ${formatRelative(user.lastLoginAt)}` : "",
    ].filter(Boolean),
    target: user,
    actions,
  };
}

/**
 * A person wherever they are named: their photo or initials, their role, whether two-factor
 * sign-in is on and when they last signed in. "You" marks your own account.
 */
export function UserIdentity<T extends UserEntity>({ user, tier = "row", actions, self, headerActions, className, onOpen }: {
  user: T;
  tier?: "header" | "compact" | "row" | "chip" | "icon" | "auto";
  actions?: ActionSet<T>;
  self?: boolean;
  headerActions?: ReactNode;
  className?: string;
  onOpen?: () => void;
}) {
  return (
    <EntityIdentity
      {...userIdentityProps(user, { actions, self })}
      tier={tier}
      className={className}
      slots={headerActions ? { actions: headerActions } : undefined}
      onOpen={onOpen}
    />
  );
}
