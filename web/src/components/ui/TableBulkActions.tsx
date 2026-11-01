import { ActionMenuItems, resolveActions, type ActionSet } from "glass-ux/actions";
import { Button } from "glass-ux/button";
import { IconChevronDown } from "glass-ux/icons";
import { Menu } from "glass-ux/menu";

/** Keep bulk actions reachable when a table becomes a card list or a narrow split pane. */
export function TableBulkActions<T>({ set, rows, noun }: {
  set: ActionSet<T>;
  rows: T[];
  noun: { one: string; other: string };
}) {
  if (!rows.length) return null;
  const groups = resolveActions(set, rows, "bulk");
  if (!groups.length) return null;
  const label = `Actions for ${rows.length} selected ${rows.length === 1 ? noun.one : noun.other}`;
  return (
    <Menu.Root>
      <Menu.Trigger render={<Button size="sm" variant="secondary" aria-label={label} iconEnd={<IconChevronDown />}>Actions</Button>} />
      <Menu.Popup side="top" align="start" aria-label={label}>
        <ActionMenuItems groups={groups} targets={rows} surface="bulk" />
      </Menu.Popup>
    </Menu.Root>
  );
}
