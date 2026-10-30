import { useState } from "react";
import { Switch } from "glass-ux/switch";
import { SettingRow } from "glass-ux/setting-row";
import { IconChevronUp, IconChevronDown } from "glass-ux/icons";
import { Button, Dialog, ErrorState } from "@/components/ui";
import { errorMessage } from "@/lib/graphql/errors";
import { useHomeShelves, usePersonalLists } from "./usePersonalLibrary";

export function HomeCustomization({ open, onOpenChange }: { open: boolean; onOpenChange: (open: boolean) => void }) {
  const home = useHomeShelves();
  const lists = usePersonalLists();
  const [pending, setPending] = useState(false);
  const [failure, setFailure] = useState<string>();
  const apply = async (action: () => Promise<void>) => { setPending(true); setFailure(undefined); try { await action(); } catch (error) { setFailure(errorMessage(error)); } finally { setPending(false); } };
  return <Dialog isOpen={open} onOpenChange={onOpenChange} title="Customize Home" isDismissable={!pending} footer={<Button onPress={() => onOpenChange(false)} isDisabled={pending}>Done</Button>}>
    {home.error ? <ErrorState error={home.error} onRetry={() => void home.refetch()} /> : null}
    <div className="flex min-w-0 flex-col gap-2">
      {home.shelves.map((savedShelf, index) => {
        const shelf = { ...savedShelf, label: savedShelf.listId ? lists.rows.find((list) => list.id === savedShelf.listId)?.name ?? savedShelf.label : savedShelf.label };
        return <SettingRow key={shelf.key} label={shelf.label} control={<div className="flex items-center gap-1">
        <Button variant="ghost" aria-label={`Move ${shelf.label} up`} isDisabled={pending || index === 0 || !home.complete} onPress={() => void apply(async () => {
          const previous = home.shelves[index - 1]!;
          const before = home.shelves[index - 2];
          const position = before ? Math.floor((before.position + previous.position) / 2) : previous.position - 1024;
          if (position < 0 || (before && previous.position - before.position < 2)) {
            const ordered = [...home.shelves]; ordered.splice(index, 1); ordered.splice(index - 1, 0, savedShelf);
            for (const [i, item] of ordered.entries()) await home.save(item.key, { ...item, position: i * 1024 });
          } else await home.save(shelf.key, { ...shelf, position });
        })}><IconChevronUp /></Button>
        <Button variant="ghost" aria-label={`Move ${shelf.label} down`} isDisabled={pending || index === home.shelves.length - 1 || !home.complete} onPress={() => void apply(async () => {
          const next = home.shelves[index + 1]!;
          const after = home.shelves[index + 2];
          const position = after ? Math.floor((next.position + after.position) / 2) : next.position + 1024;
          if (after && after.position - next.position < 2) {
            const ordered = [...home.shelves]; ordered.splice(index, 1); ordered.splice(index + 1, 0, savedShelf);
            for (const [i, item] of ordered.entries()) await home.save(item.key, { ...item, position: i * 1024 });
          } else await home.save(shelf.key, { ...shelf, position });
        })}><IconChevronDown /></Button>
        <Switch aria-label={`Show ${shelf.label}`} checked={shelf.visible} disabled={pending || !home.complete} onCheckedChange={(visible) => void apply(() => home.save(shelf.key, { ...shelf, visible }))} />
      </div>} />; })}
    </div>
    {failure ? <p role="alert" className="text-body-sm text-danger">{failure}</p> : null}
  </Dialog>;
}
