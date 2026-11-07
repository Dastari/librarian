import { Button } from "glass-ux/button";
import { Field } from "glass-ux/field";
import { useBoundField } from "glass-ux/form";
import { IconFolder } from "glass-ux/icons";
import { Input } from "glass-ux/input";
import { Tooltip } from "glass-ux/tooltip";
import { useState } from "react";

import { FolderPickerDialog } from "@/features/files/FolderPickerDialog";

/** A folder path, typed or browsed for, with the browse button inside the input. Goes in a form's field. */
export function FolderField({ label = "Folder", placeholder, required }: { label?: string; placeholder?: string; required?: boolean }) {
  const { field, root, error } = useBoundField<string>();
  const [browsing, setBrowsing] = useState(false);
  return (
    <>
      <Field {...root} label={label} required={required}>
        <Input
          name={field.name}
          value={field.state.value}
          onValueChange={(value) => field.handleChange(value)}
          onBlur={field.handleBlur}
          placeholder={placeholder}
          required={required}
          spellCheck={false}
          className="font-mono"
          aria-invalid={error ? true : undefined}
          trailing={<Tooltip content="Browse folders"><Button variant="ghost" size="xs" icon={<IconFolder />} aria-label="Browse folders" onClick={() => setBrowsing(true)} /></Tooltip>}
        />
      </Field>
      <FolderPickerDialog isOpen={browsing} onOpenChange={setBrowsing} initialPath={field.state.value} onPick={(path) => field.handleChange(path)} />
    </>
  );
}
