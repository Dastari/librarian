import { Link, useNavigate } from "@tanstack/react-router";
import { MediaRow, PosterCard, Section, ErrorState } from "@/components/ui";
import { PersonalEntriesDocument } from "@/graphql/generated/graphql";
import { useLiveQuery } from "@/lib/graphql/live/hooks";
import { entryPresentation } from "./types";
import { usePersonalLists } from "./usePersonalLibrary";
import { SmartResults } from "./SmartResults";
import { EntitySavedViewGetDocument } from "@/graphql/generated/graphql";

export function PersonalShelf({ listId }: { listId: string }) {
  const lists = usePersonalLists();
  const list = lists.rows.find((item) => item.id === listId);
  const entries = useLiveQuery(PersonalEntriesDocument, { variables: { listId, page: { limit: 20, offset: 0 } }, skip: list?.kind === "SMART" });
  const view = useLiveQuery(EntitySavedViewGetDocument, { variables: { id: list?.savedViewId ?? "" }, skip: !list?.savedViewId });
  const navigate = useNavigate();
  if (!list) return null;
  const savedView = (view.data ?? view.previousData)?.savedView;
  return <Section title={list.name} bleed trailing={<Link to="/lists/$listId" params={{ listId }}>See all</Link>}>
    {list.kind === "SMART" ? <>{view.error ? <ErrorState error={view.error} onRetry={() => void view.refetch()} /> : null}{savedView?.id === list.savedViewId ? <SmartResults view={savedView} preview /> : view.loading ? <p role="status">Loading rules…</p> : !view.error ? <p role="status">Collection rules are unavailable.</p> : null}</> : <>{entries.error ? <ErrorState error={entries.error} onRetry={() => void entries.refetch()} /> : null}<MediaRow ariaLabel={list.name}>
      {entries.data?.listEntries.edges.map(({ node }) => { const item = entryPresentation(node); return item ? <PosterCard key={node.id} title={item.title} meta={item.subtitle} image={item.image} width="row" aspect={item.square ? "square" : "poster"} onPress={() => void navigate({ href: item.href })} /> : null; })}
    </MediaRow></>}
  </Section>;
}
