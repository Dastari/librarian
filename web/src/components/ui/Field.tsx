import { Description, FieldError, Input, Label, ListBox, ListBoxItem, NumberField, Select, TextArea, TextField } from "@heroui/react";

import { GlassSwitch } from "./GlassControls";
import type { ReactNode } from "react";
import { Controller, type Control, type FieldPath, type FieldValues } from "react-hook-form";

import { cn } from "@/lib/utils";

/**
 * Form fields bound to react-hook-form. Every form in the app uses these so labels, help text
 * and errors always look and behave the same.
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
        <TextField
          name={field.name}
          value={field.value ?? ""}
          onChange={field.onChange}
          onBlur={field.onBlur}
          isInvalid={Boolean(fieldState.error)}
          isDisabled={isDisabled}
          isRequired={isRequired}
          type={type}
          autoComplete={autoComplete}
          className={cn("w-full", className)}
          fullWidth
        >
          <Label>{label}</Label>
          {multiline ? (
            <TextArea ref={field.ref} rows={rows} placeholder={placeholder} autoFocus={autoFocus} className={cn(mono && "font-mono text-label")} />
          ) : (
            <Input ref={field.ref} placeholder={placeholder} autoFocus={autoFocus} className={cn(mono && "font-mono text-label")} />
          )}
          {description ? <Description>{description}</Description> : null}
          <FieldError>{fieldState.error?.message}</FieldError>
        </TextField>
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

export function FormNumberField<TValues extends FieldValues>({ control, name, label, description, className, min, max, step, isDisabled, isRequired }: NumberFieldProps<TValues>) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field, fieldState }) => (
        <NumberField
          name={field.name}
          value={typeof field.value === "number" ? field.value : Number.NaN}
          onChange={(value) => field.onChange(Number.isNaN(value) ? null : value)}
          onBlur={field.onBlur}
          minValue={min}
          maxValue={max}
          step={step}
          isInvalid={Boolean(fieldState.error)}
          isDisabled={isDisabled}
          isRequired={isRequired}
          className={cn("w-full", className)}
          fullWidth
        >
          <Label>{label}</Label>
          <NumberField.Group>
            <NumberField.DecrementButton />
            <NumberField.Input ref={field.ref} />
            <NumberField.IncrementButton />
          </NumberField.Group>
          {description ? <Description>{description}</Description> : null}
          <FieldError>{fieldState.error?.message}</FieldError>
        </NumberField>
      )}
    />
  );
}

export interface SelectOption {
  key: string;
  label: ReactNode;
  description?: ReactNode;
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
        <Select
          name={field.name}
          selectedKey={field.value === null || field.value === undefined || field.value === "" ? null : String(field.value)}
          onSelectionChange={(key) => field.onChange(key === null ? null : String(key))}
          onBlur={field.onBlur}
          isInvalid={Boolean(fieldState.error)}
          isDisabled={isDisabled}
          isRequired={isRequired}
          placeholder={placeholder}
          className={cn("w-full", className)}
          fullWidth
        >
          <Label>{label}</Label>
          <Select.Trigger ref={field.ref}>
            <Select.Value />
            <Select.Indicator />
          </Select.Trigger>
          {description ? <Description>{description}</Description> : null}
          <FieldError>{fieldState.error?.message}</FieldError>
          <Select.Popover>
            <SelectList options={options} />
          </Select.Popover>
        </Select>
      )}
    />
  );
}

export function SelectList({ options }: { options: SelectOption[] }) {
  return (
    <ListBox>
      {options.map((option) => (
        <ListBoxItem key={option.key} id={option.key} textValue={typeof option.label === "string" ? option.label : option.key}>
          <span className="flex flex-col">
            <span>{option.label}</span>
            {option.description ? <span className="text-label-sm text-muted">{option.description}</span> : null}
          </span>
        </ListBoxItem>
      ))}
    </ListBox>
  );
}

interface SwitchFieldProps<TValues extends FieldValues> extends BaseFieldProps<TValues> {}

export function FormSwitchField<TValues extends FieldValues>({ control, name, label, description, className, isDisabled }: SwitchFieldProps<TValues>) {
  return (
    <Controller
      control={control}
      name={name}
      render={({ field }) => (
        <div className={cn("flex items-start justify-between gap-4 py-1", className)}>
          <div className="min-w-0">
            <p className="text-body-sm text-foreground">{label}</p>
            {description ? <p className="text-label-sm text-muted">{description}</p> : null}
          </div>
          <GlassSwitch checked={Boolean(field.value)} onChange={field.onChange} disabled={isDisabled} ariaLabel={label} />
        </div>
      )}
    />
  );
}

/** Groups related fields in a form with a consistent gap. */
export function FieldGroup({ children, className, columns = 1 }: { children: ReactNode; className?: string; columns?: 1 | 2 }) {
  return <div className={cn("grid gap-4", columns === 2 && "sm:grid-cols-2", className)}>{children}</div>;
}
