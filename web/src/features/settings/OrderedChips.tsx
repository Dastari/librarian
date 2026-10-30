import { IconArrowDown, IconArrowUp, IconClose, IconPlus } from "glass-ux/icons";
import { Input } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { useState } from "react";
import { Controller, type Control, type FieldPath, type FieldValues } from "react-hook-form";

import { Button } from "@/components/ui";
import { cn } from "@/lib/utils";

interface OrderedChipsProps {
  label: string;
  description?: string;
  value: string[];
  onChange: (next: string[]) => void;
  /** Fixed choices; without them the control accepts free text. */
  options?: Array<{ key: string; label: string }>;
  placeholder?: string;
  className?: string;
  /** Label for an entry, e.g. an ISO code rendered as a language name. */
  renderLabel?: (item: string) => string;
  /** Shown in place of the list when it is empty. Omit when the description already says it. */
  emptyLabel?: string;
}

/**
 * An ordered list of short values (release groups, resolutions, languages). Order is the
 * preference order, so every row can move up and down.
 */
export function OrderedChips({ label, description, value, onChange, options, placeholder, className, renderLabel, emptyLabel }: OrderedChipsProps) {
  const [draft, setDraft] = useState("");
  const remaining = options?.filter((option) => !value.includes(option.key)) ?? [];

  const add = (item: string) => {
    const trimmed = item.trim();
    if (!trimmed || value.includes(trimmed)) return;
    onChange([...value, trimmed]);
    setDraft("");
  };
  const move = (index: number, delta: number) => {
    const next = [...value];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target]!, next[index]!];
    onChange(next);
  };

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div>
        <p className="text-body-sm text-foreground">{label}</p>
        {description ? <p className="text-label-sm text-muted">{description}</p> : null}
      </div>
      {value.length > 0 ? (
        <ol className="flex flex-col gap-1">
          {value.map((item, index) => (
            <li key={item} className="flex items-center gap-2 py-1">
              <span className="text-numeric w-5 shrink-0 text-label-sm text-muted">{index + 1}</span>
              <span className="min-w-0 flex-1 truncate text-body-sm text-foreground">{renderLabel ? renderLabel(item) : item}</span>
              <Button size="sm" variant="ghost" isIconOnly aria-label={`Move ${item} up`} isDisabled={index === 0} onPress={() => move(index, -1)}>
                <IconArrowUp />
              </Button>
              <Button size="sm" variant="ghost" isIconOnly aria-label={`Move ${item} down`} isDisabled={index === value.length - 1} onPress={() => move(index, 1)}>
                <IconArrowDown />
              </Button>
              <Button size="sm" variant="ghost" isIconOnly aria-label={`Remove ${item}`} onPress={() => onChange(value.filter((entry) => entry !== item))}>
                <IconClose />
              </Button>
            </li>
          ))}
        </ol>
      ) : emptyLabel ? (
        <p className="text-label-sm text-muted">{emptyLabel}</p>
      ) : null}
      {options ? (
        remaining.length > 0 ? (
          <Select
            aria-label={`Add to ${label}`}
            value={null}
            onValueChange={(item) => item !== null && add(item)}
            placeholder={placeholder ?? "Add…"}
            options={remaining.map((option) => ({ value: option.key, label: option.label }))}
          />
        ) : null
      ) : (
        <div className="flex gap-2">
          <Input
            aria-label={`Add to ${label}`}
            value={draft}
            onValueChange={setDraft}
            className="min-w-0 flex-1"
            placeholder={placeholder ?? "Add an entry"}
            onKeyDown={(event) => {
              if (event.key !== "Enter") return;
              event.preventDefault();
              add(draft);
            }}
          />
          <Button variant="secondary" isDisabled={!draft.trim()} onPress={() => add(draft)}>
            <IconPlus /> Add
          </Button>
        </div>
      )}
    </div>
  );
}

interface FormOrderedChipsProps<TValues extends FieldValues> extends Omit<OrderedChipsProps, "value" | "onChange"> {
  control: Control<TValues>;
  name: FieldPath<TValues>;
}

/** `OrderedChips` bound to react-hook-form. */
export function FormOrderedChips<TValues extends FieldValues>({ control, name, ...props }: FormOrderedChipsProps<TValues>) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field }) => <OrderedChips {...props} value={Array.isArray(field.value) ? (field.value as string[]) : []} onChange={field.onChange} />}
    />
  );
}
