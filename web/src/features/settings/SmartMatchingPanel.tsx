import { useMutation } from "@apollo/client/react";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { IconPlug } from "glass-ux/icons";
import { toast } from "glass-ux/toast";
import { z } from "zod";

import { Button, FieldGroup, Panel } from "@/components/ui";
import { EntityMatchingAssistantCreateDocument, EntityMatchingAssistantListDocument, EntityMatchingAssistantUpdateDocument, TestJevConnectionDocument, type EntityMatchingAssistantListQuery } from "@/graphql/generated/graphql";
import { assertSuccess, errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

const DEFAULT_MODEL = "jev-latest";

const schema = z.object({
  enabled: z.boolean(),
  apiKey: z.string().trim(),
  model: z.string().trim(),
});

type Row = NonNullable<EntityMatchingAssistantListQuery["matchingAssistants"]["edges"][number]["node"]>;

/**
 * TypeSafe's Jev, for release matches that names, tags and lengths can't settle. The key is written
 * here and never read back: the server stores it encrypted and only says whether it has one.
 */
export function SmartMatchingPanel() {
  const { data, refetch } = useQuery(EntityMatchingAssistantListDocument, { variables: { page: { limit: 1, offset: 0 } } });
  // Mounted once what is stored has arrived, so the form starts from it.
  if (!data) return null;
  const row = data.matchingAssistants.edges[0]?.node ?? null;
  return <SmartMatchingForm key={row?.id ?? "new"} row={row} refetch={refetch} />;
}

function SmartMatchingForm({ row, refetch }: { row: Row | null; refetch: () => Promise<unknown> }) {
  const [create] = useMutation(EntityMatchingAssistantCreateDocument);
  const [update] = useMutation(EntityMatchingAssistantUpdateDocument);
  const [test, testing] = useMutation(TestJevConnectionDocument);
  const hasKey = Boolean(row?.hasApiKey);
  const save = async (values: z.infer<typeof schema>) => {
    const model = values.model || DEFAULT_MODEL;
    if (row) {
      const { data: result } = await update({ variables: { id: row.id, input: { enabled: values.enabled, model, ...(values.apiKey ? { apiKey: values.apiKey } : {}) } } });
      assertSuccess(result?.updateMatchingAssistant, "Could not save smart matching");
    } else {
      const { data: result } = await create({ variables: { input: { enabled: values.enabled, model, apiKey: values.apiKey, hasApiKey: false } } });
      assertSuccess(result?.createMatchingAssistant, "Could not save smart matching");
    }
    await refetch();
  };
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: { enabled: row?.enabled ?? false, apiKey: "", model: row?.model ?? DEFAULT_MODEL },
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const values = schema.parse(value);
      await save(values);
      formApi.reset({ ...values, apiKey: "" });
      toast.success("Smart matching saved");
    },
  });
  const runTest = async () => {
    try {
      const { data: result } = await test({ variables: { apiKey: form.state.values.apiKey.trim() || null } });
      if (result?.testJevConnection.success) toast.success(`Connected. Models: ${result.testJevConnection.models.join(", ") || "none"}`);
      else toast.warning(result?.testJevConnection.error ?? "Jev test failed");
    } catch (error) { toast.danger(errorMessage(error)); }
  };
  const removeKey = async () => {
    if (!row) return;
    try {
      const { data: result } = await update({ variables: { id: row.id, input: { enabled: false, apiKey: "" } } });
      assertSuccess(result?.updateMatchingAssistant, "Could not remove the key");
      form.setFieldValue("enabled", false);
      await refetch();
      toast.success("Key removed");
    } catch (error) { toast.danger(errorMessage(error)); }
  };

  return (
    <Form form={form} className="flex min-w-0 flex-col gap-6" onError={(error) => toast.danger(errorMessage(error, "Could not save smart matching"))}>
      <form.Subscribe selector={(state) => state.isSubmitting}>
        {(submitting) => (
          <Panel
            title="Smart matching with Jev"
            description="Asked only when names, tags and lengths can't decide which book or track a file is. File and title names are sent to TypeSafe while this is on. The key is stored encrypted and never shown again."
            actions={<Button size="sm" variant="ghost" onPress={() => void runTest()} isPending={testing.loading}><IconPlug size="md" /> Test</Button>}
          >
            <FieldGroup columns={2}>
              <form.AppField name="enabled">{(field) => <field.SwitchRow label="Enable" className="sm:col-span-2" disabled={submitting} />}</form.AppField>
              <form.AppField name="apiKey">
                {(field) => <field.TextField label="API key" type="password" autoComplete="off" placeholder={hasKey ? "Saved. Enter a new key to replace it." : "From console.typesafe.ai"} disabled={submitting} />}
              </form.AppField>
              <form.AppField name="model">{(field) => <field.TextField label="Model" className="font-mono" placeholder={DEFAULT_MODEL} disabled={submitting} />}</form.AppField>
            </FieldGroup>
            <FormActions>
              {hasKey ? <Button size="sm" variant="ghost" onPress={() => void removeKey()}>Remove key</Button> : null}
              <form.SubmitButton disabled={submitting}>Save</form.SubmitButton>
            </FormActions>
          </Panel>
        )}
      </form.Subscribe>
    </Form>
  );
}
