import { useMutation } from "@apollo/client/react";
import { useEffect, useState } from "react";
import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { toast } from "glass-ux/toast";

import { Button, Dialog, ErrorState } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityListEntryCreateDocument, EntityListEntryDeleteDocument, PersonalEntriesDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { targetFields, type PersonalTarget } from "./types";
import { usePersonalLists } from "./usePersonalLibrary";

export function SaveToListDialog({ open, onOpenChange, target, title }: { open: boolean; onOpenChange: (open: boolean) => void; target: PersonalTarget; title: string }) {
  const { user } = useSession();
  const lists = usePersonalLists();
  const [listId, setListId] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string>();
  const entries = useCompleteConnection(PersonalEntriesDocument, { listId: listId ?? "" }, (data) => data.listEntries, { skip: !open || !listId });
  const [add] = useMutation(EntityListEntryCreateDocument);
  const [remove] = useMutation(EntityListEntryDeleteDocument);
  useEffect(() => { if (!open) { setListId(null); setFailure(undefined); setName(""); } }, [open]);
  const existing = entries.rows.find((entry) => entry[`${target.kind}Id`] === target.id);
  const run = async (action: () => Promise<void>) => {
    setPending(true); setFailure(undefined);
    try { await action(); } catch (error) { setFailure(errorMessage(error)); } finally { setPending(false); }
  };
  const append = async (destination: string, position: number) => {
    if (!user) throw new Error("Sign in to save this item");
    const { data } = await add({ variables: { input: { userId: user.id, listId: destination, position, ...targetFields(target) } } });
    assertSuccess(data?.createListEntry, "Could not save item");
    toast.success("Saved to your list"); onOpenChange(false);
  };
  return <Dialog isOpen={open} onOpenChange={onOpenChange} title={`Save ${title}`} description="Personal lists do not change download settings." isDismissable={!pending}
    footer={<><Button variant="ghost" onPress={() => onOpenChange(false)} isDisabled={pending}>Cancel</Button><Button variant="primary" isPending={pending} isDisabled={!listId || !entries.complete} onPress={() => void run(async () => {
      if (existing) { const { data } = await remove({ variables: { id: existing.id } }); assertSuccess(data?.deleteListEntry, "Could not remove item"); toast.success("Removed from your list"); onOpenChange(false); }
      else await append(listId!, Math.max(-1, ...entries.rows.map((entry) => entry.position)) + 1);
    })}>{existing ? "Remove from list" : "Save"}</Button></>}>
    <div className="flex min-w-0 flex-col gap-4">
      {lists.error ? <ErrorState error={lists.error} onRetry={() => void lists.refetch()} /> : null}
      <Button variant="secondary" isPending={pending} isDisabled={!lists.complete} onPress={() => void run(async () => {
        const saved = lists.rows.find((list) => list.kind === "WATCH_LATER") ?? await lists.createList("Watch Later", "WATCH_LATER");
        setListId(saved.id);
      })}>Choose Watch Later</Button>
      <Field label="List" nativeLabel={false}><Select value={listId} onValueChange={setListId} disabled={pending || lists.loading} options={lists.rows.filter((list) => list.kind !== "SMART").map((list) => ({ value: list.id, label: list.name }))} width="full" /></Field>
      <form className="flex min-w-0 items-start gap-2" onSubmit={(event) => { event.preventDefault(); void run(async () => { const created = await lists.createList(name); setName(""); setListId(created.id); }); }}>
        <Field label="New list" className="min-w-0 flex-1"><Input value={name} onValueChange={setName} required disabled={pending} /></Field>
        <Button type="submit" className="mt-6" isDisabled={!name.trim() || pending}>Create</Button>
      </form>
      {entries.error ? <ErrorState error={entries.error} onRetry={() => void entries.refetch()} /> : null}
      {failure ? <p role="alert" className="text-body-sm text-danger">{failure}</p> : null}
    </div>
  </Dialog>;
}
