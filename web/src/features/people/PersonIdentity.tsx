import type { ActionSet } from "glass-ux/actions";
import { EntityIdentity, type EntityIdentityProps, type EntityState, type EntityTierProp } from "glass-ux/entity";
import type { ReactNode } from "react";

import { sizedArtwork } from "@/lib/api/urls";
import { personProfile } from "@/lib/artwork";
import { ENTITY_KINDS } from "@/lib/entities/kinds";

export interface PersonEntity {
  id: string;
  name: string;
  profileUrl?: string | null;
  knownForDepartment?: string | null;
  birthday?: string | null;
  deathday?: string | null;
  tmdbPersonId?: number | null;
  detailsFetchedAt?: string | null;
}

const year = (date?: string | null) => date?.slice(0, 4) ?? "";

/** "1964", "1931–2004": the years a person lived, where known. */
export function lifeYears(person: Pick<PersonEntity, "birthday" | "deathday">): string {
  const born = year(person.birthday);
  const died = year(person.deathday);
  if (born && died) return `${born}–${died}`;
  if (died) return `died ${died}`;
  return born ? `born ${born}` : "";
}

/**
 * A person's state, shown as a badge: whether their own record (biography, dates) has been read
 * yet. A person known only by name (a book's narrator) has no record to read.
 */
export function personState(person: PersonEntity): EntityState | undefined {
  if (person.deathday) return { id: "deceased", label: `Died ${year(person.deathday)}`, tone: "neutral" };
  if ((person.tmdbPersonId ?? 0) <= 0) return undefined;
  if (!person.detailsFetchedAt) return { id: "pending", label: "Details not read yet", tone: "neutral", paused: true };
  return undefined;
}

/** What PersonIdentity draws, for surfaces that take an identity's props (a page header). */
export function personIdentityProps<T extends PersonEntity>(person: T, { actions }: { actions?: ActionSet<T> } = {}): EntityIdentityProps<T> {
  return {
    kind: ENTITY_KINDS.person.noun,
    name: person.name,
    icon: <ENTITY_KINDS.person.icon />,
    portrait: { name: person.name, src: sizedArtwork(personProfile(person), 160) },
    state: personState(person),
    facts: [person.knownForDepartment ?? "", lifeYears(person)],
    target: person,
    href: `/people/${person.id}`,
    actions,
  };
}

/** A person wherever they are named: their portrait (or initials), what they are known for and when they lived. */
export function PersonIdentity<T extends PersonEntity>({ person, tier = "row", actions, heading, className, secondary, headerActions }: {
  person: T;
  tier?: EntityTierProp;
  actions?: ActionSet<T>;
  heading?: "h1" | "h2" | "h3" | "h4" | "p";
  className?: string;
  /** A row's second line in place of the facts (a character, a job). */
  secondary?: ReactNode;
  headerActions?: ReactNode;
}) {
  return (
    <EntityIdentity
      {...personIdentityProps(person, { actions })}
      tier={tier}
      heading={heading}
      className={className}
      secondary={secondary}
      slots={headerActions ? { actions: headerActions } : undefined}
    />
  );
}
