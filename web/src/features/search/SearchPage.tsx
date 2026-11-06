import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";
import { SearchField } from "glass-ux/input";
import { IconDownload, IconSearch } from "glass-ux/icons";
import { OnScreenKeyboard, type NavigationDirection } from "glass-ux/on-screen-keyboard";
import { parseAsString, parseAsStringEnum, useQueryState } from "nuqs";
import { useEffect, useMemo, useRef, useState } from "react";

import { PageScroller } from "@/components/shell/PageScroller";
import { Button, EmptyState, SegmentTabs, SkeletonRow } from "@/components/ui";
import { ReleaseSearchDialog } from "@/features/downloads/ReleaseSearchDialog";
import { LibrariesOverviewDocument, LibrarySearchDocument, SearchCreditsDocument, SearchStartDocument } from "@/graphql/generated/graphql";
import { useDebouncedValue } from "@/hooks/useDebouncedValue";
import { useIsAdmin } from "@/lib/auth/useSession";
import { findNext, revealFocused, useInputMode } from "@/lib/input-mode";
import { libraryType } from "@/lib/library-types";
import { usePref } from "@/lib/prefs";
import { cn } from "@/lib/utils";

import { ProviderResults } from "./ProviderResults";
import { SearchTile } from "./SearchTile";
import { useSearchFocusRequest } from "./searchFocus";
import { rankHits, startHits, suggestTerms, type SearchHit } from "./searchResults";

const SCOPES = ["library", "movie", "show", "album", "audiobook"] as const;
const ADD_LABELS = { movies: "Add movies", tv: "Add TV shows", music: "Add music", audiobooks: "Add audiobooks" } as const;
type Scope = (typeof SCOPES)[number];

const RECENT_LIMIT = 8;
/** Terms under a TV's keyboard: as many as fit beside the keys without scrolling. */
const TV_TERMS = 6;
const ARROWS: Record<NavigationDirection, "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"> = { up: "ArrowUp", down: "ArrowDown", left: "ArrowLeft", right: "ArrowRight" };

/**
 * Search, the way TV apps do it: results update with every letter, every kind of item in
 * one grid with the best matches first, and search terms (people, directors, authors,
 * titles) offered beside them. On a TV the letters come from an on-screen alphabet grid
 * with the terms under it; elsewhere from the search field. Admins can switch the scope to
 * a metadata provider to add new items, which is never searched by accident.
 */
export function SearchPage() {
  const isAdmin = useIsAdmin();
  const [query, setQuery] = useQueryState("q", parseAsString.withDefault(""));
  const [scope, setScope] = useQueryState("in", parseAsStringEnum<Scope>([...SCOPES]).withDefault("library"));
  const debounced = useDebouncedValue(query.trim(), 250);
  const inputRef = useRef<HTMLInputElement>(null);
  const [releases, setReleases] = useState(false);
  const [recent, setRecent] = usePref<string[]>("search.recent", []);
  const tv = useInputMode().mode === "tv";

  const libraries = useQuery(LibrariesOverviewDocument);
  const libraryList = libraries.data?.libraries.edges.map((edge) => edge.node) ?? [];
  const searching = scope === "library" && debounced.length > 0;
  const results = useQuery(LibrarySearchDocument, { variables: { needle: debounced, limit: 20 }, skip: !searching });
  const start = useQuery(SearchStartDocument, { variables: { limit: 12 }, skip: scope !== "library" });
  const data = searching ? (results.data ?? results.previousData) : undefined;
  const personIds = useMemo(() => data?.people.edges.map(({ node }) => node.id) ?? [], [data]);
  const credits = useQuery(SearchCreditsDocument, { variables: { personIds, limit: 24 }, skip: !searching || personIds.length === 0 });

  const hits = useMemo(
    () => (searching ? rankHits(debounced, data, personIds.length ? credits.data?.movieCastCredits.edges.map(({ node }) => node) : []) : startHits(start.data)),
    [searching, debounced, data, credits.data, personIds.length, start.data],
  );
  const terms = useMemo(() => (searching ? suggestTerms(debounced, data, hits) : recent), [searching, debounced, data, hits, recent]);

  useEffect(() => {
    if (!tv) inputRef.current?.focus();
  }, [tv]);

  // The top bar's search box, pressed while already here.
  useSearchFocusRequest(() => {
    const input = inputRef.current;
    if (input) {
      input.focus();
      input.select();
    } else document.querySelector<HTMLElement>(".search-tv-keys button")?.focus();
  });

  const remember = () => {
    const term = debounced;
    if (term) setRecent((previous) => [term, ...previous.filter((item) => item.toLocaleLowerCase() !== term.toLocaleLowerCase())].slice(0, RECENT_LIMIT));
  };
  const choose = (term: string) => void setQuery(term);

  const availableScopes: Array<{ key: Scope; label: string }> = [
    { key: "library", label: "Your library" },
    ...(isAdmin
      ? (["movies", "tv", "music", "audiobooks"] as const)
          .filter((type) => libraryList.some((library) => libraryType(library.libraryType).type === type))
          .map((type) => ({ key: (type === "movies" ? "movie" : type === "tv" ? "show" : type === "music" ? "album" : "audiobook") as Scope, label: ADD_LABELS[type] }))
      : []),
  ];
  const scopes = availableScopes.length > 1 ? <SegmentTabs ariaLabel="Search scope" items={availableScopes} selected={scope} onSelect={(key) => void setScope(key as Scope)} size="sm" className="max-w-2xl" /> : null;

  const heading = !searching ? "Recently added" : results.loading && !data ? "Searching…" : hits.length ? "Top results" : null;
  const body = scope !== "library" ? (
    <ProviderResults kind={scope} query={debounced} libraries={libraryList} />
  ) : searching && results.loading && !data ? (
    <SkeletonRow />
  ) : searching && hits.length === 0 ? (
    <EmptyState icon={IconSearch} title={`Nothing in your library matches “${debounced}”`} description={isAdmin ? "Switch to an “Add” tab to search metadata providers." : "Try another title, or a person's name."} className="border-none" />
  ) : (
    <ResultsGrid hits={hits} tv={tv} onOpen={remember} />
  );

  if (tv) {
    // Remote arrows leave the keys at an edge: right to the results, down to the terms.
    const leaveKeys = (direction: NavigationDirection) => {
      const active = document.activeElement as HTMLElement | null;
      const next = active ? findNext(active, ARROWS[direction]) : null;
      if (!next) return;
      next.focus({ preventScroll: true });
      revealFocused(next);
    };
    return (
      <div className="search-tv page-gutter grid min-h-0 flex-1 gap-10 pt-8">
        <aside data-nav-section className="search-tv-keys flex min-h-0 flex-col gap-6">
          <OnScreenKeyboard layout="alphabet" size="lg" value={query} onValueChange={(value) => void setQuery(value || null)} onNavigateOut={leaveKeys} autoFocus aria-label="Search keyboard" aria-describedby="search-query" />
          <SuggestionList title={searching ? undefined : terms.length ? "Recent searches" : undefined} terms={terms.slice(0, TV_TERMS)} onChoose={choose} />
        </aside>
        {/* The query, its heading and the tabs hold still; only the results scroll. */}
        <div className="flex min-h-0 min-w-0 flex-col gap-4">
          <div className="flex shrink-0 items-start justify-between gap-4">
            <div className="min-w-0">
              <p id="search-query" aria-live="polite" className={`truncate text-display-xl ${query ? "text-foreground" : "text-muted"}`}>{query || "Search"}</p>
              <p className="text-overline mt-1 min-h-4 text-muted">{heading}</p>
            </div>
            {isAdmin && debounced ? <Button variant="secondary" className="shrink-0" onPress={() => setReleases(true)}><IconDownload /> Find releases</Button> : null}
          </div>
          <div className="min-h-10 shrink-0">{scopes}</div>
          <PageScroller className="search-tv-scroll flex-1">{body}</PageScroller>
        </div>
        <ReleaseSearchDialog isOpen={releases} onOpenChange={setReleases} query={debounced} />
      </div>
    );
  }

  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col">
      {/* Everything you steer the search with holds still; only the results scroll. */}
      <div className="page-gutter flex shrink-0 flex-col gap-3 pb-3 pt-2">
        <h1 className="sr-only">Search</h1>
        <SearchField ref={inputRef} aria-label="Search" value={query} onValueChange={(value) => void setQuery(value || null)} placeholder="Titles, people, albums…" size="lg" className="w-full max-w-2xl" data-spatial-start />
        <div className="search-terms flex h-9 min-w-0 items-center gap-2" role="group" aria-label={searching ? "Suggested searches" : "Recent searches"}>
          {terms.length && scope === "library" ? (
            <>
              {!searching ? <span className="shrink-0 text-label-sm text-muted">Recent</span> : null}
              {terms.map((term) => (
                <Button key={term} variant="ghost" size="sm" className="shrink-0 rounded-full border border-separator" onPress={() => choose(term)}>{term}</Button>
              ))}
            </>
          ) : null}
        </div>
        <div className="flex min-h-10 min-w-0 items-center justify-between gap-3">
          <div className="min-w-0 flex-1">{scopes}</div>
          {isAdmin && debounced ? <Button variant="secondary" size="sm" className="shrink-0" onPress={() => setReleases(true)}><IconDownload /> Find releases</Button> : null}
        </div>
      </div>
      <PageScroller className="search-results page-gutter flex flex-1 flex-col gap-3 pb-8 pt-1">
        {scope === "library" ? <p className="text-overline min-h-4 shrink-0 text-muted">{heading}</p> : null}
        {body}
      </PageScroller>
      <ReleaseSearchDialog isOpen={releases} onOpenChange={setReleases} query={debounced} />
    </div>
  );
}

function ResultsGrid({ hits, tv, onOpen }: { hits: readonly SearchHit[]; tv: boolean; onOpen: () => void }) {
  return (
    <div data-nav-group role="list" aria-label="Results" className={cn("grid gap-x-4", tv ? "search-tv-results gap-y-6" : "grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-y-5")}>
      {hits.map((hit) => (
        <div key={hit.key} role="listitem" className="min-w-0">
          <SearchTile hit={hit} onOpen={onOpen} />
        </div>
      ))}
    </div>
  );
}

/** The search terms under a TV's keyboard: plain lines of text, like the keys above them. */
function SuggestionList({ title, terms, onChoose }: { title?: string; terms: readonly string[]; onChoose: (term: string) => void }) {
  if (!terms.length) return null;
  return (
    <nav aria-label={title ?? "Suggested searches"} className="flex flex-col">
      {title ? <p className="text-overline mb-2 text-muted">{title}</p> : null}
      <ul className="flex flex-col">
        {terms.map((term) => (
          <li key={term}>
            <button type="button" onClick={() => onChoose(term)} className="search-suggestion nav-focus w-full truncate rounded-control px-3 py-1.5 text-start text-title-sm text-muted" data-focusable>
              {term}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
