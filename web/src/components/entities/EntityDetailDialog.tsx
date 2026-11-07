import { Dialog } from "glass-ux/dialog";
import { DetailTile } from "glass-ux/detail-tile";
import { ScrollArea } from "glass-ux/scroll-area";
import { Tabs } from "glass-ux/tabs";
import type { ReactNode } from "react";

export interface DetailSection {
  key: string;
  label: string;
  count?: number;
  content: ReactNode;
}

interface EntityDetailDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Accessible name of the kind of thing: "Download details". */
  label: string;
  /** The record's own `EntityIdentity` at its header tier. A long name truncates; the actions never move. */
  identity: ReactNode;
  actions?: ReactNode;
  /** States and links under the identity: import status, what it belongs to. */
  badges?: ReactNode;
  /** Something that needs attention, above everything else. */
  notice?: ReactNode;
  /** The few facts someone opened this to see. Always in view. */
  summary?: ReactNode;
  /** The rest, a tab each, scrolling on their own under the summary. */
  sections: readonly DetailSection[];
  finalFocus?: Parameters<typeof Dialog.Popup>[0]["finalFocus"];
}

/**
 * The detail dialog for a record: its identity with its actions in one line at the top, the facts
 * that matter, and everything else in tabs. A fixed height means changing tab never resizes the
 * dialog. Downloads and files share it, so they read alike.
 */
export function EntityDetailDialog({ open, onOpenChange, label, identity, actions, badges, notice, summary, sections, finalFocus }: EntityDetailDialogProps) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Popup size="xl" kind="settings" aria-label={label} finalFocus={finalFocus}>
        <Dialog.Header identity={identity} className="entity-detail-header">{actions}</Dialog.Header>
        <Dialog.Body flush className="entity-detail flex min-h-0 flex-1 flex-col">
          <div className="flex shrink-0 flex-col gap-4 px-(--inset-panel) pb-4">
            {badges ? <div className="flex min-w-0 flex-wrap items-center gap-2">{badges}</div> : null}
            {notice}
            {summary}
          </div>
          <Tabs.Root defaultValue={sections[0]?.key} className="flex min-h-0 flex-1 flex-col">
            <div className="shrink-0 px-(--inset-panel)">
              <Tabs.List aria-label="Details" items={sections.map((section) => ({ value: section.key, label: section.label, count: section.count }))} />
            </div>
            <ScrollArea aria-label="Details content" className="min-h-0 flex-1">
              {sections.map((section) => <Tabs.Panel key={section.key} value={section.key} className="px-(--inset-panel) py-4">{section.content}</Tabs.Panel>)}
            </ScrollArea>
          </Tabs.Root>
        </Dialog.Body>
      </Dialog.Popup>
    </Dialog.Root>
  );
}

/** Facts as tiles that reflow with the dialog's width. */
export function DetailGrid({ children, label }: { children: ReactNode; label?: string }) {
  return <div role="group" aria-label={label} className="grid grid-cols-[repeat(auto-fit,minmax(9.5rem,1fr))] gap-3">{children}</div>;
}

export { DetailTile };
