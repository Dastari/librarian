import { Button } from "glass-ux/button";
import { Field } from "glass-ux/field";
import { useBoundField, useFormContext } from "glass-ux/form";
import { IconLock, IconUnlock } from "glass-ux/icons";
import { Input, type InputProps } from "glass-ux/input";
import { TagInput } from "glass-ux/tag-input";
import { Textarea } from "glass-ux/textarea";
import { Tooltip } from "glass-ux/tooltip";
import type { ReactNode } from "react";

/**
 * A form field's lock. Locks live in the form's `lockedFields` value, by field name, which is the
 * GraphQL name the backend leaves alone on a refresh. A form without that value shows no locks.
 * Editing a field locks it, as Plex does, because an edit a refresh undoes is no edit at all.
 */
function useFieldLock<T>() {
  const bound = useBoundField<T>();
  const form = useFormContext();
  const { field } = bound;
  const locks = () => form.getFieldValue("lockedFields" as never) as string[] | undefined;
  const setLocked = (next: boolean) => {
    const current = locks();
    if (!current || next === current.includes(field.name)) return;
    form.setFieldValue("lockedFields" as never, (next ? [...current, field.name] : current.filter((name) => name !== field.name)) as never);
  };
  const toggle: ReactNode = (
    <form.Subscribe selector={(state) => (state.values as { lockedFields?: string[] }).lockedFields}>
      {(current) => (current ? <LockToggle locked={current.includes(field.name)} onToggle={() => setLocked(!current.includes(field.name))} /> : null)}
    </form.Subscribe>
  );
  const change = (value: T) => {
    field.handleChange(value);
    setLocked(true);
  };
  return { ...bound, toggle, change };
}

function LockToggle({ locked, onToggle }: { locked: boolean; onToggle: () => void }) {
  return (
    <Tooltip content={locked ? "Locked: a metadata refresh leaves this alone" : "A metadata refresh can change this"}>
      <Button
        variant="ghost"
        size="xs"
        icon={locked ? <IconLock /> : <IconUnlock />}
        aria-label={locked ? "Unlock this field" : "Lock this field"}
        aria-pressed={locked}
        data-locked={locked ? "" : undefined}
        className="metadata-lock"
        onClick={onToggle}
      />
    </Tooltip>
  );
}

interface FieldText {
  label: string;
  hint?: ReactNode;
  required?: boolean;
  placeholder?: string;
}

/** A line of text, with its lock in the input's end adornment. */
export function MetadataTextField({ label, hint, required, placeholder, inputMode }: FieldText & Pick<InputProps, "inputMode">) {
  const { field, root, error, toggle, change } = useFieldLock<string>();
  return (
    <Field {...root} label={label} hint={hint} required={required}>
      <Input name={field.name} value={field.state.value} onValueChange={change} onBlur={field.handleBlur} required={required} placeholder={placeholder} inputMode={inputMode} trailing={toggle} aria-invalid={error ? true : undefined} />
    </Field>
  );
}

/** Long text. A text area has no adornment, so its lock sits at the end of the label's line. */
export function MetadataTextareaField({ label, hint, placeholder, minRows = 4 }: FieldText & { minRows?: number }) {
  const { field, root, toggle, change } = useFieldLock<string>();
  return (
    <Field {...root} label={label} hint={hint} aside={toggle}>
      <Textarea name={field.name} value={field.state.value} onValueChange={change} onBlur={field.handleBlur} placeholder={placeholder} minRows={minRows} maxRows={12} />
    </Field>
  );
}

/** A list of tags, with its lock at the end of the label's line. */
export function MetadataTagsField({ label, hint, placeholder }: FieldText) {
  const { field, root, toggle, change } = useFieldLock<string[]>();
  return (
    <Field {...root} label={label} hint={hint} aside={toggle}>
      <TagInput name={field.name} value={field.state.value} onValueChange={change} onBlur={field.handleBlur} placeholder={placeholder} />
    </Field>
  );
}
