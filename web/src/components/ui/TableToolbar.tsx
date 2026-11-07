import type { ReactNode } from "react";

/**
 * What goes in a table's toolbar: a segmented filter and the table's other controls, on one line.
 * Its section of the table's toolbar has no natural width (overrides.css), so the filter folds to a
 * menu when there's no room rather than the toolbar wrapping to make room for it.
 */
export function TableToolbar({ before, filter, children }: { before?: ReactNode; filter?: ReactNode; children?: ReactNode }) {
  return (
    <div className="table-toolbar flex min-w-0 flex-1 items-center justify-end gap-2">
      {before}
      {filter ? <div className="flex min-w-24 flex-1 justify-end">{filter}</div> : <div className="flex-1" />}
      {children}
    </div>
  );
}
