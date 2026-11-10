import { useApolloClient } from "@apollo/client/react";
import { Checkbox } from "glass-ux/checkbox";
import { Segmented } from "glass-ux/segmented";
import { toast } from "glass-ux/toast";
import { useEffect, useMemo, useState } from "react";

import { Button, Dialog } from "@/components/ui";
import { EntityStatsShareCreateDocument, EntityStatsShareDeleteDocument, StatisticsSharingDocument } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

export type Audience = "private" | "everyone" | "people";

type Share = { id: string; audience: string; viewerUserId?: string | null };

/** What the saved shares amount to. */
export function audienceOf(shares: readonly Share[]): { audience: Audience; people: string[] } {
  if (shares.some((share) => share.audience === "everyone")) return { audience: "everyone", people: [] };
  const people = shares.filter((share) => share.audience === "user" && share.viewerUserId).map((share) => share.viewerUserId!);
  return { audience: people.length > 0 ? "people" : "private", people };
}

/** The writes that turn the saved shares into the chosen ones: what to remove and what to add. */
export function shareChanges(shares: readonly Share[], audience: Audience, people: readonly string[]) {
  const wanted = audience === "everyone" ? ["*"] : audience === "people" ? [...new Set(people)] : [];
  const key = (share: Share) => (share.audience === "everyone" ? "*" : share.viewerUserId ?? "");
  const remove = shares.filter((share) => !wanted.includes(key(share))).map((share) => share.id);
  const have = new Set(shares.map(key));
  const add = wanted.filter((viewer) => !have.has(viewer)).map((viewer) => (viewer === "*" ? { audience: "everyone" as const, viewerUserId: null } : { audience: "user" as const, viewerUserId: viewer }));
  return { remove, add };
}

/**
 * Who can open your statistics: nobody (the default), everyone on this server, or the people you
 * pick. They see them read-only; your history itself, devices and addresses stay yours.
 */
export function ShareStatisticsDialog({ userId, open, onOpenChange }: { userId: string; open: boolean; onOpenChange: (open: boolean) => void }) {
  const client = useApolloClient();
  const sharing = useQuery(StatisticsSharingDocument, { variables: { userId } });
  const shares = useMemo(() => sharing.data?.statsShares.edges.map((edge) => edge.node) ?? [], [sharing.data]);
  const candidates = sharing.data?.statisticsShareCandidates ?? [];
  const saved = useMemo(() => audienceOf(shares), [shares]);
  const [audience, setAudience] = useState<Audience>(saved.audience);
  const [people, setPeople] = useState<string[]>(saved.people);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    if (!sharing.data) return;
    setAudience(saved.audience);
    setPeople(saved.people);
  }, [sharing.data, saved]);

  const save = async () => {
    setSaving(true);
    try {
      const { remove, add } = shareChanges(shares, audience, people);
      for (const id of remove) assertSuccess((await client.mutate({ mutation: EntityStatsShareDeleteDocument, variables: { id } })).data?.deleteStatsShare, "A share could not be removed");
      for (const share of add) assertSuccess((await client.mutate({ mutation: EntityStatsShareCreateDocument, variables: { input: { userId, ...share } } })).data?.createStatsShare, "A share could not be added");
      toast.success(audience === "private" ? "Your statistics are private" : audience === "everyone" ? "Shared with everyone on this server" : `Shared with ${people.length === 1 ? "1 person" : `${people.length} people`}`);
      await sharing.refetch();
      onOpenChange(false);
    } catch (error) {
      toast.danger(errorMessage(error, "Sharing could not be saved"));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog
      isOpen={open}
      onOpenChange={onOpenChange}
      title="Share your statistics"
      description="People you share with can open your statistics read-only. Your history, devices and addresses are never shared."
      size="sm"
      footer={(
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="primary" isPending={saving} isDisabled={!sharing.data || (audience === "people" && people.length === 0)} onPress={() => void save()}>Save</Button>
        </>
      )}
    >
      <div className="flex min-w-0 flex-col gap-4">
        <Segmented
          aria-label="Who can see your statistics"
          value={audience}
          onValueChange={(value) => setAudience(value as Audience)}
          options={[{ value: "private", label: "Only me" }, { value: "everyone", label: "Everyone" }, { value: "people", label: "Some people" }]}
        />
        {audience === "people" ? (
          candidates.length === 0 ? (
            <p className="text-body-sm text-muted">Nobody else has an account on this server yet.</p>
          ) : (
            <fieldset className="flex min-w-0 flex-col gap-2">
              <legend className="mb-1 text-label-sm text-muted">Share with</legend>
              {candidates.map((person) => (
                <Checkbox
                  key={person.userId}
                  label={person.name}
                  checked={people.includes(person.userId)}
                  onCheckedChange={(checked) => setPeople((current) => (checked ? [...current, person.userId] : current.filter((id) => id !== person.userId)))}
                />
              ))}
            </fieldset>
          )
        ) : (
          <p className="text-body-sm text-muted">{audience === "everyone" ? "Anyone with an account on this server can open them." : "Nobody else can open them. Administrators can always see everyone's."}</p>
        )}
      </div>
    </Dialog>
  );
}
