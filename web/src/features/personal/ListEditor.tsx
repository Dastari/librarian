import { useMutation } from "@apollo/client/react";
import { Form, FormActions, FormStack, formBehaviour, useAppForm } from "glass-ux/form";
import { toast } from "glass-ux/toast";
import { z } from "zod";
import { Dialog } from "@/components/ui";
import { useCompleteConnection } from "@/features/libraries/browser/useCompleteConnection";
import { EntityLibraryListDocument, EntityPersonalListUpdateDocument, EntitySavedViewCreateDocument, EntitySavedViewDeleteDocument, EntitySavedViewUpdateDocument, type SavedViewFieldsFragment } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { usePersonalLists } from "./usePersonalLibrary";

const schema = z.object({ name: z.string().trim().min(1, "Enter a name").max(120), kind: z.enum(["MANUAL", "SMART"]), contentType: z.enum(["MOVIE", "SHOW", "ALBUM", "AUDIOBOOK", "TRACK"]), libraryId: z.string(), titleContains: z.string(), availability: z.enum(["ANY", "AVAILABLE", "MISSING"]), sort: z.enum(["TITLE", "CREATED_AT", "UPDATED_AT"]), descending: z.boolean() });
type Values = z.infer<typeof schema>;

export function ListEditor({ open, onOpenChange, initial, personalListId }: { open: boolean; onOpenChange: (open: boolean) => void; initial?: SavedViewFieldsFragment; personalListId?: string }) {
  const { user } = useSession();
  const lists = usePersonalLists();
  const libraries = useCompleteConnection(EntityLibraryListDocument, { where: { userId: { eq: user?.id ?? "" } } }, (data) => data.libraries);
  const [createView] = useMutation(EntitySavedViewCreateDocument);
  const [deleteView] = useMutation(EntitySavedViewDeleteDocument);
  const [updateView] = useMutation(EntitySavedViewUpdateDocument);
  const [updateList] = useMutation(EntityPersonalListUpdateDocument);
  const defaults: Values = { name: initial?.name ?? "", kind: initial ? "SMART" : "MANUAL", contentType: initial?.contentType ?? "MOVIE", libraryId: initial?.libraryId ?? "", titleContains: initial?.titleContains ?? "", availability: initial?.availability ?? "ANY", sort: initial?.sort ?? "TITLE", descending: initial?.descending ?? false };
  const form = useAppForm({ ...formBehaviour, defaultValues: defaults, validators: { onDynamic: schema }, onSubmit: async ({ value }) => {
    if (!user) throw new Error("Sign in to create a list");
    if (value.kind === "MANUAL") await lists.createList(value.name);
    else {
      const input = { name: value.name.trim(), version: 1, contentType: value.contentType, libraryId: value.libraryId || null, titleContains: value.titleContains.trim() || null, availability: value.contentType === "SHOW" ? "ANY" as const : value.availability, sort: value.sort, descending: value.descending };
      if (initial) {
        const { data } = await updateView({ variables: { id: initial.id, input } }); assertSuccess(data?.updateSavedView, "Could not save rules");
        if (personalListId) { const { data: renamed } = await updateList({ variables: { id: personalListId, input: { name: input.name } } }); assertSuccess(renamed?.updatePersonalList, "Rules saved, but the list could not be renamed"); }
      }
      else {
        const { data } = await createView({ variables: { input: { userId: user.id, ...input } } });
        const saved = assertSuccess(data?.createSavedView, "Could not save rules").savedView;
        if (!saved) throw new Error("Saved rules were not returned");
        try { await lists.createList(value.name, "SMART", saved.id); }
        catch (error) { await deleteView({ variables: { id: saved.id } }).catch(() => {}); throw error; }
      }
    }
    toast.success(initial ? "Rules saved" : "List created"); onOpenChange(false);
  } });
  return <Dialog isOpen={open} onOpenChange={onOpenChange} title={initial ? "Edit collection rules" : "New list or collection"}>
    <Form form={form} onError={(error) => toast.danger(errorMessage(error))}><FormStack>
      <form.AppField name="name">{(field) => <field.TextField label="Name" required />}</form.AppField>
      {!initial ? <form.AppField name="kind">{(field) => <field.SelectField label="Type" options={[{ value: "MANUAL", label: "Manual list / playlist" }, { value: "SMART", label: "Smart collection" }]} />}</form.AppField> : null}
      <form.Subscribe selector={(state) => state.values}>{(values) => values.kind === "SMART" ? <>
        <form.AppField name="contentType">{(field) => <field.SelectField label="Media" options={[{ value: "MOVIE", label: "Movies" }, { value: "SHOW", label: "Shows" }, { value: "ALBUM", label: "Albums" }, { value: "AUDIOBOOK", label: "Audiobooks" }, { value: "TRACK", label: "Tracks" }]} />}</form.AppField>
        <form.AppField name="libraryId">{(field) => <field.SelectField label="Library" options={[{ value: "", label: "All my libraries" }, ...libraries.rows.map((library) => ({ value: library.id, label: library.name }))]} />}</form.AppField>
        <form.AppField name="titleContains">{(field) => <field.TextField label="Title contains" optional />}</form.AppField>
        <form.AppField name="availability">{(field) => <field.SelectField label="Availability" disabled={values.contentType === "SHOW"} hint={values.contentType === "SHOW" ? "Show rules include all matching shows." : undefined} options={[{ value: "ANY", label: "Any" }, { value: "AVAILABLE", label: "Has a file" }, { value: "MISSING", label: "Missing files" }]} />}</form.AppField>
        <form.AppField name="sort">{(field) => <field.SelectField label="Sort by" options={[{ value: "TITLE", label: "Title" }, { value: "CREATED_AT", label: "Date added" }, { value: "UPDATED_AT", label: "Date updated" }]} />}</form.AppField>
        <form.AppField name="descending">{(field) => <field.CheckboxField label="Descending order" />}</form.AppField>
      </> : null}</form.Subscribe>
      <FormActions><form.SubmitButton>{initial ? "Save rules" : "Create"}</form.SubmitButton></FormActions>
    </FormStack></Form>
  </Dialog>;
}
