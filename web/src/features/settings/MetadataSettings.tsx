import { Input } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { useMutation } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { Form, FormActions, formBehaviour, useAppForm } from "glass-ux/form";
import { IconPlugConnected, IconTestPipe } from "@tabler/icons-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { z } from "zod";

import { Button, FieldGroup, KeyValueList, Panel } from "@/components/ui";
import { TestLlmParserDocument, TestOllamaConnectionDocument, TestTmdbConnectionDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS } from "@/lib/library-types";

import { asBool, asNumber, useAppSettings } from "./useAppSettings";

const requiredNumber = (rule: z.ZodNumber) => z.number().nullable().transform((value, context) => {
  if (value === null) {
    context.addIssue({ code: "custom", message: "Enter a number" });
    return z.NEVER;
  }
  return value;
}).pipe(rule);

const schema = z.object({
  tmdbEnabled: z.boolean(),
  tmdbApiKey: z.string().trim(),
  tvdbApiKey: z.string().trim(),
  tvmazeEnabled: z.boolean(),
  musicbrainzEnabled: z.boolean(),
  openlibraryEnabled: z.boolean(),
  autoFetch: z.boolean(),
  cacheTmdb: requiredNumber(z.number().int().min(0).max(365)),
  cacheTvmaze: requiredNumber(z.number().int().min(0).max(365)),
  cacheMusicbrainz: requiredNumber(z.number().int().min(0).max(365)),
  cacheOpenlibrary: requiredNumber(z.number().int().min(0).max(365)),
  llmEnabled: z.boolean(),
  ollamaUrl: z.string().trim(),
  ollamaModel: z.string().trim(),
  llmConfidence: requiredNumber(z.number().min(0).max(1)),
  llmTimeout: requiredNumber(z.number().int().min(1).max(600)),
  llmUseForAmbiguous: z.boolean(),
});
type Values = z.input<typeof schema>;

export function MetadataSettings() {
  const metadata = useAppSettings("metadata");
  const llm = useAppSettings("llm");
  const [testTmdb, tmdbTest] = useMutation(TestTmdbConnectionDocument);
  const [testOllama, ollamaTest] = useMutation(TestOllamaConnectionDocument);
  const [testParser, parserTest] = useMutation(TestLlmParserDocument);
  const [sample, setSample] = useState({ filename: "The.Matrix.1999.2160p.UHD.BluRay.x265-GROUP.mkv", libraryType: "movies" });

  const storedValues = useMemo<Values>(() => {
    const m = metadata.values;
    const l = llm.values;
    return {
      tmdbEnabled: asBool(m.get("metadata.tmdb_enabled"), true),
      tmdbApiKey: m.get("metadata.tmdb_api_key") ?? "",
      tvdbApiKey: m.get("metadata.tvdb_api_key") ?? "",
      tvmazeEnabled: asBool(m.get("metadata.tvmaze_enabled"), true),
      musicbrainzEnabled: asBool(m.get("metadata.musicbrainz_enabled"), true),
      openlibraryEnabled: asBool(m.get("metadata.openlibrary_enabled"), true),
      autoFetch: asBool(m.get("metadata.auto_fetch"), true),
      cacheTmdb: asNumber(m.get("metadata.cache_days.tmdb"), 7),
      cacheTvmaze: asNumber(m.get("metadata.cache_days.tvmaze"), 7),
      cacheMusicbrainz: asNumber(m.get("metadata.cache_days.musicbrainz"), 30),
      cacheOpenlibrary: asNumber(m.get("metadata.cache_days.openlibrary"), 30),
      llmEnabled: asBool(l.get("llm.enabled"), false),
      ollamaUrl: l.get("llm.ollama_url") || "http://localhost:11434",
      ollamaModel: l.get("llm.ollama_model") ?? "",
      llmConfidence: asNumber(l.get("llm.confidence_threshold"), 0.7),
      llmTimeout: asNumber(l.get("llm.timeout_seconds"), 30),
      llmUseForAmbiguous: asBool(l.get("llm.use_for_ambiguous"), true),
    };
  }, [metadata.values, llm.values]);
  const defaults = useRef(storedValues);
  const loading = metadata.loading || llm.loading;
  const form = useAppForm({
    ...formBehaviour,
    defaultValues: defaults.current,
    validators: { onDynamic: schema },
    onSubmit: async ({ value, formApi }) => {
      const values = schema.parse(value);
      await metadata.save({
        "metadata.tmdb_enabled": values.tmdbEnabled,
        "metadata.tmdb_api_key": values.tmdbApiKey || null,
        "metadata.tvdb_api_key": values.tvdbApiKey || null,
        "metadata.tvmaze_enabled": values.tvmazeEnabled,
        "metadata.musicbrainz_enabled": values.musicbrainzEnabled,
        "metadata.openlibrary_enabled": values.openlibraryEnabled,
        "metadata.auto_fetch": values.autoFetch,
        "metadata.cache_days.tmdb": values.cacheTmdb,
        "metadata.cache_days.tvmaze": values.cacheTvmaze,
        "metadata.cache_days.musicbrainz": values.cacheMusicbrainz,
        "metadata.cache_days.openlibrary": values.cacheOpenlibrary,
      });
      await llm.save({
        "llm.enabled": values.llmEnabled,
        "llm.ollama_url": values.ollamaUrl,
        "llm.ollama_model": values.ollamaModel || null,
        "llm.confidence_threshold": values.llmConfidence,
        "llm.timeout_seconds": values.llmTimeout,
        "llm.use_for_ambiguous": values.llmUseForAmbiguous,
      });
      defaults.current = values;
      formApi.reset(values);
      toast.success("Settings saved");
    },
  });
  useEffect(() => {
    if (loading || form.state.isSubmitting) return;
    const current = form.state.values;
    const edited = (Object.keys(storedValues) as Array<keyof Values>).filter((key) => !Object.is(current[key], defaults.current[key]));
    defaults.current = storedValues;
    form.reset(storedValues);
    for (const key of edited) form.setFieldValue(key, current[key]);
  }, [storedValues, loading, form]);

  const runTmdbTest = async () => {
    try {
      const { data } = await testTmdb({ variables: { input: { apiKey: form.state.values.tmdbApiKey } } });
      if (data?.testTmdbConnection.success) toast.success(data.testTmdbConnection.message);
      else toast.warning(data?.testTmdbConnection.message ?? "TMDB test failed");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  const runOllamaTest = async () => {
    try {
      const { data } = await testOllama({ variables: { input: { ollamaUrl: form.state.values.ollamaUrl } } });
      if (data?.testOllamaConnection.success) toast.success(`Connected. Models: ${data.testOllamaConnection.models.join(", ") || "none"}`);
      else toast.warning(data?.testOllamaConnection.error ?? "Ollama test failed");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <Form form={form} className="flex min-w-0 flex-col gap-6" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
      <form.Subscribe selector={(state) => [state.isSubmitting, state.isDefaultValue]}>
        {([submitting, unchanged]) => <>
          <Panel title="Movies · TMDB" actions={<Button size="sm" variant="ghost" onPress={() => void runTmdbTest()} isPending={tmdbTest.loading}><IconPlugConnected size={16} /> Test</Button>}>
            <FieldGroup>
              <form.AppField name="tmdbEnabled">{(field) => <field.SwitchRow label="Use TMDB for movies and collections" disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="tmdbApiKey">{(field) => <field.TextField label="API key" type="password" autoComplete="off" className="font-mono" hint="Required for movie libraries. Get one at themoviedb.org." disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="cacheTmdb">{(field) => <field.NumberField label="Cache results for (days)" min={0} max={365} disabled={loading || submitting} />}</form.AppField>
            </FieldGroup>
          </Panel>
          <Panel title="TV · TVmaze">
            <FieldGroup>
              <form.AppField name="tvmazeEnabled">{(field) => <field.SwitchRow label="Use TVmaze for shows and episodes" disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="tvdbApiKey">{(field) => <field.TextField label="TVDB API key" type="password" autoComplete="off" className="font-mono" hint="Optional, used for extra artwork." disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="cacheTvmaze">{(field) => <field.NumberField label="Cache results for (days)" min={0} max={365} disabled={loading || submitting} />}</form.AppField>
            </FieldGroup>
          </Panel>
          <Panel title="Music and books">
            <FieldGroup columns={2}>
              <form.AppField name="musicbrainzEnabled">{(field) => <field.SwitchRow label="MusicBrainz" disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="cacheMusicbrainz">{(field) => <field.NumberField label="Cache (days)" min={0} max={365} disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="openlibraryEnabled">{(field) => <field.SwitchRow label="Open Library" disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="cacheOpenlibrary">{(field) => <field.NumberField label="Cache (days)" min={0} max={365} disabled={loading || submitting} />}</form.AppField>
            </FieldGroup>
            <div className="mt-4">
              <form.AppField name="autoFetch">{(field) => <field.SwitchRow label="Fetch metadata automatically for scanned files" disabled={loading || submitting} />}</form.AppField>
            </div>
          </Panel>
          <Panel title="Filename parsing with Ollama" description="Optional fallback when filenames are ambiguous. Deterministic parsing always runs first." actions={<Button size="sm" variant="ghost" onPress={() => void runOllamaTest()} isPending={ollamaTest.loading}><IconPlugConnected size={16} /> Test</Button>}>
            <FieldGroup columns={2}>
              <form.AppField name="llmEnabled">{(field) => <field.SwitchRow label="Enable" className="sm:col-span-2" disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="ollamaUrl">{(field) => <field.TextField label="Ollama URL" className="font-mono" disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="ollamaModel">{(field) => <field.TextField label="Model" placeholder="llama3.1" className="font-mono" disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="llmConfidence">{(field) => <field.NumberField label="Confidence threshold" min={0} max={1} step={0.05} disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="llmTimeout">{(field) => <field.NumberField label="Timeout (seconds)" min={1} max={600} disabled={loading || submitting} />}</form.AppField>
              <form.AppField name="llmUseForAmbiguous">{(field) => <field.SwitchRow label="Only consult for ambiguous filenames" className="sm:col-span-2" disabled={loading || submitting} />}</form.AppField>
            </FieldGroup>
            <div className="mt-5 rounded-card border border-border bg-surface-secondary p-4">
              <p className="text-title-sm text-foreground">Try the parser</p>
              <p className="text-label-sm text-muted">Runs the filename through the deterministic parser and, when enabled, Ollama. It does not query metadata providers.</p>
              <div className="mt-3 flex flex-col gap-2 sm:flex-row">
                <Input value={sample.filename} onValueChange={(value) => setSample({ ...sample, filename: value })} aria-label="Sample filename" className="min-w-0 flex-1" />
                <Select value={sample.libraryType} onValueChange={(value) => { if (value) setSample({ ...sample, libraryType: value }); }} aria-label="Library type" className="min-w-0 sm:w-40" options={LIBRARY_TYPE_OPTIONS.map((option) => ({ value: option.type, label: option.label }))} />
                <Button variant="secondary" isPending={parserTest.loading} onPress={() => void testParser({ variables: { input: sample } }).catch((error) => toast.danger(errorMessage(error)))}>
                  <IconTestPipe size={16} /> Parse
                </Button>
              </div>
              {parserTest.data ? (
                <div className="mt-4">
                  <KeyValueList
                    columns={2}
                    items={[
                      { label: "Regex result", value: parserTest.data.testLlmParser.regexResult, mono: true },
                      { label: "LLM title", value: parserTest.data.testLlmParser.llmResult?.title ?? parserTest.data.testLlmParser.llmResult?.showTitle },
                      { label: "LLM year", value: parserTest.data.testLlmParser.llmResult?.year },
                      { label: "Season / episode", value: parserTest.data.testLlmParser.llmResult?.season !== null && parserTest.data.testLlmParser.llmResult?.season !== undefined ? `${parserTest.data.testLlmParser.llmResult.season} / ${parserTest.data.testLlmParser.llmResult.episode ?? "—"}` : undefined },
                      { label: "Confidence", value: parserTest.data.testLlmParser.llmResult?.confidence?.toFixed(2) },
                      { label: "Error", value: parserTest.data.testLlmParser.error },
                    ]}
                  />
                </div>
              ) : null}
            </div>
          </Panel>
          <FormActions>
            <form.CancelButton disabled={loading || submitting || unchanged} onCancel={() => { defaults.current = storedValues; form.reset(storedValues); }}>Discard</form.CancelButton>
            <form.SubmitButton disabled={loading || submitting}>Save</form.SubmitButton>
          </FormActions>
        </>}
      </form.Subscribe>
    </Form>
  );
}
