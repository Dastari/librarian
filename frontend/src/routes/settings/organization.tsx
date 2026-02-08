import { createFileRoute } from "@tanstack/react-router";
import { useState, useEffect, useCallback, useMemo } from "react";
import { Card, CardBody, CardHeader } from "@heroui/card";
import { Tooltip } from "@heroui/tooltip";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { Spinner } from "@heroui/spinner";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
} from "@heroui/modal";
import { Input, Textarea } from "@heroui/input";
import { Select, SelectItem } from "@heroui/select";
import { Switch } from "@heroui/switch";
import { Accordion, AccordionItem } from "@heroui/accordion";
import { Divider } from "@heroui/divider";
import { addToast } from "@heroui/toast";
import {
  IconPlus,
  IconTrash,
  IconStarFilled,
  IconTemplate,
  IconPencil,
  IconBrain,
  IconServer,
  IconTestPipe,
  IconRefresh,
  IconCheck,
  IconX,
  IconClock,
  IconMovie,
  IconDeviceTv,
  IconMusic,
  IconBook,
} from "@tabler/icons-react";
import {
  OrganizationNamingPatternsDocument,
  OrganizationCreateNamingPatternDocument,
  OrganizationUpdateNamingPatternDocument,
  OrganizationDeleteNamingPatternDocument,
  type OrganizationNamingPatternsQuery,
  LlmAppSettingsDocument,
  CreateAppSettingDocument,
  UpdateAppSettingDocument,
  type LlmAppSettingsQuery,
} from "../../lib/graphql/generated/graphql";
import {
  type LlmParserSettings,
  type TestFilenameParserResult,
  type FilenameParseResult,
} from "../../lib/graphql";
import { apolloClient, useQuery, useMutation } from "../../lib/graphql/client";
import {
  DataTable,
  type DataTableColumn,
  type CardRendererProps,
  type RowAction,
} from "../../components/data-table";
import { previewNamingPattern, sanitizeError } from "../../lib/format";
import { SettingsHeader } from "../../components/shared";

type NamingPatternRow =
  OrganizationNamingPatternsQuery["NamingPatterns"]["Edges"][number]["Node"];

const LLM_CATEGORY = "llm";
const LLM_KEYS = {
  enabled: "llm.enabled",
  ollamaUrl: "llm.ollama_url",
  ollamaModel: "llm.ollama_model",
  timeoutSeconds: "llm.timeout_seconds",
  temperature: "llm.temperature",
  maxTokens: "llm.max_tokens",
  promptTemplate: "llm.prompt_template",
  confidenceThreshold: "llm.confidence_threshold",
  modelMovies: "llm.model.movies",
  modelTv: "llm.model.tv",
  modelMusic: "llm.model.music",
  modelAudiobooks: "llm.model.audiobooks",
  promptMovies: "llm.prompt.movies",
  promptTv: "llm.prompt.tv",
  promptMusic: "llm.prompt.music",
  promptAudiobooks: "llm.prompt.audiobooks",
} as const;

export const Route = createFileRoute("/settings/organization")({
  component: OrganizationSettingsPage,
});

// Pattern variables configuration by library type
const PATTERN_VARIABLES = {
  common: [
    { var: "{year}", desc: "Release year" },
    { var: "{ext}", desc: "File extension" },
    { var: "{original}", desc: "Original filename" },
  ],
  tv: [
    { var: "{show}", desc: "Show name" },
    { var: "{season}", desc: "Season number" },
    { var: "{season:02}", desc: "Season (zero-padded)" },
    { var: "{episode}", desc: "Episode number" },
    { var: "{episode:02}", desc: "Episode (zero-padded)" },
    { var: "{title}", desc: "Episode title" },
  ],
  movies: [
    { var: "{title}", desc: "Movie title" },
    { var: "{quality}", desc: "Quality info" },
  ],
  music: [
    { var: "{artist}", desc: "Artist name" },
    { var: "{album}", desc: "Album name" },
    { var: "{track}", desc: "Track number" },
    { var: "{track:02}", desc: "Track (zero-padded)" },
    { var: "{title}", desc: "Track title" },
    { var: "{disc}", desc: "Disc number" },
  ],
  audiobooks: [
    { var: "{author}", desc: "Author name" },
    { var: "{title}", desc: "Book title" },
    { var: "{series}", desc: "Series name" },
    { var: "{series_position}", desc: "Series position" },
    { var: "{chapter}", desc: "Chapter number" },
    { var: "{chapter:02}", desc: "Chapter (zero-padded)" },
    { var: "{chapter_title}", desc: "Chapter title" },
    { var: "{narrator}", desc: "Narrator" },
  ],
};

function appSettingsToLlmSettings(
  edges: LlmAppSettingsQuery["AppSettings"]["Edges"],
): LlmParserSettings {
  const map = new Map(edges.map((e) => [e.Node.Key, e.Node.Value]));
  const get = (key: string, fallback: string) => map.get(key) ?? fallback;
  const toNum = (key: string, fallback: number) => {
    const parsed = Number.parseFloat(get(key, String(fallback)));
    return Number.isFinite(parsed) ? parsed : fallback;
  };
  const toOpt = (key: string) => {
    const value = map.get(key);
    return value && value.length > 0 ? value : null;
  };

  return {
    enabled: get(LLM_KEYS.enabled, "false") === "true",
    ollamaUrl: get(LLM_KEYS.ollamaUrl, "http://localhost:11434"),
    ollamaModel: get(LLM_KEYS.ollamaModel, "qwen2.5-coder:7b"),
    timeoutSeconds: toNum(LLM_KEYS.timeoutSeconds, 30),
    temperature: toNum(LLM_KEYS.temperature, 0.1),
    maxTokens: toNum(LLM_KEYS.maxTokens, 256),
    promptTemplate: get(LLM_KEYS.promptTemplate, ""),
    confidenceThreshold: toNum(LLM_KEYS.confidenceThreshold, 0.7),
    modelMovies: toOpt(LLM_KEYS.modelMovies),
    modelTv: toOpt(LLM_KEYS.modelTv),
    modelMusic: toOpt(LLM_KEYS.modelMusic),
    modelAudiobooks: toOpt(LLM_KEYS.modelAudiobooks),
    promptMovies: toOpt(LLM_KEYS.promptMovies),
    promptTv: toOpt(LLM_KEYS.promptTv),
    promptMusic: toOpt(LLM_KEYS.promptMusic),
    promptAudiobooks: toOpt(LLM_KEYS.promptAudiobooks),
  };
}

function llmSettingsKeyToIdMap(
  edges: LlmAppSettingsQuery["AppSettings"]["Edges"],
): Map<string, string> {
  return new Map(edges.map((e) => [e.Node.Key, e.Node.Id]));
}

function buildSimpleFilenameParseResult(filename: string): FilenameParseResult {
  const clean = filename.replace(/\.[a-z0-9]{2,4}$/i, "");
  const yearMatch = clean.match(/\b(19\d{2}|20\d{2})\b/);
  const seMatch = clean.match(/[sS](\d{1,2})[eE](\d{1,2})/);
  const title = clean
    .replace(/[._-]+/g, " ")
    .replace(/\b(19\d{2}|20\d{2})\b/g, "")
    .replace(/[sS]\d{1,2}[eE]\d{1,2}.*/, "")
    .trim();

  return {
    mediaType: seMatch ? "tv" : "movie",
    title: title || null,
    year: yearMatch ? Number.parseInt(yearMatch[1], 10) : null,
    season: seMatch ? Number.parseInt(seMatch[1], 10) : null,
    episode: seMatch ? Number.parseInt(seMatch[2], 10) : null,
    episodeEnd: null,
    resolution: /2160p/i.test(clean)
      ? "2160p"
      : /1080p/i.test(clean)
        ? "1080p"
        : /720p/i.test(clean)
          ? "720p"
          : null,
    source: /bluray/i.test(clean)
      ? "bluray"
      : /web[- .]?dl|webrip/i.test(clean)
        ? "web"
        : null,
    videoCodec: /x265|h\.?265|hevc/i.test(clean)
      ? "h265"
      : /x264|h\.?264/i.test(clean)
        ? "h264"
        : null,
    audio: /dts|aac|ac3|truehd/i.test(clean)
      ? (clean.match(/dts|aac|ac3|truehd/i)?.[0] ?? null)
      : null,
    hdr: /hdr|dolby.?vision|dv/i.test(clean) ? "HDR" : null,
    releaseGroup: clean.includes("-")
      ? (clean.split("-").at(-1) ?? null)
      : null,
    edition: null,
    completeSeries: /complete/i.test(clean),
    confidence: seMatch || yearMatch ? 0.7 : 0.45,
  };
}

function OrganizationSettingsPage() {
  // ============================================================================
  // Naming Patterns State
  // ============================================================================
  const [patterns, setPatterns] = useState<NamingPatternRow[]>([]);
  const [isLoadingPatterns, setIsLoadingPatterns] = useState(true);
  const [patternsError, setPatternsError] = useState<string | null>(null);

  // Modal state for naming patterns
  const {
    isOpen: isAddOpen,
    onOpen: onAddOpen,
    onClose: onAddClose,
  } = useDisclosure();
  const {
    isOpen: isEditOpen,
    onOpen: onEditOpen,
    onClose: onEditClose,
  } = useDisclosure();
  const {
    isOpen: isDeleteOpen,
    onOpen: onDeleteOpen,
    onClose: onDeleteClose,
  } = useDisclosure();

  const [selectedPattern, setSelectedPattern] =
    useState<NamingPatternRow | null>(null);
  const [formData, setFormData] = useState({
    name: "",
    pattern: "",
    description: "",
    libraryType: "tv",
  });
  const [editFormData, setEditFormData] = useState({
    name: "",
    pattern: "",
    description: "",
    libraryType: "tv",
  });
  const [isSavingPattern, setIsSavingPattern] = useState(false);

  // ============================================================================
  // LLM Parser State
  // ============================================================================
  const [originalLlmSettings, setOriginalLlmSettings] =
    useState<LlmParserSettings | null>(null);
  const [llmSettingIds, setLlmSettingIds] = useState<Map<string, string>>(
    new Map(),
  );
  const [isLoadingLlm, setIsLoadingLlm] = useState(true);
  const [isSavingLlm, setIsSavingLlm] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [isParsing, setIsParsing] = useState(false);

  // LLM Form state
  const [llmEnabled, setLlmEnabled] = useState(false);
  const [ollamaUrl, setOllamaUrl] = useState("http://localhost:11434");
  const [ollamaModel, setOllamaModel] = useState("qwen2.5-coder:7b");
  const [timeoutSeconds, setTimeoutSeconds] = useState(30);
  const [temperature, setTemperature] = useState(0.1);
  const [maxTokens, setMaxTokens] = useState(256);
  const [promptTemplate, setPromptTemplate] = useState("");
  const [confidenceThreshold, setConfidenceThreshold] = useState(0.7);
  const [modelMovies, setModelMovies] = useState<string>("");
  const [modelTv, setModelTv] = useState<string>("");
  const [modelMusic, setModelMusic] = useState<string>("");
  const [modelAudiobooks, setModelAudiobooks] = useState<string>("");

  // Connection test state
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [connectionStatus, setConnectionStatus] = useState<
    "untested" | "success" | "error"
  >("untested");
  const [connectionError, setConnectionError] = useState<string | null>(null);

  // Parser test state
  const [testFilename, setTestFilename] = useState(
    "The.Matrix.1999.REMASTERED.2160p.UHD.BluRay.x265.10bit.HDR.TrueHD.7.1.Atmos-FGT",
  );
  const [parseResult, setParseResult] =
    useState<TestFilenameParserResult | null>(null);
  const [createNamingPattern] = useMutation(
    OrganizationCreateNamingPatternDocument,
  );
  const [updateNamingPattern] = useMutation(
    OrganizationUpdateNamingPatternDocument,
  );
  const [deleteNamingPattern] = useMutation(
    OrganizationDeleteNamingPatternDocument,
  );
  const [createAppSetting] = useMutation(CreateAppSettingDocument);
  const [updateAppSetting] = useMutation(UpdateAppSettingDocument);
  const { data: llmSettingsData, loading: llmSettingsLoading } = useQuery(
    LlmAppSettingsDocument,
    {
      fetchPolicy: "network-only",
      notifyOnNetworkStatusChange: false,
      variables: {},
    },
  );

  // Track LLM changes
  const hasLlmChanges = useMemo(() => {
    if (!originalLlmSettings) return false;
    return (
      llmEnabled !== originalLlmSettings.enabled ||
      ollamaUrl !== originalLlmSettings.ollamaUrl ||
      ollamaModel !== originalLlmSettings.ollamaModel ||
      timeoutSeconds !== originalLlmSettings.timeoutSeconds ||
      temperature !== originalLlmSettings.temperature ||
      maxTokens !== originalLlmSettings.maxTokens ||
      promptTemplate !== originalLlmSettings.promptTemplate ||
      confidenceThreshold !== originalLlmSettings.confidenceThreshold ||
      modelMovies !== (originalLlmSettings.modelMovies || "") ||
      modelTv !== (originalLlmSettings.modelTv || "") ||
      modelMusic !== (originalLlmSettings.modelMusic || "") ||
      modelAudiobooks !== (originalLlmSettings.modelAudiobooks || "")
    );
  }, [
    originalLlmSettings,
    llmEnabled,
    ollamaUrl,
    ollamaModel,
    timeoutSeconds,
    temperature,
    maxTokens,
    promptTemplate,
    confidenceThreshold,
    modelMovies,
    modelTv,
    modelMusic,
    modelAudiobooks,
  ]);

  // ============================================================================
  // Fetch Data
  // ============================================================================

  const fetchPatterns = useCallback(async () => {
    try {
      setIsLoadingPatterns(true);
      const { data } =
        await apolloClient.query<OrganizationNamingPatternsQuery>({
          query: OrganizationNamingPatternsDocument,
          fetchPolicy: "network-only",
          variables: {
            OrderBy: [{ Name: "Asc" }],
            Page: { Limit: 200, Offset: 0 },
          },
        });

      if (data?.NamingPatterns?.Edges) {
        setPatterns(data.NamingPatterns.Edges.map((e) => e.Node));
      }
      setPatternsError(null);
    } catch (e) {
      // Silently ignore auth errors - they can happen during login race conditions
      const errorMsg = e instanceof Error ? e.message : String(e);
      if (errorMsg.toLowerCase().includes("authentication")) {
        // Clear any error for auth issues - user just needs to sign in
        setPatternsError(null);
      } else {
        setPatternsError(errorMsg || "Failed to load naming patterns");
      }
    } finally {
      setIsLoadingPatterns(false);
    }
  }, []);

  useEffect(() => {
    fetchPatterns();
  }, [fetchPatterns]);

  useEffect(() => {
    if (llmSettingsLoading) return;
    const edges = llmSettingsData?.AppSettings?.Edges ?? [];
    const settings = appSettingsToLlmSettings(edges);
    setOriginalLlmSettings(settings);
    setLlmSettingIds(llmSettingsKeyToIdMap(edges));
    setLlmEnabled(settings.enabled);
    setOllamaUrl(settings.ollamaUrl);
    setOllamaModel(settings.ollamaModel);
    setTimeoutSeconds(settings.timeoutSeconds);
    setTemperature(settings.temperature);
    setMaxTokens(settings.maxTokens);
    setPromptTemplate(settings.promptTemplate);
    setConfidenceThreshold(settings.confidenceThreshold);
    setModelMovies(settings.modelMovies || "");
    setModelTv(settings.modelTv || "");
    setModelMusic(settings.modelMusic || "");
    setModelAudiobooks(settings.modelAudiobooks || "");
    setIsLoadingLlm(false);
  }, [llmSettingsData, llmSettingsLoading]);

  // ============================================================================
  // Naming Pattern Handlers
  // ============================================================================

  const handleCreate = async () => {
    if (!formData.name.trim() || !formData.pattern.trim()) {
      addToast({
        title: "Validation Error",
        description: "Name and pattern are required",
        color: "danger",
      });
      return;
    }

    setIsSavingPattern(true);
    try {
      const { data } = await createNamingPattern({
        variables: {
          Input: {
            Name: formData.name.trim(),
            Pattern: formData.pattern.trim(),
            Description: formData.description.trim() || null,
            LibraryType: formData.libraryType,
            IsDefault: false,
            IsSystem: false,
            UserId: "",
          },
        },
      });

      if (data?.CreateNamingPattern?.Success) {
        addToast({
          title: "Pattern Created",
          description: `"${formData.name}" has been added`,
          color: "success",
        });
        onAddClose();
        setFormData({
          name: "",
          pattern: "",
          description: "",
          libraryType: "tv",
        });
        fetchPatterns();
      } else {
        throw new Error(
          data?.CreateNamingPattern?.Error || "Failed to create pattern",
        );
      }
    } catch (e) {
      addToast({
        title: "Error",
        description:
          e instanceof Error ? e.message : "Failed to create pattern",
        color: "danger",
      });
    } finally {
      setIsSavingPattern(false);
    }
  };

  const handleUpdate = async () => {
    if (!selectedPattern) return;
    if (!editFormData.name.trim() || !editFormData.pattern.trim()) {
      addToast({
        title: "Validation Error",
        description: "Name and pattern are required",
        color: "danger",
      });
      return;
    }

    setIsSavingPattern(true);
    try {
      const { data } = await updateNamingPattern({
        variables: {
          Id: selectedPattern.Id,
          Input: {
            Name: editFormData.name.trim(),
            Pattern: editFormData.pattern.trim(),
            Description: editFormData.description.trim() || null,
          },
        },
      });

      if (data?.UpdateNamingPattern?.Success) {
        addToast({
          title: "Pattern Updated",
          description: `"${editFormData.name}" has been updated`,
          color: "success",
        });
        onEditClose();
        setSelectedPattern(null);
        fetchPatterns();
      } else {
        throw new Error(
          data?.UpdateNamingPattern?.Error || "Failed to update pattern",
        );
      }
    } catch (e) {
      addToast({
        title: "Error",
        description:
          e instanceof Error ? e.message : "Failed to update pattern",
        color: "danger",
      });
    } finally {
      setIsSavingPattern(false);
    }
  };

  const handleDelete = async () => {
    if (!selectedPattern) return;

    setIsSavingPattern(true);
    try {
      const { data } = await deleteNamingPattern({
        variables: {
          Id: selectedPattern.Id,
        },
      });

      if (data?.DeleteNamingPattern?.Success) {
        addToast({
          title: "Pattern Deleted",
          description: `"${selectedPattern.Name}" has been removed`,
          color: "success",
        });
        onDeleteClose();
        setSelectedPattern(null);
        fetchPatterns();
      } else {
        throw new Error(
          data?.DeleteNamingPattern?.Error || "Failed to delete pattern",
        );
      }
    } catch (e) {
      addToast({
        title: "Error",
        description:
          e instanceof Error ? e.message : "Failed to delete pattern",
        color: "danger",
      });
    } finally {
      setIsSavingPattern(false);
    }
  };

  const handleSetDefault = async (pattern: NamingPatternRow) => {
    try {
      const oldDefault = patterns.find((p) => p.IsDefault);
      if (oldDefault && oldDefault.Id !== pattern.Id) {
        const { data: clearData } = await updateNamingPattern({
          variables: {
            Id: oldDefault.Id,
            Input: {
              IsDefault: false,
            },
          },
        });
        if (!clearData?.UpdateNamingPattern?.Success) {
          throw new Error(
            clearData?.UpdateNamingPattern?.Error ||
              "Failed to clear previous default",
          );
        }
      }

      const { data } = await updateNamingPattern({
        variables: {
          Id: pattern.Id,
          Input: {
            IsDefault: true,
          },
        },
      });

      if (data?.UpdateNamingPattern?.Success) {
        addToast({
          title: "Default Updated",
          description: `"${pattern.Name}" is now the default pattern`,
          color: "success",
        });
        fetchPatterns();
      } else {
        throw new Error(
          data?.UpdateNamingPattern?.Error || "Failed to set default",
        );
      }
    } catch (e) {
      addToast({
        title: "Error",
        description: e instanceof Error ? e.message : "Failed to set default",
        color: "danger",
      });
    }
  };

  // ============================================================================
  // LLM Parser Handlers
  // ============================================================================

  const handleSaveLlm = async () => {
    setIsSavingLlm(true);
    try {
      const values: Array<[string, string]> = [
        [LLM_KEYS.enabled, llmEnabled ? "true" : "false"],
        [LLM_KEYS.ollamaUrl, ollamaUrl],
        [LLM_KEYS.ollamaModel, ollamaModel],
        [LLM_KEYS.timeoutSeconds, String(timeoutSeconds)],
        [LLM_KEYS.temperature, String(temperature)],
        [LLM_KEYS.maxTokens, String(maxTokens)],
        [LLM_KEYS.promptTemplate, promptTemplate],
        [LLM_KEYS.confidenceThreshold, String(confidenceThreshold)],
        [LLM_KEYS.modelMovies, modelMovies],
        [LLM_KEYS.modelTv, modelTv],
        [LLM_KEYS.modelMusic, modelMusic],
        [LLM_KEYS.modelAudiobooks, modelAudiobooks],
      ];

      for (const [key, value] of values) {
        const existingId = llmSettingIds.get(key);
        if (existingId) {
          const { data } = await updateAppSetting({
            variables: {
              Id: existingId,
              Input: { Value: value },
            },
          });
          if (!data?.UpdateAppSetting?.Success) {
            throw new Error(
              data?.UpdateAppSetting?.Error || "Failed to save settings",
            );
          }
        } else {
          const { data } = await createAppSetting({
            variables: {
              Input: {
                Key: key,
                Value: value,
                Category: LLM_CATEGORY,
              },
            },
          });
          if (!data?.CreateAppSetting?.Success) {
            throw new Error(
              data?.CreateAppSetting?.Error || "Failed to save settings",
            );
          }
        }
      }

      const saved: LlmParserSettings = {
        enabled: llmEnabled,
        ollamaUrl,
        ollamaModel,
        timeoutSeconds,
        temperature,
        maxTokens,
        promptTemplate,
        confidenceThreshold,
        modelMovies: modelMovies || null,
        modelTv: modelTv || null,
        modelMusic: modelMusic || null,
        modelAudiobooks: modelAudiobooks || null,
        promptMovies: null,
        promptTv: null,
        promptMusic: null,
        promptAudiobooks: null,
      };
      setOriginalLlmSettings(saved);
      addToast({
        title: "Settings saved",
        description: "LLM parser settings have been updated",
        color: "success",
      });
    } catch (e) {
      addToast({
        title: "Error",
        description: sanitizeError(e),
        color: "danger",
      });
    } finally {
      setIsSavingLlm(false);
    }
  };

  const handleResetLlm = () => {
    if (originalLlmSettings) {
      setLlmEnabled(originalLlmSettings.enabled);
      setOllamaUrl(originalLlmSettings.ollamaUrl);
      setOllamaModel(originalLlmSettings.ollamaModel);
      setTimeoutSeconds(originalLlmSettings.timeoutSeconds);
      setTemperature(originalLlmSettings.temperature);
      setMaxTokens(originalLlmSettings.maxTokens);
      setPromptTemplate(originalLlmSettings.promptTemplate);
      setConfidenceThreshold(originalLlmSettings.confidenceThreshold);
      setModelMovies(originalLlmSettings.modelMovies || "");
      setModelTv(originalLlmSettings.modelTv || "");
      setModelMusic(originalLlmSettings.modelMusic || "");
      setModelAudiobooks(originalLlmSettings.modelAudiobooks || "");
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setConnectionStatus("untested");
    setConnectionError(null);
    try {
      const baseUrl = ollamaUrl.replace(/\/$/, "");
      const response = await fetch(`${baseUrl}/api/tags`);
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const json = (await response.json()) as {
        models?: Array<{ name?: string }>;
      };
      const models = (json.models ?? [])
        .map((m) => m.name)
        .filter((m): m is string => Boolean(m));
      if (models.length > 0) {
        setConnectionStatus("success");
        setAvailableModels(models);
        addToast({
          title: "Connection successful",
          description: `Found ${models.length} models`,
          color: "success",
        });
      } else {
        setConnectionStatus("error");
        setConnectionError("Connected, but no models were returned.");
        addToast({
          title: "Connection failed",
          description: "Connected, but no models were returned.",
          color: "danger",
        });
      }
    } catch (e) {
      setConnectionStatus("error");
      setConnectionError(sanitizeError(e));
      addToast({
        title: "Error",
        description: sanitizeError(e),
        color: "danger",
      });
    } finally {
      setIsTesting(false);
    }
  };

  const handleTestParser = async () => {
    if (!testFilename.trim()) return;
    setIsParsing(true);
    setParseResult(null);
    try {
      const startedAt = performance.now();
      const regexResult = buildSimpleFilenameParseResult(testFilename);
      const regexTimeMs = Math.round(performance.now() - startedAt);
      setParseResult({
        regexResult,
        regexTimeMs,
        llmResult: null,
        llmTimeMs: null,
        llmError:
          "Typed parser-test resolver is unavailable; showing local regex preview.",
      });
    } catch (e) {
      addToast({
        title: "Error",
        description: sanitizeError(e),
        color: "danger",
      });
    } finally {
      setIsParsing(false);
    }
  };

  // ============================================================================
  // Table Configuration
  // ============================================================================

  // Table columns - Type before Pattern
  const columns: DataTableColumn<NamingPatternRow>[] = [
    {
      key: "name",
      label: "Name",
      sortable: true,
      render: (pattern) => (
        <div className="flex items-center gap-2">
          <IconTemplate size={16} className="text-amber-400" />
          <span className="font-medium">{pattern.Name}</span>
          {pattern.IsDefault && (
            <Chip size="sm" color="primary" variant="flat">
              Default
            </Chip>
          )}
          {pattern.IsSystem && (
            <Chip size="sm" variant="flat" className="text-default-500">
              System
            </Chip>
          )}
        </div>
      ),
      width: 300,
    },
    {
      key: "libraryType",
      label: "Type",
      sortable: true,
      width: 100,
      render: (pattern) => (
        <span className="text-sm text-default-500 capitalize">
          {pattern.LibraryType || "tv"}
        </span>
      ),
    },
    {
      key: "pattern",
      label: "Pattern",
      render: (pattern) => (
        <Tooltip
          content={
            <div className="text-xs">
              <span className="text-default-500">Example: </span>
              <code className="font-mono">
                {previewNamingPattern(
                  pattern.Pattern,
                  pattern.LibraryType || undefined,
                )}
              </code>
            </div>
          }
          delay={300}
        >
          <code className="text-xs  px-2 py-1 rounded font-mono text-default-600 break-all cursor-help">
            {pattern.Pattern}
          </code>
        </Tooltip>
      ),
    },
  ];

  // Row actions
  const rowActions: RowAction<NamingPatternRow>[] = [
    {
      key: "edit",
      label: "Edit",
      icon: <IconPencil size={16} />,
      color: "primary",
      inDropdown: true,
      isDisabled: (pattern: NamingPatternRow) => pattern.IsSystem,
      onAction: (pattern: NamingPatternRow) => {
        setSelectedPattern(pattern);
        setEditFormData({
          name: pattern.Name,
          pattern: pattern.Pattern,
          description: pattern.Description || "",
          libraryType: pattern.LibraryType || "tv",
        });
        onEditOpen();
      },
    },
    {
      key: "set-default",
      label: "Set as Default",
      icon: <IconStarFilled size={16} />,
      color: "warning",
      inDropdown: true,
      isVisible: (pattern: NamingPatternRow) => !pattern.IsDefault,
      onAction: handleSetDefault,
    },
    {
      key: "delete",
      label: "Delete",
      icon: <IconTrash size={16} />,
      color: "danger",
      inDropdown: true,
      isDisabled: (pattern: NamingPatternRow) => pattern.IsSystem,
      onAction: (pattern: NamingPatternRow) => {
        setSelectedPattern(pattern);
        onDeleteOpen();
      },
    },
  ];

  // Card renderer for mobile view
  const cardRenderer = ({
    item,
    actions,
  }: CardRendererProps<NamingPatternRow>) => (
    <Card className="w-full">
      <CardBody className="p-4">
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-2">
              <IconTemplate size={18} className="text-amber-400" />
              <span className="font-medium">{item.Name}</span>
              {item.IsDefault && (
                <Chip size="sm" color="primary" variant="flat">
                  Default
                </Chip>
              )}
            </div>
            <code className="text-xs bg-default-100 px-2 py-1 rounded font-mono text-default-600 block mb-2 break-all">
              {item.Pattern}
            </code>
            {item.Description && (
              <p className="text-sm text-default-500">{item.Description}</p>
            )}
          </div>
          <div className="flex gap-1">
            {actions
              .filter((action) => !action.isVisible || action.isVisible(item))
              .map((action) => (
                <Button
                  key={action.key}
                  size="sm"
                  isIconOnly
                  variant="light"
                  color={action.color}
                  onPress={() => action.onAction(item)}
                  isDisabled={action.isDisabled?.(item)}
                >
                  {typeof action.icon === "function"
                    ? action.icon(item)
                    : action.icon}
                </Button>
              ))}
          </div>
        </div>
      </CardBody>
    </Card>
  );

  // Get pattern variables based on library type
  const getPatternVariables = (libraryType: string) => {
    const vars = [...PATTERN_VARIABLES.common];
    switch (libraryType) {
      case "tv":
        vars.push(...PATTERN_VARIABLES.tv);
        break;
      case "movies":
        vars.push(...PATTERN_VARIABLES.movies);
        break;
      case "music":
        vars.push(...PATTERN_VARIABLES.music);
        break;
      case "audiobooks":
        vars.push(...PATTERN_VARIABLES.audiobooks);
        break;
    }
    return vars;
  };

  // Loading state
  if (isLoadingPatterns && isLoadingLlm) {
    return (
      <div className="flex items-center justify-center h-64">
        <Spinner size="lg" />
      </div>
    );
  }

  return (
    <div
      className="grow overflow-y-auto overflow-x-hidden pb-8"
      style={{ scrollbarGutter: "stable" }}
    >
      <SettingsHeader
        title="File Organization"
        subtitle="Configure file naming patterns and filename parsing"
        hasChanges={hasLlmChanges}
        isSaving={isSavingLlm}
        onSave={handleSaveLlm}
        onReset={handleResetLlm}
        isSaveDisabled={!hasLlmChanges}
        isResetDisabled={!hasLlmChanges}
      />

      <Accordion selectionMode="multiple" variant="splitted">
        {/* File Naming and Patterns Section */}
        <AccordionItem
          key="naming"
          aria-label="File Naming and Patterns"
          title={
            <div className="flex items-center gap-2">
              <IconTemplate size={18} className="text-amber-400" />
              <span className="font-semibold">File Naming and Patterns</span>
            </div>
          }
          subtitle="Configure how media files are renamed and organized"
        >
          <div className="pb-2">
            {patternsError ? (
              <div className="text-center py-8">
                <p className="text-danger mb-4">{patternsError}</p>
                <Button color="primary" onPress={fetchPatterns}>
                  Retry
                </Button>
              </div>
            ) : (
              <DataTable
                skeletonDelay={500}
                data={patterns}
                columns={columns}
                getRowKey={(pattern) => pattern.Id}
                rowActions={rowActions}
                cardRenderer={cardRenderer}
                removeWrapper
                isStriped
                toolbarContent={
                  <Tooltip content="Add Pattern">
                    <Button
                      color="primary"
                      size="sm"
                      isIconOnly
                      onPress={() => {
                        setFormData({
                          name: "",
                          pattern: "",
                          description: "",
                          libraryType: "tv",
                        });
                        onAddOpen();
                      }}
                    >
                      <IconPlus size={16} />
                    </Button>
                  </Tooltip>
                }
                toolbarContentPosition="end"
                emptyContent={
                  <div className="text-center py-8">
                    <IconTemplate
                      size={48}
                      className="mx-auto text-default-300 mb-4"
                    />
                    <p className="text-default-500">No naming patterns found</p>
                    <Tooltip content="Add Pattern">
                      <Button
                        color="primary"
                        size="sm"
                        isIconOnly
                        className="mt-4"
                        onPress={onAddOpen}
                      >
                        <IconPlus size={16} />
                      </Button>
                    </Tooltip>
                  </div>
                }
              />
            )}
          </div>
        </AccordionItem>

        {/* AI LLM Parser Section */}
        <AccordionItem
          key="llm"
          aria-label="AI LLM Parser"
          title={
            <div className="flex items-center gap-2">
              <IconBrain size={18} className="text-cyan-400" />
              <span className="font-semibold">AI LLM Parser (Ollama)</span>
              {llmEnabled && connectionStatus === "success" && (
                <Chip size="sm" color="success" variant="flat">
                  Connected
                </Chip>
              )}
            </div>
          }
          subtitle="Use a local LLM as fallback for complex filenames"
        >
          <div className="space-y-4 pb-2">
            {/* Enable Toggle */}
            <div className="flex items-center justify-between p-3 bg-content2 rounded-lg">
              <div>
                <p className="font-medium">Enable LLM Parser</p>
                <p className="text-sm text-default-500">
                  Use Ollama for parsing when regex confidence is low
                </p>
              </div>
              <Switch isSelected={llmEnabled} onValueChange={setLlmEnabled} />
            </div>

            {llmEnabled && (
              <>
                {/* Connection */}
                <div className="space-y-3">
                  <p className="text-sm font-medium text-default-500 flex items-center gap-2">
                    <IconServer size={16} className="text-blue-400" />
                    Connection
                  </p>
                  <div className="flex gap-2">
                    <Input
                      label="Ollama URL"
                      placeholder="http://localhost:11434"
                      value={ollamaUrl}
                      onChange={(e) => setOllamaUrl(e.target.value)}
                      startContent={
                        <IconServer size={16} className="text-default-400" />
                      }
                      className="flex-1"
                    />
                    <Button
                      color={
                        connectionStatus === "success"
                          ? "success"
                          : connectionStatus === "error"
                            ? "danger"
                            : "default"
                      }
                      variant={
                        connectionStatus === "untested" ? "flat" : "solid"
                      }
                      isLoading={isTesting}
                      onPress={handleTestConnection}
                      className="self-end"
                    >
                      {connectionStatus === "success" ? (
                        <IconCheck size={16} />
                      ) : connectionStatus === "error" ? (
                        <IconX size={16} />
                      ) : (
                        "Test"
                      )}
                    </Button>
                  </div>

                  {connectionError && (
                    <Card className="bg-danger/10 border border-danger/20">
                      <CardBody className="py-2 px-3">
                        <p className="text-danger text-sm">{connectionError}</p>
                      </CardBody>
                    </Card>
                  )}

                  {availableModels.length > 0 ? (
                    <Select
                      label="Default Model"
                      selectedKeys={[ollamaModel]}
                      onSelectionChange={(keys) => {
                        const selected = Array.from(keys)[0] as string;
                        if (selected) setOllamaModel(selected);
                      }}
                      description={`${availableModels.length} models available`}
                    >
                      {availableModels.map((model) => (
                        <SelectItem key={model}>{model}</SelectItem>
                      ))}
                    </Select>
                  ) : (
                    <Input
                      label="Default Model"
                      placeholder="qwen2.5-coder:7b"
                      value={ollamaModel}
                      onChange={(e) => setOllamaModel(e.target.value)}
                      description="Test connection to see available models"
                    />
                  )}
                </div>

                <Divider />

                {/* Library-Type Models */}
                <div className="space-y-3">
                  <p className="text-sm font-medium text-default-500 flex items-center gap-2">
                    <IconMovie size={16} className="text-purple-400" />
                    Library-Type Models
                  </p>
                  <p className="text-small text-default-500">
                    Override the default model for specific library types. Leave
                    empty to use the default.
                  </p>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    {availableModels.length > 0 ? (
                      <>
                        <Select
                          label="Movies"
                          selectedKeys={
                            modelMovies ? [modelMovies] : ["__default__"]
                          }
                          onSelectionChange={(keys) => {
                            const selected = Array.from(keys)[0] as string;
                            setModelMovies(
                              selected === "__default__" ? "" : selected || "",
                            );
                          }}
                          startContent={
                            <IconMovie size={16} className="text-purple-400" />
                          }
                          items={[
                            {
                              key: "__default__",
                              label: "Use default model",
                              isDefault: true,
                            },
                            ...availableModels.map((m) => ({
                              key: m,
                              label: m,
                              isDefault: false,
                            })),
                          ]}
                        >
                          {(item) => (
                            <SelectItem
                              key={item.key}
                              className={
                                item.isDefault ? "text-default-500" : ""
                              }
                            >
                              {item.label}
                            </SelectItem>
                          )}
                        </Select>
                        <Select
                          label="TV Shows"
                          selectedKeys={modelTv ? [modelTv] : ["__default__"]}
                          onSelectionChange={(keys) => {
                            const selected = Array.from(keys)[0] as string;
                            setModelTv(
                              selected === "__default__" ? "" : selected || "",
                            );
                          }}
                          startContent={
                            <IconDeviceTv size={16} className="text-blue-400" />
                          }
                          items={[
                            {
                              key: "__default__",
                              label: "Use default model",
                              isDefault: true,
                            },
                            ...availableModels.map((m) => ({
                              key: m,
                              label: m,
                              isDefault: false,
                            })),
                          ]}
                        >
                          {(item) => (
                            <SelectItem
                              key={item.key}
                              className={
                                item.isDefault ? "text-default-500" : ""
                              }
                            >
                              {item.label}
                            </SelectItem>
                          )}
                        </Select>
                        <Select
                          label="Audiobooks"
                          selectedKeys={
                            modelAudiobooks
                              ? [modelAudiobooks]
                              : ["__default__"]
                          }
                          onSelectionChange={(keys) => {
                            const selected = Array.from(keys)[0] as string;
                            setModelAudiobooks(
                              selected === "__default__" ? "" : selected || "",
                            );
                          }}
                          startContent={
                            <IconBook size={16} className="text-amber-400" />
                          }
                          items={[
                            {
                              key: "__default__",
                              label: "Use default model",
                              isDefault: true,
                            },
                            ...availableModels.map((m) => ({
                              key: m,
                              label: m,
                              isDefault: false,
                            })),
                          ]}
                        >
                          {(item) => (
                            <SelectItem
                              key={item.key}
                              className={
                                item.isDefault ? "text-default-500" : ""
                              }
                            >
                              {item.label}
                            </SelectItem>
                          )}
                        </Select>
                        <Select
                          label="Music"
                          selectedKeys={
                            modelMusic ? [modelMusic] : ["__default__"]
                          }
                          onSelectionChange={(keys) => {
                            const selected = Array.from(keys)[0] as string;
                            setModelMusic(
                              selected === "__default__" ? "" : selected || "",
                            );
                          }}
                          startContent={
                            <IconMusic size={16} className="text-success" />
                          }
                          items={[
                            {
                              key: "__default__",
                              label: "Use default model",
                              isDefault: true,
                            },
                            ...availableModels.map((m) => ({
                              key: m,
                              label: m,
                              isDefault: false,
                            })),
                          ]}
                        >
                          {(item) => (
                            <SelectItem
                              key={item.key}
                              className={
                                item.isDefault ? "text-default-500" : ""
                              }
                            >
                              {item.label}
                            </SelectItem>
                          )}
                        </Select>
                      </>
                    ) : (
                      <>
                        <Input
                          label="Movies"
                          placeholder="Use default"
                          value={modelMovies}
                          onChange={(e) => setModelMovies(e.target.value)}
                          startContent={
                            <IconMovie size={16} className="text-purple-400" />
                          }
                        />
                        <Input
                          label="TV Shows"
                          placeholder="Use default"
                          value={modelTv}
                          onChange={(e) => setModelTv(e.target.value)}
                          startContent={
                            <IconDeviceTv size={16} className="text-blue-400" />
                          }
                        />
                        <Input
                          label="Audiobooks"
                          placeholder="Use default"
                          value={modelAudiobooks}
                          onChange={(e) => setModelAudiobooks(e.target.value)}
                          startContent={
                            <IconBook size={16} className="text-amber-400" />
                          }
                        />
                        <Input
                          label="Music"
                          placeholder="Use default"
                          value={modelMusic}
                          onChange={(e) => setModelMusic(e.target.value)}
                          startContent={
                            <IconMusic size={16} className="text-success" />
                          }
                        />
                      </>
                    )}
                  </div>
                </div>

                <Divider />

                {/* Advanced Settings */}
                <div className="space-y-3">
                  <p className="text-sm font-medium text-default-500 flex items-center gap-2">
                    <IconBrain size={16} className="text-default-500" />
                    Advanced Settings
                  </p>
                  <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <Input
                      type="number"
                      label="Timeout (seconds)"
                      value={timeoutSeconds.toString()}
                      onChange={(e) =>
                        setTimeoutSeconds(parseInt(e.target.value) || 30)
                      }
                      description="Max wait time for LLM response"
                    />
                    <Input
                      type="number"
                      label="Temperature"
                      value={temperature.toString()}
                      onChange={(e) =>
                        setTemperature(parseFloat(e.target.value) || 0.1)
                      }
                      step={0.1}
                      min={0}
                      max={2}
                      description="Lower = more deterministic"
                    />
                    <Input
                      type="number"
                      label="Max Tokens"
                      value={maxTokens.toString()}
                      onChange={(e) =>
                        setMaxTokens(parseInt(e.target.value) || 256)
                      }
                      description="Maximum output length"
                    />
                  </div>

                  <Input
                    type="number"
                    label="Confidence Threshold"
                    value={confidenceThreshold.toString()}
                    onChange={(e) =>
                      setConfidenceThreshold(parseFloat(e.target.value) || 0.7)
                    }
                    step={0.1}
                    min={0}
                    max={1}
                    description="Use LLM when regex confidence is below this value (0.0 - 1.0)"
                    className="max-w-xs"
                  />

                  <Textarea
                    label="Default Prompt Template"
                    placeholder="Parse this media filename..."
                    value={promptTemplate}
                    onChange={(e) => setPromptTemplate(e.target.value)}
                    minRows={4}
                    description="Use {filename} as placeholder. Models with baked-in system prompts may ignore this."
                  />
                </div>
              </>
            )}
          </div>
        </AccordionItem>

        {/* Test Parser Section */}
        <AccordionItem
          key="test"
          aria-label="Test Parser"
          title={
            <div className="flex items-center gap-2">
              <IconTestPipe size={18} className="text-default-400" />
              <span className="font-semibold">Test Parser</span>
            </div>
          }
          subtitle="Compare regex and LLM parsing results side by side"
        >
          <div className="space-y-4 pb-2">
            <div className="flex gap-2">
              <Input
                label="Test Filename"
                placeholder="Movie.Name.2024.1080p.BluRay.x264-GROUP"
                value={testFilename}
                onChange={(e) => setTestFilename(e.target.value)}
                className="flex-1"
              />
              <Button
                color="primary"
                isLoading={isParsing}
                onPress={handleTestParser}
                className="self-end"
                startContent={!isParsing && <IconRefresh size={16} />}
              >
                Parse
              </Button>
            </div>

            {parseResult && (
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                <ParseResultCard
                  title="Regex Parser"
                  result={parseResult.regexResult}
                  timeMs={parseResult.regexTimeMs}
                  variant="regex"
                />
                <ParseResultCard
                  title="LLM Parser"
                  result={parseResult.llmResult}
                  timeMs={parseResult.llmTimeMs}
                  error={parseResult.llmError}
                  variant="llm"
                />
              </div>
            )}
          </div>
        </AccordionItem>
      </Accordion>

      {/* Add Pattern Modal */}
      <Modal isOpen={isAddOpen} onClose={onAddClose} size="xl">
        <ModalContent>
          <ModalHeader>Add Naming Pattern</ModalHeader>
          <ModalBody>
            <div className="space-y-4">
              <Input
                label="Name"
                labelPlacement="inside"
                variant="flat"
                placeholder="e.g., My Custom Pattern"
                value={formData.name}
                onValueChange={(value) =>
                  setFormData({ ...formData, name: value })
                }
                isRequired
                classNames={{
                  label: "text-sm font-medium text-primary!",
                }}
              />
              <Select
                label="Library Type"
                selectedKeys={[formData.libraryType]}
                onChange={(e) =>
                  setFormData({ ...formData, libraryType: e.target.value })
                }
                size="sm"
              >
                <SelectItem key="tv">TV Shows</SelectItem>
                <SelectItem key="movies">Movies</SelectItem>
                <SelectItem key="music">Music</SelectItem>
                <SelectItem key="audiobooks">Audiobooks</SelectItem>
                <SelectItem key="other">Other</SelectItem>
              </Select>
              <Textarea
                label="Pattern"
                placeholder={
                  formData.libraryType === "music"
                    ? "{artist}/{album} ({year})/{track:02} - {title}.{ext}"
                    : formData.libraryType === "movies"
                      ? "{title} ({year})/{title} ({year}).{ext}"
                      : formData.libraryType === "audiobooks"
                        ? "{author}/{title}/{chapter:02} - {chapter_title}.{ext}"
                        : "{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}"
                }
                value={formData.pattern}
                onValueChange={(value) =>
                  setFormData({ ...formData, pattern: value })
                }
                isRequired
                classNames={{
                  input: "font-mono text-sm",
                }}
              />

              {/* Pattern Variables */}
              <div className="bg-content2 rounded-lg p-3">
                <p className="text-xs text-default-500 mb-2 font-medium">
                  Available Variables
                </p>
                <div className="flex flex-wrap gap-2">
                  {getPatternVariables(formData.libraryType).map((v) => (
                    <Tooltip key={v.var} content={v.desc}>
                      <Button
                        size="sm"
                        variant="flat"
                        className="font-mono text-xs h-7"
                        onPress={() =>
                          setFormData({
                            ...formData,
                            pattern: formData.pattern + v.var,
                          })
                        }
                      >
                        {v.var}
                      </Button>
                    </Tooltip>
                  ))}
                </div>
              </div>

              <Input
                label="Description / Example"
                labelPlacement="inside"
                variant="flat"
                placeholder="e.g., Show/Season 01/Show - S01E01 - Title.mkv"
                value={formData.description}
                onValueChange={(value) =>
                  setFormData({ ...formData, description: value })
                }
                classNames={{
                  label: "text-sm font-medium text-primary!",
                }}
              />
              {formData.pattern && (
                <div className="bg-default-100 p-3 rounded">
                  <p className="text-xs text-default-500 mb-1">Preview:</p>
                  <code className="text-sm font-mono break-all">
                    {previewNamingPattern(
                      formData.pattern,
                      formData.libraryType,
                    )}
                  </code>
                </div>
              )}
            </div>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onAddClose}>
              Cancel
            </Button>
            <Button
              color="primary"
              onPress={handleCreate}
              isLoading={isSavingPattern}
              isDisabled={!formData.name.trim() || !formData.pattern.trim()}
            >
              Create Pattern
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>

      {/* Edit Pattern Modal */}
      <Modal isOpen={isEditOpen} onClose={onEditClose} size="xl">
        <ModalContent>
          <ModalHeader>Edit Naming Pattern</ModalHeader>
          <ModalBody>
            <div className="space-y-4">
              <Input
                label="Name"
                labelPlacement="inside"
                variant="flat"
                placeholder="e.g., My Custom Pattern"
                value={editFormData.name}
                onValueChange={(value) =>
                  setEditFormData({ ...editFormData, name: value })
                }
                isRequired
                classNames={{
                  label: "text-sm font-medium text-primary!",
                }}
              />
              <Textarea
                label="Pattern"
                placeholder="{show}/Season {season:02}/{show} - S{season:02}E{episode:02} - {title}.{ext}"
                value={editFormData.pattern}
                onValueChange={(value) =>
                  setEditFormData({ ...editFormData, pattern: value })
                }
                isRequired
                classNames={{
                  input: "font-mono text-sm",
                }}
              />

              {/* Pattern Variables */}
              <div className="bg-content2 rounded-lg p-3">
                <p className="text-xs text-default-500 mb-2 font-medium">
                  Available Variables
                </p>
                <div className="flex flex-wrap gap-2">
                  {getPatternVariables(
                    editFormData.libraryType ||
                      selectedPattern?.LibraryType ||
                      "tv",
                  ).map((v) => (
                    <Tooltip key={v.var} content={v.desc}>
                      <Button
                        size="sm"
                        variant="flat"
                        className="font-mono text-xs h-7"
                        onPress={() =>
                          setEditFormData({
                            ...editFormData,
                            pattern: editFormData.pattern + v.var,
                          })
                        }
                      >
                        {v.var}
                      </Button>
                    </Tooltip>
                  ))}
                </div>
              </div>

              <Input
                label="Description / Example"
                labelPlacement="inside"
                variant="flat"
                placeholder="e.g., Show/Season 01/Show - S01E01 - Title.mkv"
                value={editFormData.description}
                onValueChange={(value) =>
                  setEditFormData({ ...editFormData, description: value })
                }
                classNames={{
                  label: "text-sm font-medium text-primary!",
                }}
              />
              {editFormData.pattern && (
                <div className="bg-default-100 p-3 rounded">
                  <p className="text-xs text-default-500 mb-1">Preview:</p>
                  <code className="text-sm font-mono break-all">
                    {previewNamingPattern(
                      editFormData.pattern,
                      selectedPattern?.LibraryType || undefined,
                    )}
                  </code>
                </div>
              )}
            </div>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onEditClose}>
              Cancel
            </Button>
            <Button
              color="primary"
              onPress={handleUpdate}
              isLoading={isSavingPattern}
              isDisabled={
                !editFormData.name.trim() || !editFormData.pattern.trim()
              }
            >
              Save Changes
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>

      {/* Delete Confirmation Modal */}
      <Modal isOpen={isDeleteOpen} onClose={onDeleteClose}>
        <ModalContent>
          <ModalHeader>Delete Pattern</ModalHeader>
          <ModalBody>
            <p>
              Are you sure you want to delete{" "}
              <strong>"{selectedPattern?.Name}"</strong>?
            </p>
            <p className="text-sm text-default-500 mt-2">
              This action cannot be undone. Libraries using this pattern will
              fall back to the default.
            </p>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onDeleteClose}>
              Cancel
            </Button>
            <Button
              color="danger"
              onPress={handleDelete}
              isLoading={isSavingPattern}
            >
              Delete
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  );
}

// ============================================================================
// Parser Result Components
// ============================================================================

interface ParseResultCardProps {
  title: string;
  result: FilenameParseResult | null;
  timeMs: number | null;
  error?: string | null;
  variant?: "regex" | "llm";
}

function ParseResultCard({
  title,
  result,
  timeMs,
  error,
  variant = "regex",
}: ParseResultCardProps) {
  const headerIcon =
    variant === "llm" ? (
      <IconBrain size={18} className="text-cyan-400" />
    ) : (
      <IconRefresh size={18} className="text-blue-400" />
    );

  if (error) {
    return (
      <Card>
        <CardHeader className="flex justify-between items-center gap-2">
          <div className="flex items-center gap-2">
            {headerIcon}
            <span className="font-semibold">{title}</span>
          </div>
          <Chip color="danger" size="sm" variant="flat">
            Error
          </Chip>
        </CardHeader>
        <Divider />
        <CardBody>
          <p className="text-danger text-sm">{error}</p>
        </CardBody>
      </Card>
    );
  }

  if (!result) {
    return (
      <Card>
        <CardHeader className="flex justify-between items-center gap-2">
          <div className="flex items-center gap-2">
            {headerIcon}
            <span className="font-semibold">{title}</span>
          </div>
          <Chip color="default" size="sm" variant="flat">
            Disabled
          </Chip>
        </CardHeader>
        <Divider />
        <CardBody>
          <p className="text-default-500 text-sm">
            Enable LLM parsing to compare results
          </p>
        </CardBody>
      </Card>
    );
  }

  const confidenceColor =
    result.confidence >= 0.8
      ? "success"
      : result.confidence >= 0.5
        ? "warning"
        : "danger";

  return (
    <Card>
      <CardHeader className="flex justify-between items-center gap-2">
        <div className="flex items-center gap-2">
          {headerIcon}
          <span className="font-semibold">{title}</span>
        </div>
        <div className="flex gap-2 items-center">
          {timeMs !== null && (
            <Chip
              size="sm"
              variant="flat"
              startContent={<IconClock size={12} />}
            >
              {timeMs.toFixed(1)}ms
            </Chip>
          )}
          <Chip color={confidenceColor} size="sm" variant="flat">
            {(result.confidence * 100).toFixed(0)}%
          </Chip>
        </div>
      </CardHeader>
      <Divider />
      <CardBody>
        <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
          <ParseField label="Type" value={result.mediaType} highlight />
          <ParseField label="Title" value={result.title} highlight />
          <ParseField label="Year" value={result.year?.toString()} />
          <ParseField label="Season" value={result.season?.toString()} />
          <ParseField label="Episode" value={result.episode?.toString()} />
          <ParseField label="Resolution" value={result.resolution} />
          <ParseField label="Source" value={result.source} />
          <ParseField label="Codec" value={result.videoCodec} />
          <ParseField label="Audio" value={result.audio} />
          <ParseField label="HDR" value={result.hdr} />
          <ParseField label="Group" value={result.releaseGroup} />
          <ParseField label="Edition" value={result.edition} />
        </div>
      </CardBody>
    </Card>
  );
}

interface ParseFieldProps {
  label: string;
  value: string | null | undefined;
  highlight?: boolean;
}

function ParseField({ label, value, highlight }: ParseFieldProps) {
  return (
    <div className="flex justify-between">
      <span className="text-default-500">{label}</span>
      <span
        className={
          value
            ? highlight
              ? "text-foreground font-medium"
              : "text-foreground"
            : "text-default-400"
        }
      >
        {value || "—"}
      </span>
    </div>
  );
}
