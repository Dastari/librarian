import { ByteSizeInput } from "glass-ux/byte-size-field";
import { DurationInput } from "glass-ux/duration-field";
import { formBehaviour, useBoundField } from "glass-ux/form";
import { Input } from "glass-ux/input";
import { NumberField } from "glass-ux/number-field";
import { PercentInput } from "glass-ux/percent-field";
import { Select, type SelectOptions } from "glass-ux/select";
import { SettingRow } from "glass-ux/setting-row";
import { UrlInput } from "glass-ux/url-field";
import type { ComponentProps, ReactNode } from "react";

import { cn } from "@/lib/utils";

/**
 * Bound settings for a `useAppForm` field: label and help on the left, the control on the right,
 * the message in the row's own description slot. They are the row-shaped counterparts of the
 * library's bound fields (`field.NumberField` stacks a label above its input), so a settings page
 * reads as one list of rows. Switches and segmented choices already come as `field.SwitchRow` and
 * `field.SegmentedRow`.
 *
 * Each takes the value contract of its Glass control: seconds for a duration, bytes for a size, a
 * fraction for a percentage. The form schema stays the validation owner.
 */

/** Moves focus to the first row that failed validation, in page order. */
function focusFirstInvalidRow(formId: string) {
  requestAnimationFrame(() => {
    const form = document.querySelector(`[data-form="${CSS.escape(formId)}"]`);
    const row = form?.querySelector(".fa-setting-row[data-status='error'], .fa-field[data-invalid]");
    const target = row?.querySelector<HTMLElement>("input:not([type='hidden']):not([tabindex='-1']):not([aria-hidden='true']), textarea, button:not([tabindex='-1']), [role='slider'], [tabindex='0']");
    target?.focus({ focusVisible: true });
  });
}

/**
 * `formBehaviour` for a form made of setting rows: the library's focus-first-invalid only looks for
 * `Field`s, and a row reports its error on itself.
 */
export const settingsFormBehaviour = {
  ...formBehaviour,
  onSubmitInvalid: ({ formApi }: { formApi: { formId: string } }) => focusFirstInvalidRow(formApi.formId),
};

interface RowChrome {
  label: ReactNode;
  description?: ReactNode;
  badge?: ReactNode;
  disabled?: boolean;
  className?: string;
}

/**
 * Control widths inside a row, from a count to an address. The row is a container: while it is too
 * narrow to hold text and control side by side the control wraps under the text, and takes the full
 * width there instead of a short box stranded at one end.
 */
const WIDTH = { count: "w-full @min-[26rem]:w-44", text: "w-full @min-[34rem]:w-72", address: "w-full @min-[34rem]:w-80" } as const;

function Row({ label, description, badge, disabled, className, error, children }: RowChrome & { error: string | undefined; children: ReactNode }) {
  return <SettingRow label={label} description={description} badge={badge} disabled={disabled} className={className} status={error ? "error" : undefined} error={error} control={children} />;
}

type NumberProps = Pick<ComponentProps<typeof NumberField>, "min" | "max" | "step" | "largeStep" | "unit" | "placeholder" | "format">;

/** A count or measure with bounds: days, seeders, a threshold. */
export function NumberRow({ label, description, badge, disabled, className, ...control }: RowChrome & NumberProps) {
  const { field, error } = useBoundField<number | null>();
  return (
    <Row label={label} description={description} badge={badge} disabled={disabled} className={className} error={error}>
      <NumberField {...control} className={WIDTH.count} name={field.name} value={field.state.value} onValueChange={(value) => field.handleChange(value)} onBlur={field.handleBlur} disabled={disabled} invalid={Boolean(error)} />
    </Row>
  );
}

/** A ratio kept as a fraction (0.8 reads 80 %), the way the server stores it. */
export function PercentRow({ label, description, badge, disabled, className, ...control }: RowChrome & Pick<ComponentProps<typeof PercentInput>, "min" | "max" | "step" | "decimals">) {
  const { field, error } = useBoundField<number | null>();
  return (
    <Row label={label} description={description} badge={badge} disabled={disabled} className={className} error={error}>
      <PercentInput {...control} scale="fraction" className={WIDTH.count} name={field.name} value={field.state.value} onValueChange={(value) => field.handleChange(value)} onBlur={field.handleBlur} disabled={disabled} invalid={Boolean(error)} />
    </Row>
  );
}

/** An interval in seconds, written and shown as "5 minutes" or "90 s". */
export function DurationRow({ label, description, badge, disabled, className, ...control }: RowChrome & Pick<ComponentProps<typeof DurationInput>, "units" | "defaultUnit" | "placeholder">) {
  const { field, error } = useBoundField<number | null>();
  return (
    <Row label={label} description={description} badge={badge} disabled={disabled} className={className} error={error}>
      <DurationInput {...control} className={WIDTH.count} name={field.name} value={field.state.value} onValueChange={(value) => field.handleChange(value)} onBlur={field.handleBlur} disabled={disabled} invalid={Boolean(error)} />
    </Row>
  );
}

/** A size in bytes, written and shown as "700 MB" or "4.5 GB". */
export function ByteSizeRow({ label, description, badge, disabled, className, ...control }: RowChrome & Pick<ComponentProps<typeof ByteSizeInput>, "base" | "defaultUnit" | "placeholder" | "clearable">) {
  const { field, error } = useBoundField<number | null>();
  return (
    <Row label={label} description={description} badge={badge} disabled={disabled} className={className} error={error}>
      <ByteSizeInput {...control} className={WIDTH.count} name={field.name} value={field.state.value} onValueChange={(value) => field.handleChange(value)} onBlur={field.handleBlur} disabled={disabled} invalid={Boolean(error)} />
    </Row>
  );
}

type TextProps = Pick<ComponentProps<typeof Input>, "placeholder" | "type" | "autoComplete" | "maxLength" | "spellCheck">;

/** Short free text: a name, an ISO code, a model. `mono` for identifiers and patterns. */
export function TextRow({ label, description, badge, disabled, className, mono, ...control }: RowChrome & TextProps & { mono?: boolean }) {
  const { field, error } = useBoundField<string>();
  return (
    <Row label={label} description={description} badge={badge} disabled={disabled} className={className} error={error}>
      <Input {...control} className={cn(WIDTH.text, mono && "font-mono")} name={field.name} value={field.state.value} onValueChange={(value) => field.handleChange(value)} onBlur={field.handleBlur} disabled={disabled} invalid={Boolean(error)} />
    </Row>
  );
}

/** A web address. A missing scheme is added on blur. Pass `schemes` to allow more than http(s). */
export function UrlRow({ label, description, badge, disabled, className, ...control }: RowChrome & Pick<ComponentProps<typeof UrlInput>, "schemes" | "defaultScheme" | "requireTld" | "openable" | "placeholder">) {
  const { field, error } = useBoundField<string>();
  return (
    <Row label={label} description={description} badge={badge} disabled={disabled} className={className} error={error}>
      <UrlInput {...control} className={cn(WIDTH.address, "font-mono")} name={field.name} value={field.state.value} onValueChange={(value) => field.handleChange(value)} onBlur={field.handleBlur} disabled={disabled} invalid={Boolean(error)} />
    </Row>
  );
}

/** A choice among more than a handful, or whose options carry detail. Two to four short ones use `field.SegmentedRow`. */
export function SelectRow<V extends string = string>({ label, description, badge, disabled, className, options, placeholder }: RowChrome & { options: SelectOptions<V>; placeholder?: string }) {
  const { field, error } = useBoundField<V>();
  return (
    <Row label={label} description={description} badge={badge} disabled={disabled} className={className} error={error}>
      <Select<V> className={WIDTH.text} options={options} placeholder={placeholder} name={field.name} value={field.state.value} onValueChange={(value) => field.handleChange(value as V)} onBlur={field.handleBlur} disabled={disabled} invalid={Boolean(error)} />
    </Row>
  );
}
