import { useMutation } from "@apollo/client/react";
import { formBehaviour, useAppForm } from "glass-ux/form";
import { toast } from "glass-ux/toast";
import { useEffect } from "react";
import { z } from "zod";

import { Button, Dialog } from "@/components/ui";
import { EntityLibraryCreateDocument } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS } from "@/lib/library-types";

import { LibraryLocationField } from "./LibraryLocationField";

const schema = z.object({
  name: z.string().trim().min(1, "Give the library a name").max(80),
  libraryType: z.string().min(1, "Choose a type"),
  path: z.string().trim().min(1, "Choose the folder, or the server and share"),
  autoScan: z.boolean(),
  autoOrganize: z.boolean(),
});

const EMPTY = { name: "", libraryType: "movies", path: "", autoScan: true, autoOrganize: false };
const TYPE_OPTIONS = LIBRARY_TYPE_OPTIONS.map((meta) => ({ value: meta.type, label: meta.label, icon: <meta.icon size={16} stroke={1.75} aria-hidden /> }));

/** A new library: what it holds, where, and whether Librarian may move its files. */
export function AddLibraryDialog({ open, onOpenChange, onCreated }: { open: boolean; onOpenChange: (open: boolean) => void; onCreated?: () => void }) {
  const { user } = useSession();
  const [create] = useMutation(EntityLibraryCreateDocument);
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: EMPTY,
    validators: { onDynamic: schema },
    onSubmit: async ({ value }) => {
      if (!user) return;
      try {
        const { data } = await create({ variables: { input: { userId: user.id, ...value, namingPattern: "", scanIntervalMinutes: 60, watchForChanges: false, scanning: false } } });
        assertSuccess(data?.createLibrary, "Could not create the library");
        toast.success(`${value.name} created. Run a scan to import existing files.`);
        onCreated?.();
        onOpenChange(false);
      } catch (error) {
        toast.danger(errorMessage(error));
      }
    },
  });
  useEffect(() => { if (open) form.reset(EMPTY); }, [open]); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <Dialog
      isOpen={open}
      onOpenChange={onOpenChange}
      title="Add library"
      size="md"
      footer={(
        <>
          <Button variant="ghost" onPress={() => onOpenChange(false)}>Cancel</Button>
          <form.Subscribe selector={(state) => state.isSubmitting}>
            {(saving) => <Button variant="primary" isPending={saving} onPress={() => void form.handleSubmit()}>Create</Button>}
          </form.Subscribe>
        </>
      )}
    >
      <form.AppForm>
        <form noValidate className="flex flex-col gap-3" onSubmit={(event) => { event.preventDefault(); void form.handleSubmit(); }}>
          <form.AppField name="name">{(field) => <field.TextField label="Name" placeholder="Movies" required />}</form.AppField>
          <form.AppField name="libraryType">{(field) => <field.SelectField label="Type" options={TYPE_OPTIONS} required />}</form.AppField>
          <form.AppField name="path">{() => <LibraryLocationField localPlaceholder="/data/media/Movies" required />}</form.AppField>
          <form.AppField name="autoScan">{(field) => <field.SwitchRow label="Scan on a schedule" description="Look for new, changed and missing files automatically." />}</form.AppField>
          <form.AppField name="autoOrganize">{(field) => <field.SwitchRow label="Organize files" description="Rename and move matched files into the naming pattern." />}</form.AppField>
        </form>
      </form.AppForm>
    </Dialog>
  );
}
