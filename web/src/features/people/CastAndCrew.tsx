import { Link } from "@tanstack/react-router";
import { Button } from "glass-ux/button";
import { IconImage } from "glass-ux/icons";
import { useMemo, useState } from "react";

import { Artwork, MediaRow, Section } from "@/components/ui";
import { TitleCreditsDocument, type CreditWhereInput, type TitleCreditsQuery } from "@/graphql/generated/graphql";
import { personProfile } from "@/lib/artwork";
import { useLiveQuery } from "@/lib/graphql/live/hooks";

import { castOf, crewGroups, type CreditPerson } from "./credits";
import { PersonIdentity } from "./PersonIdentity";
import { usePersonActions } from "./usePersonActions";

export type TitleCredit = TitleCreditsQuery["credits"]["edges"][number]["node"];

/** A title's credits, live. `where` names the title (`{ movieId: { eq } }`); `skip` waits for it. */
export function useTitleCredits(where: CreditWhereInput, skip = false) {
  const query = useLiveQuery(TitleCreditsDocument, { variables: { where }, skip });
  const credits = useMemo(() => query.data?.credits.edges.map((edge) => edge.node) ?? [], [query.data]);
  return { credits, loading: query.loading, error: query.error };
}

/** A cast member: their portrait, name and the part they played. Opens their page. */
export function PersonTile({ person, role, detail }: { person: CreditPerson; role?: string | null; detail?: string | null }) {
  return (
    <Link to="/people/$personId" params={{ personId: person.id }} data-focusable role="listitem"
      className="tv-lift nav-focus group w-28 shrink-0 rounded-tile p-1 text-center"
      aria-label={[person.name, role ? `as ${role}` : null, detail].filter(Boolean).join(", ")}>
      <Artwork src={personProfile(person)} alt="" aspect="square" displayWidth={96} className="tv-lift-target mx-auto w-24 overflow-clip rounded-full" fallback={<IconImage size="xl" />} />
      <p className="mt-2 truncate text-label text-foreground">{person.name}</p>
      <p className="truncate text-label-sm text-muted">{role ?? " "}</p>
      {detail ? <p className="text-numeric truncate text-label-sm text-muted">{detail}</p> : null}
    </Link>
  );
}

const episodesLabel = (count?: number | null) => (count ? `${count} episode${count === 1 ? "" : "s"}` : null);

/** The cast as a row of portraits, in billing order. */
export function CastRow({ credits, title = "Cast", guest, showEpisodes }: { credits: readonly TitleCredit[]; title?: string; guest?: boolean; showEpisodes?: boolean }) {
  const cast = useMemo(() => castOf(credits, { guest }), [credits, guest]);
  if (cast.length === 0) return null;
  return (
    <Section title={title} bleed trailing={<span>{cast.length}</span>}>
      <MediaRow ariaLabel={title}>
        {cast.map((entry) => (
          <PersonTile key={entry.person.id} person={entry.person} role={entry.characters.join(" / ") || null} detail={showEpisodes ? episodesLabel(entry.episodeCount) : null} />
        ))}
      </MediaRow>
    </Section>
  );
}

/** Groups a page always shows; the rest of the crew is one press away. */
const KEY_GROUPS = new Set(["Created by", "Directed by", "Written by", "Produced by", "Music"]);
/** People listed per group before "and N more". */
const PER_GROUP = 8;

/** The crew in groups (directors, writers, producers…), each person a chip that opens their page. */
export function CrewList({ credits, title = "Crew" }: { credits: readonly TitleCredit[]; title?: string }) {
  const groups = useMemo(() => crewGroups(credits), [credits]);
  const [all, setAll] = useState(false);
  const actions = usePersonActions<CreditPerson>();
  if (groups.length === 0) return null;
  const key = groups.filter((group) => KEY_GROUPS.has(group.label));
  const shown = all || key.length === 0 ? groups : key;
  const hidden = groups.length - shown.length;
  return (
    <section data-nav-section aria-label={title} className="page-gutter flex flex-col gap-3">
      <h2 className="text-title-lg text-foreground">{title}</h2>
      <dl className="grid gap-x-6 gap-y-3 sm:grid-cols-[minmax(8rem,auto)_minmax(0,1fr)]">
        {shown.map((group) => {
          const people = all ? group.entries : group.entries.slice(0, PER_GROUP);
          const more = group.entries.length - people.length;
          return (
            <div key={group.label} className="contents">
              <dt className="pt-1 text-label text-muted">{group.label}</dt>
              <dd data-nav-group className="flex min-w-0 flex-wrap items-center gap-2">
                {people.map((entry) => (
                  <PersonIdentity key={entry.person.id} person={entry.person} tier="chip" actions={actions}
                    secondary={KEY_GROUPS.has(group.label) ? undefined : entry.jobs.join(", ")} />
                ))}
                {more > 0 ? <span className="text-label-sm text-muted">and {more} more</span> : null}
              </dd>
            </div>
          );
        })}
      </dl>
      {hidden > 0 || (!all && groups.some((group) => group.entries.length > PER_GROUP)) ? (
        <div>
          <Button size="sm" variant="ghost" onClick={() => setAll((value) => !value)}>{all ? "Fewer crew" : "Full crew"}</Button>
        </div>
      ) : null}
    </section>
  );
}

/** A title's Cast and Crew sections. Nothing shows until there are credits. */
export function CastAndCrew({ credits, castTitle = "Cast", crewTitle = "Crew", showEpisodes }: { credits: readonly TitleCredit[]; castTitle?: string; crewTitle?: string; showEpisodes?: boolean }) {
  return (
    <>
      <CastRow credits={credits} title={castTitle} guest={false} showEpisodes={showEpisodes} />
      <CrewList credits={credits} title={crewTitle} />
    </>
  );
}
