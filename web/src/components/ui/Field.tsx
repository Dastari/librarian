import { Field } from "glass-ux/field";
import { Input } from "glass-ux/input";
import { NumberField } from "glass-ux/number-field";
import { Select } from "glass-ux/select";
import { SettingRow } from "glass-ux/setting-row";
import { Switch } from "glass-ux/switch";
import { Textarea } from "glass-ux/textarea";
import type { ReactNode } from "react";
import { Controller, type Control, type FieldPath, type FieldValues } from "react-hook-form";

import { cn } from "@/lib/utils";

/**
 * Temporary react-hook-form bridge to bare Glass UX controls. Existing form schemas remain
 * the validation owner until each complete form migrates to useAppForm.
 */

interface BaseFieldProps<TValues extends FieldValues> {
  control: Control<TValues>;
  name: FieldPath<TValues>;
  label: string;
  description?: ReactNode;
  className?: string;
  isDisabled?: boolean;
  isRequired?: boolean;
}

interface TextFieldProps<TValues extends FieldValues> extends BaseFieldProps<TValues> {
  type?: "text" | "password" | "email" | "url" | "search";
  placeholder?: string;
  autoComplete?: string;
  multiline?: boolean;
  rows?: number;
  autoFocus?: boolean;
  mono?: boolean;
}

export function FormTextField<TValues extends FieldValues>({
  control,
  name,
  label,
  description,
  className,
  type = "text",
  placeholder,
  autoComplete,
  multiline,
  rows = 3,
  isDisabled,
  isRequired,
  autoFocus,
  mono,
}: TextFieldProps<TValues>) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field, fieldState }) => (
        <Field label={label} hint={description} error={fieldState.error?.message} required={isRequired} disabled={isDisabled} className={cn("w-full", className)}>
          {multiline ? (
            <Textarea
              ref={field.ref}
              name={field.name}
              value={field.value ?? ""}
              onValueChange={(value) => field.onChange(value)}
              onBlur={field.onBlur}
              required={isRequired}
              disabled={isDisabled}
              autoComplete={autoComplete}
              minRows={rows}
              maxRows={Math.max(rows, 8)}
              placeholder={placeholder}
              autoFocus={autoFocus}
              className={cn(mono && "font-mono")}
            />
          ) : (
            <Input
              ref={field.ref}
              name={field.name}
              value={field.value ?? ""}
              onValueChange={(value) => field.onChange(value)}
              onBlur={field.onBlur}
              type={type}
              required={isRequired}
              disabled={isDisabled}
              autoComplete={autoComplete}
              placeholder={placeholder}
              autoFocus={autoFocus}
              className={cn(mono && "font-mono")}
            />
          )}
        </Field>
      )}
    />
  );
}

interface NumberFieldProps<TValues extends FieldValues> extends BaseFieldProps<TValues> {
  min?: number;
  max?: number;
  step?: number;
  suffix?: string;
}

export function FormNumberField<TValues extends FieldValues>({ control, name, label, description, className, min, max, step, suffix, isDisabled, isRequired }: NumberFieldProps<TValues>) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field, fieldState }) => (
        <Field label={label} hint={description} error={fieldState.error?.message} required={isRequired} disabled={isDisabled} className={cn("w-full", className)}>
          <NumberField
            ref={field.ref}
            name={field.name}
            value={typeof field.value === "number" && !Number.isNaN(field.value) ? field.value : null}
            onValueChange={(value) => field.onChange(value)}
            onBlur={field.onBlur}
            min={min}
            max={max}
            step={step}
            unit={suffix}
            disabled={isDisabled}
            required={isRequired}
          />
        </Field>
      )}
    />
  );
}

export interface SelectOption {
  key: string;
  label: string;
  description?: string;
}

interface SelectFieldProps<TValues extends FieldValues> extends BaseFieldProps<TValues> {
  options: SelectOption[];
  placeholder?: string;
}

export function FormSelectField<TValues extends FieldValues>({ control, name, label, description, className, options, placeholder, isDisabled, isRequired }: SelectFieldProps<TValues>) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field, fieldState }) => (
        <Field label={label} hint={description} error={fieldState.error?.message} required={isRequired} disabled={isDisabled} nativeLabel={false} className={cn("w-full", className)}>
          <Select
            ref={field.ref}
            name={field.name}
            value={field.value === null || field.value === undefined || field.value === "" ? null : String(field.value)}
            onValueChange={(value) => field.onChange(value)}
            onBlur={field.onBlur}
            disabled={isDisabled}
            required={isRequired}
            placeholder={placeholder}
            options={options.map((option) => ({ value: option.key, label: option.label, detail: option.description }))}
          />
        </Field>
      )}
    />
  );
}

interface SwitchFieldProps<TValues extends FieldValues> extends BaseFieldProps<TValues> {}

export function FormSwitchField<TValues extends FieldValues>({ control, name, label, description, className, isDisabled }: SwitchFieldProps<TValues>) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field, fieldState }) => (
        <SettingRow
          label={label}
          description={description}
          className={className}
          disabled={isDisabled}
          status={fieldState.error ? "error" : undefined}
          error={fieldState.error?.message}
          control={
            <Switch
              ref={field.ref}
              name={field.name}
              checked={Boolean(field.value)}
              onCheckedChange={(checked) => field.onChange(checked)}
              onBlur={field.onBlur}
              disabled={isDisabled}
              invalid={Boolean(fieldState.error)}
            />
          }
        />
      )}
    />
  );
}

/** Groups related fields in a form with a consistent gap. */
export function FieldGroup({ children, className, columns = 1 }: { children: ReactNode; className?: string; columns?: 1 | 2 }) {
  return <div className={cn("grid gap-4", columns === 2 && "sm:grid-cols-2", className)}>{children}</div>;
}
