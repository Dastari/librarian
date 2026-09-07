import { useMutation } from "@apollo/client/react";
import { toast } from "@heroui/react";
import { zodResolver } from "@hookform/resolvers/zod";
import { IconPlugConnected, IconTestPipe } from "@tabler/icons-react";
import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button, FieldGroup, FormNumberField, FormSwitchField, FormTextField, KeyValueList, Panel } from "@/components/ui";
import { TestLlmParserDocument, TestOllamaConnectionDocument, TestTmdbConnectionDocument } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";
import { LIBRARY_TYPE_OPTIONS } from "@/lib/library-types";

import { SettingsForm } from "./SettingsForm";
import { asBool, asNumber, useAppSettings } from "./useAppSettings";

const schema = z.object({
  tmdbEnabled: z.boolean(),
  tmdbApiKey: z.string().trim(),
  tvdbApiKey: z.string().trim(),
  tvmazeEnabled: z.boolean(),
  musicbrainzEnabled: z.boolean(),
  openlibraryEnabled: z.boolean(),
  autoFetch: z.boolean(),
  cacheTmdb: z.number().int().min(0).max(365),
  cacheTvmaze: z.number().int().min(0).max(365),
  cacheMusicbrainz: z.number().int().min(0).max(365),
  cacheOpenlibrary: z.number().int().min(0).max(365),
  llmEnabled: z.boolean(),
  ollamaUrl: z.string().trim(),
  ollamaModel: z.string().trim(),
  llmConfidence: z.number().min(0).max(1),
  llmTimeout: z.number().int().min(1).max(600),
  llmUseForAmbiguous: z.boolean(),
});
type Values = z.infer<typeof schema>;

export function MetadataSettings() {
  const metadata = useAppSettings("metadata");
  const llm = useAppSettings("llm");
  const [testTmdb, tmdbTest] = useMutation(TestTmdbConnectionDocument);
  const [testOllama, ollamaTest] = useMutation(TestOllamaConnectionDocument);
  const [testParser, parserTest] = useMutation(TestLlmParserDocument);
  const [sample, setSample] = useState({ filename: "The.Matrix.1999.2160p.UHD.BluRay.x265-GROUP.mkv", libraryType: "movies" });

  const form = useForm<Values>({ resolver: zodResolver(schema), defaultValues: { tmdbEnabled: true, tmdbApiKey: "", tvdbApiKey: "", tvmazeEnabled: true, musicbrainzEnabled: true, openlibraryEnabled: true, autoFetch: true, cacheTmdb: 7, cacheTvmaze: 7, cacheMusicbrainz: 30, cacheOpenlibrary: 30, llmEnabled: false, ollamaUrl: "http://localhost:11434", ollamaModel: "", llmConfidence: 0.7, llmTimeout: 30, llmUseForAmbiguous: true } });

  useEffect(() => {
    const m = metadata.values;
    const l = llm.values;
    form.reset({
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
    });
  }, [metadata.values, llm.values, form]);

  const runTmdbTest = async () => {
    try {
      const { data } = await testTmdb({ variables: { input: { apiKey: form.getValues("tmdbApiKey") } } });
      if (data?.testTmdbConnection.success) toast.success(data.testTmdbConnection.message);
      else toast.warning(data?.testTmdbConnection.message ?? "TMDB test failed");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  const runOllamaTest = async () => {
    try {
      const { data } = await testOllama({ variables: { input: { ollamaUrl: form.getValues("ollamaUrl") } } });
      if (data?.testOllamaConnection.success) toast.success(`Connected. Models: ${data.testOllamaConnection.models.join(", ") || "none"}`);
      else toast.warning(data?.testOllamaConnection.error ?? "Ollama test failed");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };

  return (
    <SettingsForm
      form={form}
      onSave={async (values) => {
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
      }}
    >
      <Panel title="Movies · TMDB" actions={<Button size="sm" variant="ghost" onPress={() => void runTmdbTest()} isPending={tmdbTest.loading}><IconPlugConnected size={16} /> Test</Button>}>
        <FieldGroup>
          <FormSwitchField control={form.control} name="tmdbEnabled" label="Use TMDB for movies and collections" />
          <FormTextField control={form.control} name="tmdbApiKey" label="API key" type="password" autoComplete="off" mono description="Required for movie libraries. Get one at themoviedb.org." />
          <FormNumberField control={form.control} name="cacheTmdb" label="Cache results for (days)" min={0} max={365} />
        </FieldGroup>
      </Panel>
      <Panel title="TV · TVmaze">
        <FieldGroup>
          <FormSwitchField control={form.control} name="tvmazeEnabled" label="Use TVmaze for shows and episodes" />
          <FormTextField control={form.control} name="tvdbApiKey" label="TVDB API key" type="password" autoComplete="off" mono description="Optional, used for extra artwork." />
          <FormNumberField control={form.control} name="cacheTvmaze" label="Cache results for (days)" min={0} max={365} />
        </FieldGroup>
      </Panel>
      <Panel title="Music and books">
        <FieldGroup columns={2}>
          <FormSwitchField control={form.control} name="musicbrainzEnabled" label="MusicBrainz" />
          <FormNumberField control={form.control} name="cacheMusicbrainz" label="Cache (days)" min={0} max={365} />
          <FormSwitchField control={form.control} name="openlibraryEnabled" label="Open Library" />
          <FormNumberField control={form.control} name="cacheOpenlibrary" label="Cache (days)" min={0} max={365} />
        </FieldGroup>
        <div className="mt-4">
          <FormSwitchField control={form.control} name="autoFetch" label="Fetch metadata automatically for scanned files" />
        </div>
      </Panel>
      <Panel title="Filename parsing with Ollama" description="Optional fallback when filenames are ambiguous. Deterministic parsing always runs first." actions={<Button size="sm" variant="ghost" onPress={() => void runOllamaTest()} isPending={ollamaTest.loading}><IconPlugConnected size={16} /> Test</Button>}>
        <FieldGroup columns={2}>
          <FormSwitchField control={form.control} name="llmEnabled" label="Enable" className="sm:col-span-2" />
          <FormTextField control={form.control} name="ollamaUrl" label="Ollama URL" mono />
          <FormTextField control={form.control} name="ollamaModel" label="Model" placeholder="llama3.1" mono />
          <FormNumberField control={form.control} name="llmConfidence" label="Confidence threshold" min={0} max={1} step={0.05} />
          <FormNumberField control={form.control} name="llmTimeout" label="Timeout (seconds)" min={1} max={600} />
          <FormSwitchField control={form.control} name="llmUseForAmbiguous" label="Only consult for ambiguous filenames" className="sm:col-span-2" />
        </FieldGroup>
        <div className="mt-5 rounded-card border border-border bg-surface-secondary p-4">
          <p className="text-title-sm text-foreground">Try the parser</p>
          <p className="text-label-sm text-muted">Runs the filename through the deterministic parser and, when enabled, Ollama. It does not query metadata providers.</p>
          <div className="mt-3 flex flex-col gap-2 sm:flex-row">
            <input value={sample.filename} onChange={(event) => setSample({ ...sample, filename: event.target.value })} aria-label="Sample filename" className="nav-focus h-10 min-w-0 flex-1 rounded-lg border border-field-border bg-field px-3 font-mono text-label text-foreground" />
            <select value={sample.libraryType} onChange={(event) => setSample({ ...sample, libraryType: event.target.value })} aria-label="Library type" className="nav-focus h-10 rounded-lg border border-field-border bg-field px-3 text-body-sm text-foreground">
              {LIBRARY_TYPE_OPTIONS.map((option) => (
                <option key={option.type} value={option.type}>
                  {option.label}
                </option>
              ))}
            </select>
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
    </SettingsForm>
  );
}
