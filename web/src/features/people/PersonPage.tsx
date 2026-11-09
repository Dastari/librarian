import { ActionBar } from "glass-ux/actions";
import { IconInfo } from "glass-ux/icons";
import { useMemo } from "react";

import { EmptyState, ErrorState, HeroFacts, MediaRow, MetaLine, PosterCard, Section, SkeletonHero } from "@/components/ui";
import { DetailShell } from "@/features/media/DetailShell";
import { PersonCreditsDocument, PersonDetailDocument, type PersonCreditsQuery } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, moviePoster, personProfile, showPoster } from "@/lib/artwork";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { LIBRARY_TYPES } from "@/lib/library-types";

import { lifeYears } from "./PersonIdentity";
import { usePersonActions } from "./usePersonActions";

type PersonCredit = PersonCreditsQuery["credits"]["edges"][number]["node"];

/** One title on a person's page, with every part they had in it. */
interface CreditedTitle {
  key: string;
  title: string;
  year?: number | null;
  image: string;
  to: "/movies/$movieId" | "/shows/$showId" | "/albums/$albumId" | "/audiobooks/$audiobookId";
  params: Record<string, string>;
  roles: string[];
}

function roleOf(credit: PersonCredit): string | null {
  if (credit.kind === "cast") {
    if (credit.characterName) return credit.characterName;
    // A narrator or a musician: the part is the job.
    if (credit.job && credit.department !== "Acting") return credit.job;
    return credit.guest ? "Guest star" : null;
  }
  return credit.job ?? credit.department ?? null;
}

function add(map: Map<string, CreditedTitle>, base: Omit<CreditedTitle, "roles">, role: string | null) {
  const entry = map.get(base.key) ?? { ...base, roles: [] };
  if (role && !entry.roles.includes(role)) entry.roles.push(role);
  map.set(base.key, entry);
}

const calendar = new Intl.DateTimeFormat(undefined, { year: "numeric", month: "long", day: "numeric", timeZone: "UTC" });
/** A calendar date (`YYYY-MM-DD`) as written, never shifted by the viewer's time zone. */
export function calendarDate(value?: string | null): string | null {
  if (!value || !/^\d{4}-\d{2}-\d{2}$/.test(value)) return value ?? null;
  const date = new Date(`${value}T00:00:00Z`);
  return Number.isNaN(date.getTime()) ? value : calendar.format(date);
}

const byYear = (a: CreditedTitle, b: CreditedTitle) => (b.year ?? 0) - (a.year ?? 0) || a.title.localeCompare(b.title);

/** A person's credits in the libraries the viewer can see, by kind of title. */
export function groupPersonCredits(credits: readonly PersonCredit[]) {
  const movies = new Map<string, CreditedTitle>();
  const shows = new Map<string, CreditedTitle>();
  const albums = new Map<string, CreditedTitle>();
  const books = new Map<string, CreditedTitle>();
  for (const credit of credits) {
    const role = roleOf(credit);
    if (credit.movie) {
      add(movies, { key: credit.movie.id, title: credit.movie.title, year: credit.movie.year, image: moviePoster(credit.movie.id), to: "/movies/$movieId", params: { movieId: credit.movie.id } }, role);
    }
    const showId = credit.show?.id ?? credit.season?.showId ?? credit.episode?.showId;
    if (showId) {
      const name = credit.show?.name ?? "";
      const episodeRole = credit.episode ? [`S${String(credit.episode.season).padStart(2, "0")}E${String(credit.episode.episode).padStart(2, "0")}`, role].filter(Boolean).join(" · ") : role;
      add(shows, { key: showId, title: name, year: credit.show?.year, image: showPoster({ id: showId }), to: "/shows/$showId", params: { showId } }, episodeRole);
    }
    const albumId = credit.album?.id ?? credit.track?.albumId;
    if (albumId) {
      const trackRole = credit.track ? [credit.track.title, role].filter(Boolean).join(" · ") : role;
      add(albums, { key: albumId, title: credit.album?.name ?? "", year: credit.album?.year, image: albumCover({ id: albumId }), to: "/albums/$albumId", params: { albumId } }, trackRole);
    }
    if (credit.audiobook) {
      add(books, { key: credit.audiobook.id, title: credit.audiobook.title, image: audiobookCover(credit.audiobook), to: "/audiobooks/$audiobookId", params: { audiobookId: credit.audiobook.id } }, role);
    }
  }
  // A show or album named only through an episode or track has no title of its own here.
  const named = (list: CreditedTitle[]) => list.map((entry) => ({ ...entry, title: entry.title || "Untitled" }));
  return {
    movies: [...movies.values()].sort(byYear),
    shows: named([...shows.values()]).sort(byYear),
    albums: named([...albums.values()]).sort(byYear),
    books: [...books.values()].sort((a, b) => a.title.localeCompare(b.title)),
  };
}

function CreditShelf({ title, items, square, tint }: { title: string; items: CreditedTitle[]; square?: boolean; tint?: string }) {
  if (items.length === 0) return null;
  return (
    <Section title={title} bleed trailing={<span>{items.length}</span>}>
      <MediaRow ariaLabel={title}>
        {items.map((item) => (
          <PosterCard key={item.key} width="row" title={item.title} aspect={square ? "square" : "poster"} tint={tint}
            meta={[item.year, item.roles.slice(0, 2).join(", ") + (item.roles.length > 2 ? ` +${item.roles.length - 2}` : "")].filter(Boolean).join(" · ") || undefined}
            image={item.image} to={item.to} params={item.params} />
        ))}
      </MediaRow>
    </Section>
  );
}

export function PersonPage({ personId }: { personId: string }) {
  const { data, previousData, loading, error, refetch } = useLiveQuery(PersonDetailDocument, { variables: { id: personId } });
  const candidate = (data ?? previousData)?.person;
  const person = candidate?.id === personId ? candidate : null;
  const creditsQuery = useLiveQuery(PersonCreditsDocument, { variables: { personId } });
  const grouped = useMemo(() => groupPersonCredits(creditsQuery.data?.credits.edges.map((edge) => edge.node) ?? []), [creditsQuery.data]);
  const actions = usePersonActions<NonNullable<typeof person>>({ open: false });

  if (error && !person) return <ErrorState error={error} onRetry={() => void refetch()} className="m-8" />;
  if (!person) return loading ? <SkeletonHero /> : <EmptyState icon={IconInfo} title="Person not found" description="Nobody by that id is credited in your libraries." className="page-gutter my-8" />;

  const total = grouped.movies.length + grouped.shows.length + grouped.albums.length + grouped.books.length;
  const born = [calendarDate(person.birthday), person.placeOfBirth].filter(Boolean).join(" · ");

  return (
    <DetailShell
      backdrop={null}
      hero={{
        poster: personProfile(person),
        eyebrow: person.knownForDepartment ?? "Person",
        title: person.name,
        meta: <MetaLine items={[lifeYears(person) || null, total ? `${total} title${total === 1 ? "" : "s"} in your libraries` : null]} />,
        description: person.biography ? <span className="line-clamp-6 whitespace-pre-line">{person.biography}</span> : null,
        facts: <HeroFacts items={[
          { label: "Born", value: born || null },
          { label: "Died", value: calendarDate(person.deathday) },
          { label: "Also known as", value: person.alsoKnownAs?.length ? person.alsoKnownAs.slice(0, 4).join(", ") : null },
        ]} />,
        actions: <ActionBar set={actions} targets={[person]} label={person.name} />,
      }}
    >
      <CreditShelf title="Movies" items={grouped.movies} tint={LIBRARY_TYPES.movies.tintVar} />
      <CreditShelf title="TV" items={grouped.shows} tint={LIBRARY_TYPES.tv.tintVar} />
      <CreditShelf title="Music" items={grouped.albums} square tint={LIBRARY_TYPES.music.tintVar} />
      <CreditShelf title="Audiobooks" items={grouped.books} tint={LIBRARY_TYPES.audiobooks.tintVar} />
      {total === 0 && !creditsQuery.loading ? <p className="text-body text-muted">No titles in your libraries credit {person.name}.</p> : null}
    </DetailShell>
  );
}
