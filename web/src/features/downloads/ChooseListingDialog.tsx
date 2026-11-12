import { Badge } from "glass-ux/badge";
import { Dialog as GlassDialog } from "glass-ux/dialog";
import { SearchField } from "glass-ux/input";
import { useEffect, useState } from "react";

import { Artwork, Button, EmptyState, Spinner } from "@/components/ui";
import { useDialogReturnFocus } from "@/components/ui/Dialog";
import type { IdentificationCandidateFieldsFragment } from "@/graphql/generated/graphql";
import { formatRuntime } from "@/lib/format";
import { IconSearch } from "glass-ux/icons";

export type Listing = IdentificationCandidateFieldsFragment;

const FIT = {
  recording: { label: "Same length", tone: "success" },
  book: { label: "Close in length", tone: "warning" },
  different: { label: "Different length", tone: "danger" },
} as const;

function ListingFacts({ listing }: { listing: Listing }) {
  const facts = [
    listing.creator,
    listing.narrator ? `Read by ${listing.narrator}` : null,
    listing.year,
    // A listing's length is what's compared, so it's always worth seeing.
    listing.seconds ? formatRuntime(Math.round(listing.seconds)) : null,
    listing.chapters ? `${listing.chapters} chapters` : null,
    listing.provider,
  ].filter(Boolean);
  return <p className="text-label-sm text-muted [overflow-wrap:anywhere]">{facts.join(" · ")}</p>;
}

/**
 * Choose what something really is from what could be: listings compared with the files, best first,
 * with a search for when the right one isn't among them. Shared by downloads and book pages.
 */
export function ChooseListingDialog({ isOpen, onOpenChange, title, initialQuery, listings, loading, error, search, choose, chooseLabel = "Use this", currentProviderId }: {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  initialQuery: string;
  listings: readonly Listing[];
  loading: boolean;
  error?: string | null;
  search: (query: string | null) => void;
  choose: (listing: Listing) => Promise<void>;
  chooseLabel?: string;
  currentProviderId?: string | null;
}) {
  const finalFocus = useDialogReturnFocus(isOpen);
  const [query, setQuery] = useState(initialQuery);
  const [busy, setBusy] = useState<string | null>(null);
  useEffect(() => {
    if (!isOpen) return;
    setQuery(initialQuery);
    search(null);
    // Each opening starts from the original search.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);
  const pick = async (listing: Listing, key: string) => {
    setBusy(key);
    try { await choose(listing); } finally { setBusy(null); }
  };
  return (
    <GlassDialog.Root open={isOpen} onOpenChange={onOpenChange}>
      <GlassDialog.Popup size="lg" finalFocus={finalFocus}>
        <GlassDialog.Header title={title} />
        <GlassDialog.Body className="flex min-h-0 flex-col gap-3">
          <form className="flex min-w-0 gap-2" onSubmit={(event) => { event.preventDefault(); search(query.trim() || null); }}>
            <SearchField aria-label="Search listings" value={query} onValueChange={setQuery} placeholder="Title and author" className="min-w-0 flex-1" />
            <Button type="submit" variant="secondary" isPending={loading}>Search</Button>
          </form>
          {error ? <p className="text-label-sm text-danger">{error}</p> : null}
          {loading && listings.length === 0 ? (
            <div className="grid h-40 place-items-center"><Spinner size={28} /></div>
          ) : listings.length === 0 ? (
            <EmptyState compact icon={IconSearch} title="Nothing found" description="Try a different title or author." />
          ) : (
            <ul className="flex min-w-0 flex-col divide-y divide-separator">
              {listings.map((listing, index) => {
                const key = `${listing.libraryItemId ?? listing.providerId ?? listing.title}:${index}`;
                const otherAuthor = listing.reasons.some((reason) => reason.startsWith("A different author"));
                const fit = listing.fit ? FIT[listing.fit as keyof typeof FIT] : undefined;
                const current = Boolean(currentProviderId) && listing.providerId === currentProviderId;
                return (
                  <li key={key} className="flex min-w-0 items-start gap-3 py-3">
                    <Artwork src={listing.coverUrl} alt="" aspect="square" className="w-14 shrink-0" />
                    <div className="flex min-w-0 flex-1 flex-col gap-1">
                      <p className="text-title-sm text-foreground [overflow-wrap:anywhere]">{listing.title}</p>
                      <ListingFacts listing={listing} />
                      <div className="flex flex-wrap items-center gap-1.5">
                        {otherAuthor ? <Badge size="sm" marker="dot" tone="danger">Different author</Badge> : fit ? <Badge size="sm" marker="dot" tone={fit.tone}>{fit.label}</Badge> : null}
                        {listing.source === "library" ? <Badge size="sm" tone="info">In your library</Badge> : null}
                        {current ? <Badge size="sm" tone="neutral">Current</Badge> : null}
                      </div>
                      {listing.reasons.length ? <p className="text-label-sm text-muted [overflow-wrap:anywhere]">{listing.reasons.join(". ")}.</p> : null}
                    </div>
                    <Button size="sm" variant="secondary" isDisabled={current || (busy !== null && busy !== key)} isPending={busy === key} onPress={() => void pick(listing, key)}>{chooseLabel}</Button>
                  </li>
                );
              })}
            </ul>
          )}
        </GlassDialog.Body>
      </GlassDialog.Popup>
    </GlassDialog.Root>
  );
}
