import { toast } from "@heroui/react";
import { Button } from "@/components/ui";
import type { ReactNode } from "react";
import type { FieldValues, UseFormReturn } from "react-hook-form";

import { errorMessage } from "@/lib/graphql/errors";

interface SettingsFormProps<TValues extends FieldValues> {
  form: UseFormReturn<TValues>;
  onSave: (values: TValues) => Promise<void>;
  children: ReactNode;
  saving?: boolean;
}

/** Wraps a settings form with the standard Save/Discard footer and toast feedback. */
export function SettingsForm<TValues extends FieldValues>({ form, onSave, children, saving }: SettingsFormProps<TValues>) {
  const submit = form.handleSubmit(async (values) => {
    try {
      await onSave(values);
      toast.success("Settings saved");
      form.reset(values);
    } catch (error) {
      toast.danger(errorMessage(error, "Could not save settings"));
    }
  });
  return (
    <form onSubmit={submit} noValidate className="flex flex-col gap-6">
      {children}
      <div className="flex justify-end gap-2">
        <Button variant="ghost" onPress={() => form.reset()} isDisabled={!form.formState.isDirty || saving || form.formState.isSubmitting}>
          Discard
        </Button>
        <Button type="submit" variant="primary" isPending={saving || form.formState.isSubmitting} isDisabled={!form.formState.isDirty}>
          Save
        </Button>
      </div>
    </form>
  );
}
