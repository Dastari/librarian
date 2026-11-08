import { Input } from "glass-ux/input";
import { Select } from "glass-ux/select";
import { useMutation, useQuery } from "@apollo/client/react";
import { toast } from "glass-ux/toast";
import { Form, FormActions, useAppForm } from "glass-ux/form";
import { IconPlug } from "glass-ux/icons";
import { IconTestPipe } from "@tabler/icons-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { z } from "zod";

import { Button, KeyValueList } from "@/components/ui";
import { ListeningStatusDocument, TestLlmParserDocument, TestOllamaConnectionDocument, TestTmdbConnectionDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS } from "@/lib/library-types";

import { NumberRow, PercentRow, DurationRow, TextRow, UrlRow, settingsFormBehaviour } from "./SettingFields";
import { SettingsPage, SettingsSection } from "./SettingsSection";
import { SmartMatchingPanel } from "./SmartMatchingPanel";
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
  audibleEnabled: z.boolean(),
  audnexusEnabled: z.boolean(),
  listeningEnabled: z.boolean(),
  autoFetch: z.boolean(),
  cacheTmdb: requiredNumber(z.number().int().min(0).max(365)),
  cacheTvmaze: requiredNumber(z.number().int().min(0).max(365)),
  cacheMusicbrainz: requiredNumber(z.number().int().min(0).max(365)),
  cacheOpenlibrary: requiredNumber(z.number().int().min(0).max(365)),
  cacheAudible: requiredNumber(z.number().int().min(0).max(365)),
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
  const listening = useQuery(ListeningStatusDocument);
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
      audibleEnabled: asBool(m.get("metadata.audible_enabled"), true),
      audnexusEnabled: asBool(m.get("metadata.audnexus_enabled"), true),
      listeningEnabled: asBool(m.get("metadata.listening_enabled"), true),
      autoFetch: asBool(m.get("metadata.auto_fetch"), true),
      cacheTmdb: asNumber(m.get("metadata.cache_days.tmdb"), 7),
      cacheTvmaze: asNumber(m.get("metadata.cache_days.tvmaze"), 7),
      cacheMusicbrainz: asNumber(m.get("metadata.cache_days.musicbrainz"), 30),
      cacheOpenlibrary: asNumber(m.get("metadata.cache_days.openlibrary"), 30),
      cacheAudible: asNumber(m.get("metadata.cache_days.audible"), 7),
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
    ...settingsFormBehaviour,
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
        "metadata.audible_enabled": values.audibleEnabled,
        "metadata.audnexus_enabled": values.audnexusEnabled,
        "metadata.listening_enabled": values.listeningEnabled,
        "metadata.auto_fetch": values.autoFetch,
        "metadata.cache_days.tmdb": values.cacheTmdb,
        "metadata.cache_days.tvmaze": values.cacheTvmaze,
        "metadata.cache_days.musicbrainz": values.cacheMusicbrainz,
        "metadata.cache_days.openlibrary": values.cacheOpenlibrary,
        "metadata.cache_days.audible": values.cacheAudible,
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
    <SettingsPage>
      <Form form={form} className="settings-measure flex min-w-0 flex-col gap-8" onError={(error) => toast.danger(errorMessage(error, "Could not save settings"))}>
        <form.Subscribe selector={(state) => [state.isSubmitting, state.isDefaultValue]}>
          {([submitting, unchanged]) => {
            const off = loading || submitting;
            return <>
              <SettingsSection title="Movies · TMDB" width="readable" actions={<Button size="sm" variant="ghost" onPress={() => void runTmdbTest()} isPending={tmdbTest.loading}><IconPlug size="md" /> Test</Button>}>
                <form.AppField name="tmdbEnabled">{(field) => <field.SwitchRow label="Use TMDB for movies and collections" disabled={off} />}</form.AppField>
                <form.AppField name="tmdbApiKey">{() => <TextRow label="API key" type="password" autoComplete="off" mono description="Required for movie libraries. Get one at themoviedb.org." disabled={off} />}</form.AppField>
                <form.AppField name="cacheTmdb">{() => <NumberRow label="Cache results for" unit="days" min={0} max={365} disabled={off} />}</form.AppField>
              </SettingsSection>
              <SettingsSection title="TV · TVmaze" width="readable">
                <form.AppField name="tvmazeEnabled">{(field) => <field.SwitchRow label="Use TVmaze for shows and episodes" disabled={off} />}</form.AppField>
                <form.AppField name="tvdbApiKey">{() => <TextRow label="TVDB API key" type="password" autoComplete="off" mono description="Optional, used for extra artwork." disabled={off} />}</form.AppField>
                <form.AppField name="cacheTvmaze">{() => <NumberRow label="Cache results for" unit="days" min={0} max={365} disabled={off} />}</form.AppField>
              </SettingsSection>
              <SettingsSection title="Music and books" width="readable">
                <form.AppField name="musicbrainzEnabled">{(field) => <field.SwitchRow label="MusicBrainz" disabled={off} />}</form.AppField>
                <form.AppField name="cacheMusicbrainz">{() => <NumberRow label="Cache MusicBrainz results for" unit="days" min={0} max={365} disabled={off} />}</form.AppField>
                <form.AppField name="audibleEnabled">{(field) => <field.SwitchRow label="Audible" description="Audiobook search, details and lengths. Searched first." disabled={off} />}</form.AppField>
                <form.AppField name="cacheAudible">{() => <NumberRow label="Cache Audible results for" unit="days" min={0} max={365} disabled={off} />}</form.AppField>
                <form.AppField name="audnexusEnabled">{(field) => <field.SwitchRow label="Audnexus" description="Chapter lists for Audible's books. Cached with Audible." disabled={off} />}</form.AppField>
                <form.AppField name="openlibraryEnabled">{(field) => <field.SwitchRow label="Open Library" description="Audiobooks, when Audible has nothing." disabled={off} />}</form.AppField>
                <form.AppField name="cacheOpenlibrary">{() => <NumberRow label="Cache Open Library results for" unit="days" min={0} max={365} disabled={off} />}</form.AppField>
              </SettingsSection>
              <SettingsSection title="Fetching and listening" width="readable">
                <form.AppField name="autoFetch">{(field) => <field.SwitchRow label="Fetch metadata automatically for scanned files" disabled={off} />}</form.AppField>
                <form.AppField name="listeningEnabled">
                  {(field) => <field.SwitchRow label="Listen to audiobooks to identify them and check their chapters" description={<>Runs on this server, and nothing leaves it. Hears what a book's opening says about it, and where its chapters are announced.{listening.data?.listeningStatus.detail ? <> {listening.data.listeningStatus.detail}</> : null}</>} disabled={off} />}
                </form.AppField>
              </SettingsSection>
              <SettingsSection title="Filename parsing with Ollama" width="readable" description="Optional fallback when filenames are ambiguous. Deterministic parsing always runs first." actions={<Button size="sm" variant="ghost" onPress={() => void runOllamaTest()} isPending={ollamaTest.loading}><IconPlug size="md" /> Test</Button>}>
                <form.AppField name="llmEnabled">{(field) => <field.SwitchRow label="Enable" disabled={off} />}</form.AppField>
                <form.AppField name="ollamaUrl">{() => <UrlRow label="Ollama URL" schemes={["http", "https"]} defaultScheme="http" requireTld={false} openable={false} disabled={off} />}</form.AppField>
                <form.AppField name="ollamaModel">{() => <TextRow label="Model" placeholder="llama3.1" mono disabled={off} />}</form.AppField>
                <form.AppField name="llmConfidence">{() => <PercentRow label="Confidence threshold" min={0} max={1} step={0.05} disabled={off} />}</form.AppField>
                <form.AppField name="llmTimeout">{() => <DurationRow label="Timeout" units={["m", "s"]} defaultUnit="s" disabled={off} />}</form.AppField>
                <form.AppField name="llmUseForAmbiguous">{(field) => <field.SwitchRow label="Only consult for ambiguous filenames" disabled={off} />}</form.AppField>
              </SettingsSection>
              <FormActions>
                <form.CancelButton disabled={off || unchanged} onCancel={() => { defaults.current = storedValues; form.reset(storedValues); }}>Discard</form.CancelButton>
                <form.SubmitButton disabled={off}>Save</form.SubmitButton>
              </FormActions>
            </>;
          }}
        </form.Subscribe>
      </Form>

      <SettingsSection title="Try the parser" width="readable" description="Runs the filename through the deterministic parser and, when enabled, Ollama. It does not query metadata providers.">
        <div className="flex flex-col gap-2 sm:flex-row">
          <Input value={sample.filename} onValueChange={(value) => setSample({ ...sample, filename: value })} aria-label="Sample filename" className="min-w-0 flex-1" />
          <Select value={sample.libraryType} onValueChange={(value) => { if (value) setSample({ ...sample, libraryType: value }); }} aria-label="Library type" className="min-w-0 sm:w-40" options={LIBRARY_TYPE_OPTIONS.map((option) => ({ value: option.type, label: option.label }))} />
          <Button variant="secondary" isPending={parserTest.loading} onPress={() => void testParser({ variables: { input: sample } }).catch((error) => toast.danger(errorMessage(error)))}>
            <IconTestPipe size={16} /> Parse
          </Button>
        </div>
        {parserTest.data ? (
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
        ) : null}
      </SettingsSection>

      <SmartMatchingPanel />
    </SettingsPage>
  );
}
