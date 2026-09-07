import { Button } from "@heroui/button";
import { Dropdown, DropdownTrigger, DropdownMenu, DropdownItem } from "@heroui/dropdown";
import { Tooltip } from "@heroui/tooltip";
import { IconDotsVertical } from "@tabler/icons-react";
import type { RowAction } from "./types";

export function RowActionsCell<T>({ row, actions }: { row: T; actions: RowAction<T>[] }) {
  const visible = actions.filter(action => !action.isVisible || action.isVisible(row));
  const inline = visible.filter(action => action.inDropdown === false);
  const menu = visible.filter(action => action.inDropdown !== false);
  const label = (action: RowAction<T>) => typeof action.label === "function" ? action.label(row) : action.label;
  const icon = (action: RowAction<T>) => typeof action.icon === "function" ? action.icon(row) : action.icon;
  return (
    <div className="flex items-center justify-end gap-1" onClick={event => event.stopPropagation()}>
      {inline.map(action => (
        <Tooltip key={action.key} content={label(action)}>
          <Button isIconOnly size="sm" variant="light" color={action.color}
            aria-label={label(action)} isDisabled={action.isDisabled?.(row)}
            className={action.alwaysVisible ? "" : "opacity-0 group-hover:opacity-100 group-focus-within:opacity-100 [@media(hover:none)]:opacity-100"}
            onPress={() => void action.onAction(row)}>
            {icon(action)}
          </Button>
        </Tooltip>
      ))}
      {menu.length > 0 && (
        <Dropdown>
          <DropdownTrigger><Button isIconOnly size="sm" variant="light" aria-label="Row actions"><IconDotsVertical size={16} /></Button></DropdownTrigger>
          <DropdownMenu aria-label="Row actions menu" onAction={key => {
            const action = menu.find(item => item.key === key);
            if (action) void action.onAction(row);
          }} disabledKeys={menu.filter(action => action.isDisabled?.(row)).map(action => action.key)}>
            {menu.map(action => <DropdownItem key={action.key} startContent={icon(action)} color={action.color}>{label(action)}</DropdownItem>)}
          </DropdownMenu>
        </Dropdown>
      )}
    </div>
  );
}
