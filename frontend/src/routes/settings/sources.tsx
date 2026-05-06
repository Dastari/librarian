import { createFileRoute } from "@tanstack/react-router";
import { useState, useCallback, useMemo, useEffect } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import { Divider } from "@heroui/divider";
import { Input } from "@heroui/input";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
} from "@heroui/modal";
import { Select, SelectItem } from "@heroui/select";
import { Switch } from "@heroui/switch";
import { Spinner } from "@heroui/spinner";
import { Tooltip } from "@heroui/tooltip";
import {
  IconPlus,
  IconTrash,
  IconPencil,
  IconPlugConnected,
  IconArrowUp,
  IconArrowDown,
  IconSearch,
  IconWorldSearch,
  IconDownload,
  IconAlertTriangle,
} from "@tabler/icons-react";
import { DataTable } from "../../components/data-table/DataTable";
import type {
  DataTableColumn,
  RowAction,
} from "../../components/data-table/types";

import {
  SourcesDocument,
  AvailableSourceDefinitionsDocument,
  SourceSettingDefinitionsDocument,
  SearchSourcesDocument,
  AddTorrentDocument,
  CreateSourceDocument,
  UpdateSourceDocument,
  DeleteSourceDocument,
  TestSourceDocument,
  UpdateSourcePrioritiesDocument,
  type SourcesQuery,
  type AvailableSourceDefinitionsQuery,
  type SourceSettingDefinitionsQuery,
  type SearchSourcesQuery,
} from "../../lib/graphql/generated/graphql";
import { apolloClient, useMutation, useQuery } from "../../lib/graphql/client";
import { addToast } from "@heroui/toast";
import { sanitizeError } from "../../lib/format";

type SourceNode = SourcesQuery["Sources"]["Edges"][number]["Node"];
type SourceDefinitionInfo =
  AvailableSourceDefinitionsQuery["AvailableSourceDefinitions"][number];
type SourceSettingDefinition =
  SourceSettingDefinitionsQuery["SourceSettingDefinitions"][number];
type SourceReleaseInfo =
  SearchSourcesQuery["SearchSources"]["Sources"][number]["Releases"][number];

export const Route = createFileRoute("/settings/sources")({
  component: SourcesSettingsPage,
});

// =============================================================================
// Helper components
// =============================================================================

function SourceTypeChip({ type }: { type: string }) {
  const colorMap: Record<string, "primary" | "secondary" | "warning"> = {
    TorrentIndexer: "primary",
    UsenetIndexer: "secondary",
    RssFeed: "warning",
  };
  const labelMap: Record<string, string> = {
    TorrentIndexer: "Torrent Indexer",
    UsenetIndexer: "Usenet Indexer",
    RssFeed: "RSS Feed",
  };
  return (
    <Chip size="sm" variant="flat" color={colorMap[type] ?? "default"}>
      {labelMap[type] ?? type}
    </Chip>
  );
}

function StatusChip({ source }: { source: SourceNode }) {
  if (!source.Enabled) {
    return (
      <Chip size="sm" variant="flat" color="default">
        Disabled
      </Chip>
    );
  }
  if (source.ErrorCount > 0) {
    return (
      <Tooltip content={source.LastError ?? "Unknown error"}>
        <Chip size="sm" variant="flat" color="danger">
          Error
        </Chip>
      </Tooltip>
    );
  }
  if (source.LastSuccessAt) {
    return (
      <Chip size="sm" variant="flat" color="success">
        Healthy
      </Chip>
    );
  }
  return (
    <Chip size="sm" variant="flat" color="default">
      Untested
    </Chip>
  );
}

// =============================================================================
// Main Page
// =============================================================================

function SourcesSettingsPage() {
  const { data, loading, previousData, refetch } = useQuery<SourcesQuery>(
    SourcesDocument,
    {
      variables: { OrderBy: [{ Priority: "ASC" }] },
      fetchPolicy: "cache-and-network",
    },
  );
  const [testSource] = useMutation(TestSourceDocument);
  const [deleteSource] = useMutation(DeleteSourceDocument);
  const [updateSource] = useMutation(UpdateSourceDocument);
  const [updateSourcePriorities] = useMutation(UpdateSourcePrioritiesDocument);

  const sources = useMemo(() => {
    const d = data ?? previousData;
    return d?.Sources?.Edges?.map((e) => e.Node) ?? [];
  }, [data, previousData]);
  const sourceRows = useMemo(
    () => sources.map((source, index) => ({ source, index })),
    [sources],
  );

  // Modals
  const addModal = useDisclosure();
  const editModal = useDisclosure();
  const searchModal = useDisclosure();
  const [editingSource, setEditingSource] = useState<SourceNode | null>(null);
  const [testingId, setTestingId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{
    id: string;
    success: boolean;
    message: string;
  } | null>(null);

  // Search
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SourceReleaseInfo[]>([]);
  const [searching, setSearching] = useState(false);

  // Handle test connection
  const handleTest = useCallback(
    async (id: string) => {
      setTestingId(id);
      setTestResult(null);
      const { data: result } = await testSource({ variables: { Id: id } });
      if (result?.TestSource) {
        setTestResult({
          id,
          success: result.TestSource.Success,
          message: result.TestSource.Success
            ? `Found ${result.TestSource.ReleasesFound ?? 0} releases in ${result.TestSource.ElapsedMs ?? 0}ms`
            : (result.TestSource.Error ?? "Connection failed"),
        });
      }
      setTestingId(null);
    },
    [testSource],
  );

  // Handle delete
  const handleDelete = useCallback(
    async (id: string) => {
      if (!confirm("Are you sure you want to delete this source?")) return;
      await deleteSource({ variables: { Id: id } });
      refetch();
    },
    [deleteSource, refetch],
  );

  // Handle toggle enabled
  const handleToggleEnabled = useCallback(
    async (source: SourceNode) => {
      await updateSource({
        variables: {
          Id: source.Id,
          Input: { Enabled: !source.Enabled },
        },
      });
      refetch();
    },
    [refetch, updateSource],
  );

  // Handle priority change
  const handleMovePriority = useCallback(
    async (sourceId: string, direction: "up" | "down") => {
      const idx = sources.findIndex((s) => s.Id === sourceId);
      if (idx < 0) return;
      const swapIdx = direction === "up" ? idx - 1 : idx + 1;
      if (swapIdx < 0 || swapIdx >= sources.length) return;

      const newOrder = [...sources];
      const [removed] = newOrder.splice(idx, 1);
      newOrder.splice(swapIdx, 0, removed);

      await updateSourcePriorities({
        variables: {
          Input: { SourceIds: newOrder.map((s) => s.Id) },
        },
      });
      refetch();
    },
    [sources, refetch, updateSourcePriorities],
  );

  // Handle search
  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) return;
    setSearching(true);
    setSearchResults([]);
    const { data: result } = await apolloClient.query<SearchSourcesQuery>({
      query: SearchSourcesDocument,
      fetchPolicy: "network-only",
      variables: {
        Input: { Query: searchQuery },
      },
    });
    if (result?.SearchSources) {
      const allReleases = result.SearchSources.Sources.flatMap(
        (s) => s.Releases,
      ).sort((a, b) => {
        const seedDiff = (b.Seeders ?? -1) - (a.Seeders ?? -1);
        if (seedDiff !== 0) return seedDiff;
        return (b.Leechers ?? -1) - (a.Leechers ?? -1);
      });
      setSearchResults(allReleases);
    }
    setSearching(false);
  }, [searchQuery]);

  // Handle edit
  const handleEdit = useCallback(
    (source: SourceNode) => {
      setEditingSource(source);
      editModal.onOpen();
    },
    [editModal],
  );

  const sourceColumns = useMemo<DataTableColumn<(typeof sourceRows)[number]>[]>(
    () => [
      {
        key: "priority",
        label: "#",
        sortable: false,
        width: 80,
        render: ({ source, index }) => (
          <div className="flex flex-col gap-0.5">
            <Button
              isIconOnly
              size="sm"
              variant="light"
              isDisabled={index === 0}
              onPress={() => handleMovePriority(source.Id, "up")}
            >
              <IconArrowUp size={14} />
            </Button>
            <Button
              isIconOnly
              size="sm"
              variant="light"
              isDisabled={index === sources.length - 1}
              onPress={() => handleMovePriority(source.Id, "down")}
            >
              <IconArrowDown size={14} />
            </Button>
          </div>
        ),
      },
      {
        key: "name",
        label: "Name",
        sortable: true,
        render: ({ source }) => (
          <div className="flex flex-col">
            <span className="font-medium">{source.Name}</span>
            <span className="text-xs text-default-400">
              {source.DefinitionId}
            </span>
          </div>
        ),
      },
      {
        key: "sourceType",
        label: "Type",
        sortable: true,
        width: 180,
        render: ({ source }) => <SourceTypeChip type={source.SourceType} />,
      },
      {
        key: "mediaTypes",
        label: "Media",
        sortable: true,
        width: 140,
        render: ({ source }) => (
          <Chip size="sm" variant="flat">
            {source.MediaTypes}
          </Chip>
        ),
      },
      {
        key: "status",
        label: "Status",
        width: 320,
        render: ({ source }) => (
          <div className="flex items-center gap-2">
            <StatusChip source={source} />
            {testResult?.id === source.Id ? (
              <Chip
                size="sm"
                variant="flat"
                color={testResult.success ? "success" : "danger"}
              >
                {testResult.message}
              </Chip>
            ) : null}
          </div>
        ),
      },
      {
        key: "enabled",
        label: "Enabled",
        width: 120,
        align: "center",
        render: ({ source }) => (
          <Switch
            size="sm"
            isSelected={source.Enabled}
            onValueChange={() => void handleToggleEnabled(source)}
          />
        ),
      },
    ],
    [handleMovePriority, handleToggleEnabled, sources.length, testResult],
  );

  const sourceActions = useMemo<RowAction<(typeof sourceRows)[number]>[]>(
    () => [
      {
        key: "test",
        label: "Test connection",
        icon: <IconPlugConnected size={16} className="text-blue-400" />,
        inDropdown: false,
        isDisabled: ({ source }) => testingId === source.Id,
        onAction: ({ source }) => void handleTest(source.Id),
      },
      {
        key: "edit",
        label: "Edit",
        icon: <IconPencil size={16} className="text-default-400" />,
        inDropdown: false,
        onAction: ({ source }) => handleEdit(source),
      },
      {
        key: "delete",
        label: "Delete",
        icon: <IconTrash size={16} className="text-red-400" />,
        isDestructive: true,
        inDropdown: false,
        onAction: ({ source }) => void handleDelete(source.Id),
      },
    ],
    [handleDelete, handleEdit, handleTest, testingId],
  );

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">Sources</h2>
          <p className="text-sm text-default-500 mt-1">
            Manage torrent indexers, usenet indexers, and RSS feeds
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            color="default"
            variant="flat"
            startContent={<IconSearch size={16} />}
            onPress={searchModal.onOpen}
          >
            Search All
          </Button>
          <Button
            color="primary"
            startContent={<IconPlus size={16} />}
            onPress={addModal.onOpen}
          >
            Add Source
          </Button>
        </div>
      </div>

      {/* Sources Table */}
      <Card>
        <CardBody className="p-0">
          <DataTable
            stateKey="settings-sources-table"
            data={sourceRows}
            columns={sourceColumns}
            rowActions={sourceActions}
            getRowKey={(row) => row.source.Id}
            ariaLabel="Sources table"
            searchPlaceholder="Search sources..."
            showItemCount
            isLoading={loading && sourceRows.length === 0}
            emptyContent={
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <IconWorldSearch size={48} className="text-default-300 mb-4" />
                <p className="text-default-500 text-lg font-medium">
                  No sources configured
                </p>
                <p className="text-default-400 text-sm mt-1">
                  Add a torrent indexer, usenet indexer, or RSS feed to get
                  started
                </p>
                <Button
                  color="primary"
                  className="mt-4"
                  startContent={<IconPlus size={16} />}
                  onPress={addModal.onOpen}
                >
                  Add Source
                </Button>
              </div>
            }
          />
        </CardBody>
      </Card>

      {/* Add Source Modal */}
      <AddSourceModal
        isOpen={addModal.isOpen}
        onClose={addModal.onClose}
        onSuccess={() => {
          addModal.onClose();
          refetch();
        }}
      />

      {/* Edit Source Modal */}
      {editingSource && (
        <EditSourceModal
          isOpen={editModal.isOpen}
          onClose={() => {
            editModal.onClose();
            setEditingSource(null);
          }}
          onSuccess={() => {
            editModal.onClose();
            setEditingSource(null);
            refetch();
          }}
          source={editingSource}
        />
      )}

      {/* Search Modal */}
      <SearchSourcesModal
        isOpen={searchModal.isOpen}
        onClose={searchModal.onClose}
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        searchResults={searchResults}
        searching={searching}
        onSearch={handleSearch}
      />
    </div>
  );
}

// =============================================================================
// Add Source Modal
// =============================================================================

function AddSourceModal({
  isOpen,
  onClose,
  onSuccess,
}: {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
}) {
  const [step, setStep] = useState(1);
  const [selectedType, setSelectedType] = useState("");
  const [selectedDefinition, setSelectedDefinition] =
    useState<SourceDefinitionInfo | null>(null);
  const [name, setName] = useState("");
  const [mediaTypes, setMediaTypes] = useState("All");
  const [siteUrl, setSiteUrl] = useState("");
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [settingDefs, setSettingDefs] = useState<SourceSettingDefinition[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createSource] = useMutation(CreateSourceDocument);

  // Fetch available definitions
  const { data: defsData } = useQuery<AvailableSourceDefinitionsQuery>(
    AvailableSourceDefinitionsDocument,
    { skip: !isOpen },
  );
  const definitions = defsData?.AvailableSourceDefinitions ?? [];

  // Filter definitions by selected type
  const filteredDefinitions = useMemo(
    () =>
      definitions.filter((d) => !selectedType || d.SourceType === selectedType),
    [definitions, selectedType],
  );

  // Load setting definitions when a definition is selected
  useEffect(() => {
    if (!selectedDefinition) return;
    apolloClient
      .query<SourceSettingDefinitionsQuery>({
        query: SourceSettingDefinitionsDocument,
        fetchPolicy: "network-only",
        variables: {
          DefinitionId: selectedDefinition.Id,
        },
      })
      .then(({ data }) => {
        if (data?.SourceSettingDefinitions) {
          setSettingDefs(data.SourceSettingDefinitions);
          // Set defaults
          const defaults: Record<string, string> = {};
          for (const s of data.SourceSettingDefinitions) {
            if (s.DefaultValue) defaults[s.Key] = s.DefaultValue;
          }
          setSettings(defaults);
        }
      });
  }, [selectedDefinition]);

  // Reset on close
  useEffect(() => {
    if (!isOpen) {
      setStep(1);
      setSelectedType("");
      setSelectedDefinition(null);
      setName("");
      setMediaTypes("All");
      setSiteUrl("");
      setCredentials({});
      setSettings({});
      setSettingDefs([]);
      setSaving(false);
      setError(null);
    }
  }, [isOpen]);

  const handleSelectDefinition = (def: SourceDefinitionInfo) => {
    setSelectedDefinition(def);
    setSelectedType(def.SourceType);
    setName(def.Name);
    setSiteUrl(def.SiteLink);
    // Init credential fields
    const creds: Record<string, string> = {};
    for (const key of def.RequiredCredentials) {
      creds[key] = "";
    }
    setCredentials(creds);
    setStep(2);
  };

  const handleSave = async () => {
    if (!selectedDefinition) return;
    setSaving(true);
    setError(null);

    // Build credentials JSON string — the backend encrypts this via transform hook
    const credMap: Record<string, string> = {};
    for (const [key, value] of Object.entries(credentials)) {
      if (value.trim()) credMap[key] = value;
    }

    // Build settings JSON string
    const settingsMap: Record<string, string> = {};
    for (const [key, value] of Object.entries(settings)) {
      if (value.trim()) settingsMap[key] = value;
    }

    const { data: result, error: mutationError } = await createSource({
      variables: {
        Input: {
          Name: name,
          SourceType: selectedDefinition.SourceType,
          DefinitionId: selectedDefinition.Id,
          Enabled: true,
          Priority: 100,
          MediaTypes: mediaTypes,
          SiteUrl: siteUrl || null,
          SupportsSearch: true,
          SupportsTvSearch: true,
          SupportsMovieSearch: true,
          SupportsMusicSearch: true,
          SupportsBookSearch: true,
          credentials:
            Object.keys(credMap).length > 0 ? JSON.stringify(credMap) : "",
          Settings:
            Object.keys(settingsMap).length > 0
              ? JSON.stringify(settingsMap)
              : null,
          ErrorCount: 0,
        },
      },
    });

    if (mutationError) {
      setError(mutationError.message);
      setSaving(false);
      return;
    }

    if (result?.CreateSource && !result.CreateSource.Success) {
      setError(result.CreateSource.Error ?? "Failed to create source");
      setSaving(false);
      return;
    }

    setSaving(false);
    onSuccess();
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl">
      <ModalContent>
        <ModalHeader>
          {step === 1
            ? "Select Source Type"
            : `Configure ${selectedDefinition?.Name ?? "Source"}`}
        </ModalHeader>
        <ModalBody>
          {step === 1 ? (
            <div className="space-y-4">
              <p className="text-sm text-default-500">
                Choose a source to add. Each source provides access to different
                content.
              </p>
              {filteredDefinitions.length === 0 ? (
                <div className="flex items-center justify-center py-8">
                  <Spinner label="Loading available sources..." />
                </div>
              ) : (
                <div className="grid gap-3">
                  {filteredDefinitions.map((def) => (
                    <Card
                      key={def.Id}
                      isPressable
                      className="hover:bg-content2 transition-colors"
                      onPress={() => handleSelectDefinition(def)}
                    >
                      <CardBody className="flex flex-row items-center gap-4 py-3">
                        <IconWorldSearch
                          size={32}
                          className="text-default-400 shrink-0"
                        />
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="font-semibold">{def.Name}</span>
                            <SourceTypeChip type={def.SourceType} />
                            <Chip size="sm" variant="flat" color="default">
                              {def.TrackerType}
                            </Chip>
                          </div>
                          <p className="text-sm text-default-500 mt-0.5 truncate">
                            {def.Description}
                          </p>
                        </div>
                      </CardBody>
                    </Card>
                  ))}
                </div>
              )}
            </div>
          ) : (
            <div className="space-y-4">
              {error && (
                <Card className="bg-danger-50 border border-danger-200">
                  <CardBody className="flex flex-row items-center gap-2 py-2">
                    <IconAlertTriangle
                      size={16}
                      className="text-danger shrink-0"
                    />
                    <span className="text-sm text-danger">{error}</span>
                  </CardBody>
                </Card>
              )}

              <Input
                label="Name"
                placeholder="Source name"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />

              <Select
                label="Media Types"
                selectedKeys={[mediaTypes]}
                onSelectionChange={(keys) => {
                  const key = Array.from(keys)[0];
                  if (key) setMediaTypes(String(key));
                }}
              >
                <SelectItem key="All">All</SelectItem>
                <SelectItem key="Movies">Movies</SelectItem>
                <SelectItem key="Tv">TV Shows</SelectItem>
                <SelectItem key="Music">Music</SelectItem>
                <SelectItem key="Audiobooks">Audiobooks</SelectItem>
              </Select>

              <Input
                label="Site URL"
                placeholder="https://..."
                value={siteUrl}
                onChange={(e) => setSiteUrl(e.target.value)}
              />

              <Divider />
              <p className="text-sm font-medium text-default-700">
                Credentials
              </p>

              {selectedDefinition?.RequiredCredentials.map((key) => (
                <Input
                  key={key}
                  label={key}
                  placeholder={`Enter ${key}`}
                  type={
                    key.toLowerCase().includes("password") ||
                    key.toLowerCase().includes("key")
                      ? "password"
                      : "text"
                  }
                  value={credentials[key] ?? ""}
                  onChange={(e) =>
                    setCredentials((prev) => ({
                      ...prev,
                      [key]: e.target.value,
                    }))
                  }
                />
              ))}

              {settingDefs.length > 0 && (
                <>
                  <Divider />
                  <p className="text-sm font-medium text-default-700">
                    Settings
                  </p>
                  {settingDefs.map((def) => {
                    if (def.SettingType === "Checkbox") {
                      return (
                        <Switch
                          key={def.Key}
                          isSelected={settings[def.Key] === "true"}
                          onValueChange={(val) =>
                            setSettings((prev) => ({
                              ...prev,
                              [def.Key]: val ? "true" : "false",
                            }))
                          }
                        >
                          {def.Label}
                        </Switch>
                      );
                    }
                    if (def.SettingType === "Select" && def.Options) {
                      return (
                        <Select
                          key={def.Key}
                          label={def.Label}
                          selectedKeys={
                            settings[def.Key] ? [settings[def.Key]] : []
                          }
                          onSelectionChange={(keys) => {
                            const key = Array.from(keys)[0];
                            if (key)
                              setSettings((prev) => ({
                                ...prev,
                                [def.Key]: String(key),
                              }));
                          }}
                        >
                          {def.Options.map((opt) => (
                            <SelectItem key={opt.Value}>{opt.Label}</SelectItem>
                          ))}
                        </Select>
                      );
                    }
                    return (
                      <Input
                        key={def.Key}
                        label={def.Label}
                        type={
                          def.SettingType === "Password" ? "password" : "text"
                        }
                        value={settings[def.Key] ?? ""}
                        onChange={(e) =>
                          setSettings((prev) => ({
                            ...prev,
                            [def.Key]: e.target.value,
                          }))
                        }
                      />
                    );
                  })}
                </>
              )}
            </div>
          )}
        </ModalBody>
        <ModalFooter>
          {step === 2 && (
            <Button variant="flat" onPress={() => setStep(1)}>
              Back
            </Button>
          )}
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          {step === 2 && (
            <Button color="primary" isLoading={saving} onPress={handleSave}>
              Create Source
            </Button>
          )}
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

// =============================================================================
// Edit Source Modal
// =============================================================================

function EditSourceModal({
  isOpen,
  onClose,
  onSuccess,
  source,
}: {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  source: SourceNode;
}) {
  const [name, setName] = useState(source.Name);
  const [mediaTypes, setMediaTypes] = useState(source.MediaTypes);
  const [siteUrl, setSiteUrl] = useState(source.SiteUrl ?? "");
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [settings, setSettings] = useState<Record<string, string>>(() => {
    try {
      return source.Settings ? JSON.parse(source.Settings) : {};
    } catch {
      return {};
    }
  });
  const [settingDefs, setSettingDefs] = useState<SourceSettingDefinition[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [updateSource] = useMutation(UpdateSourceDocument);

  // Load definition info for credential field names
  const { data: defsData } = useQuery<AvailableSourceDefinitionsQuery>(
    AvailableSourceDefinitionsDocument,
    { skip: !isOpen },
  );
  const definition = useMemo(
    () =>
      defsData?.AvailableSourceDefinitions?.find(
        (d) => d.Id === source.DefinitionId,
      ),
    [defsData, source.DefinitionId],
  );

  // Load setting definitions
  useEffect(() => {
    if (!source.DefinitionId || !isOpen) return;
    apolloClient
      .query<SourceSettingDefinitionsQuery>({
        query: SourceSettingDefinitionsDocument,
        fetchPolicy: "network-only",
        variables: {
          DefinitionId: source.DefinitionId,
        },
      })
      .then(({ data }) => {
        if (data?.SourceSettingDefinitions) {
          setSettingDefs(data.SourceSettingDefinitions);
        }
      });
  }, [source.DefinitionId, isOpen]);

  // Init credential fields (empty for edit - "leave blank to keep existing")
  useEffect(() => {
    if (definition) {
      const creds: Record<string, string> = {};
      for (const key of definition.RequiredCredentials) {
        creds[key] = "";
      }
      setCredentials(creds);
    }
  }, [definition]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);

    // Build update input — only include changed fields
    const input: Record<string, unknown> = {};

    if (name !== source.Name) input.Name = name;
    if (mediaTypes !== source.MediaTypes) input.MediaTypes = mediaTypes;
    if (siteUrl !== (source.SiteUrl ?? "")) input.SiteUrl = siteUrl || null;

    // If any credential fields were filled, send the whole credentials JSON
    const credMap: Record<string, string> = {};
    for (const [key, value] of Object.entries(credentials)) {
      if (value.trim()) credMap[key] = value;
    }
    if (Object.keys(credMap).length > 0) {
      input.Credentials = JSON.stringify(credMap);
    }

    // Settings
    const settingsMap: Record<string, string> = {};
    for (const [key, value] of Object.entries(settings)) {
      if (value.trim()) settingsMap[key] = value;
    }
    if (Object.keys(settingsMap).length > 0) {
      input.Settings = JSON.stringify(settingsMap);
    }

    const { data: result, error: mutationError } = await updateSource({
      variables: {
        Id: source.Id,
        Input: input,
      },
    });

    if (mutationError) {
      setError(mutationError.message);
      setSaving(false);
      return;
    }

    if (result?.UpdateSource && !result.UpdateSource.Success) {
      setError(result.UpdateSource.Error ?? "Failed to update source");
      setSaving(false);
      return;
    }

    setSaving(false);
    onSuccess();
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl">
      <ModalContent>
        <ModalHeader>Edit {source.Name}</ModalHeader>
        <ModalBody>
          <div className="space-y-4">
            {error && (
              <Card className="bg-danger-50 border border-danger-200">
                <CardBody className="flex flex-row items-center gap-2 py-2">
                  <IconAlertTriangle
                    size={16}
                    className="text-danger shrink-0"
                  />
                  <span className="text-sm text-danger">{error}</span>
                </CardBody>
              </Card>
            )}

            <Input
              label="Name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />

            <Select
              label="Media Types"
              selectedKeys={[mediaTypes]}
              onSelectionChange={(keys) => {
                const key = Array.from(keys)[0];
                if (key) setMediaTypes(String(key));
              }}
            >
              <SelectItem key="All">All</SelectItem>
              <SelectItem key="Movies">Movies</SelectItem>
              <SelectItem key="Tv">TV Shows</SelectItem>
              <SelectItem key="Music">Music</SelectItem>
              <SelectItem key="Audiobooks">Audiobooks</SelectItem>
            </Select>

            <Input
              label="Site URL"
              value={siteUrl}
              onChange={(e) => setSiteUrl(e.target.value)}
            />

            <Divider />
            <p className="text-sm font-medium text-default-700">
              Credentials
              <span className="text-xs text-default-400 ml-2">
                (leave blank to keep existing)
              </span>
            </p>

            {definition?.RequiredCredentials.map((key) => (
              <Input
                key={key}
                label={key}
                placeholder={`Enter new ${key} (or leave blank)`}
                type={
                  key.toLowerCase().includes("password") ||
                  key.toLowerCase().includes("key")
                    ? "password"
                    : "text"
                }
                value={credentials[key] ?? ""}
                onChange={(e) =>
                  setCredentials((prev) => ({ ...prev, [key]: e.target.value }))
                }
              />
            ))}

            {settingDefs.length > 0 && (
              <>
                <Divider />
                <p className="text-sm font-medium text-default-700">Settings</p>
                {settingDefs.map((def) => {
                  if (def.SettingType === "Checkbox") {
                    return (
                      <Switch
                        key={def.Key}
                        isSelected={settings[def.Key] === "true"}
                        onValueChange={(val) =>
                          setSettings((prev) => ({
                            ...prev,
                            [def.Key]: val ? "true" : "false",
                          }))
                        }
                      >
                        {def.Label}
                      </Switch>
                    );
                  }
                  if (def.SettingType === "Select" && def.Options) {
                    return (
                      <Select
                        key={def.Key}
                        label={def.Label}
                        selectedKeys={
                          settings[def.Key] ? [settings[def.Key]] : []
                        }
                        onSelectionChange={(keys) => {
                          const key = Array.from(keys)[0];
                          if (key)
                            setSettings((prev) => ({
                              ...prev,
                              [def.Key]: String(key),
                            }));
                        }}
                      >
                        {def.Options.map((opt) => (
                          <SelectItem key={opt.Value}>{opt.Label}</SelectItem>
                        ))}
                      </Select>
                    );
                  }
                  return (
                    <Input
                      key={def.Key}
                      label={def.Label}
                      type={
                        def.SettingType === "Password" ? "password" : "text"
                      }
                      value={settings[def.Key] ?? ""}
                      onChange={(e) =>
                        setSettings((prev) => ({
                          ...prev,
                          [def.Key]: e.target.value,
                        }))
                      }
                    />
                  );
                })}
              </>
            )}
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button color="primary" isLoading={saving} onPress={handleSave}>
            Save Changes
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

// =============================================================================
// Search Sources Modal
// =============================================================================

function SearchSourcesModal({
  isOpen,
  onClose,
  searchQuery,
  setSearchQuery,
  searchResults,
  searching,
  onSearch,
}: {
  isOpen: boolean;
  onClose: () => void;
  searchQuery: string;
  setSearchQuery: (q: string) => void;
  searchResults: SourceReleaseInfo[];
  searching: boolean;
  onSearch: () => void;
}) {
  const [addTorrent] = useMutation(AddTorrentDocument);
  const [addingReleaseKey, setAddingReleaseKey] = useState<string | null>(null);

  const getReleaseKey = useCallback(
    (release: SourceReleaseInfo) =>
      `${release.Guid}:${release.SourceId ?? release.SourceName ?? ""}:${release.Title}`,
    [],
  );

  const handleAddToDownloads = useCallback(
    async (release: SourceReleaseInfo) => {
      const magnetUri = release.MagnetUri ?? undefined;
      const torrentUrl = release.Link ?? undefined;

      if (!magnetUri && !torrentUrl) {
        addToast({
          title: "No Download Link",
          description: "This release does not include a magnet or torrent URL.",
          color: "warning",
        });
        return;
      }

      const isMagnet = magnetUri?.startsWith("magnet:");
      const releaseKey = getReleaseKey(release);
      setAddingReleaseKey(releaseKey);

      try {
        const result = await addTorrent({
          variables: {
            Input: {
              Magnet: isMagnet ? magnetUri : undefined,
              Url: !isMagnet ? magnetUri || torrentUrl : undefined,
            },
          },
        });

        const data = result.data?.AddTorrent;
        if (data?.Success && data.Torrent) {
          addToast({
            title: "Torrent Added",
            description: `Started downloading: ${data.Torrent.Name}`,
            color: "success",
          });
          return;
        }

        addToast({
          title: "Failed to Add Torrent",
          description: sanitizeError(
            data?.Error ?? result.error?.message ?? "Unknown error",
          ),
          color: "danger",
        });
      } catch (error) {
        addToast({
          title: "Failed to Add Torrent",
          description: sanitizeError(error),
          color: "danger",
        });
      } finally {
        setAddingReleaseKey(null);
      }
    },
    [addTorrent, getReleaseKey],
  );

  const columns = useMemo<DataTableColumn<SourceReleaseInfo>[]>(
    () => [
      {
        key: "Title",
        label: "Title",
        sortable: true,
        render: (release) => (
          <div className="truncate" title={release.Title}>
            {release.Details ? (
              <a
                href={release.Details}
                target="_blank"
                rel="noopener noreferrer"
                className="text-primary hover:underline"
              >
                {release.Title}
              </a>
            ) : (
              release.Title
            )}
          </div>
        ),
        width: 500,
      },
      {
        key: "SizeFormatted",
        label: "Size",
        sortable: true,
        align: "end",
        render: (release) => (
          <span className="text-default-500 tabular-nums whitespace-nowrap">
            {release.SizeFormatted ?? "-"}
          </span>
        ),
      },
      {
        key: "Seeders",
        label: "Seeds",
        sortable: true,
        align: "end",
        render: (release) => (
          <span className="text-green-400 tabular-nums">
            {release.Seeders ?? "-"}
          </span>
        ),
      },
      {
        key: "Leechers",
        label: "Leech",
        sortable: true,
        align: "end",
        render: (release) => (
          <span className="text-red-400 tabular-nums">
            {release.Leechers ?? "-"}
          </span>
        ),
      },
      {
        key: "SourceName",
        label: "Source",
        sortable: true,
        render: (release) => (
          <span className="text-default-500 text-xs">
            {release.SourceName ?? "-"}
          </span>
        ),
      },
      {
        key: "IsFreeleech",
        label: "FL",
        sortable: true,
        align: "center",
        render: (release) =>
          release.IsFreeleech ? (
            <Chip size="sm" variant="flat" color="success">
              FL
            </Chip>
          ) : (
            "-"
          ),
      },
      {
        key: "actions",
        label: "Actions",
        sortable: false,
        align: "end",
        render: (release) => {
          const canDownload = Boolean(release.Link ?? release.MagnetUri);
          const tooltipText = canDownload ? "Add to downloads" : null;
          const releaseKey = getReleaseKey(release);

          return canDownload && tooltipText ? (
            <Tooltip content={tooltipText}>
              <Button
                isIconOnly
                size="sm"
                variant="light"
                isLoading={addingReleaseKey === releaseKey}
                onPress={() => void handleAddToDownloads(release)}
              >
                <IconDownload size={14} className="text-blue-400" />
              </Button>
            </Tooltip>
          ) : (
            <span className="text-default-400">-</span>
          );
        },
      },
    ],
    [addingReleaseKey, getReleaseKey, handleAddToDownloads],
  );

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      size="full"
      className="max-h-[80vh] max-w-[800]"
    >
      <ModalContent>
        <ModalHeader>Search All Sources</ModalHeader>
        <ModalBody>
          <div className="space-y-4 grow h-0">
            <div className="flex gap-2">
              <Input
                placeholder="Search for movies, shows, music..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                startContent={
                  <IconSearch size={16} className="text-default-400" />
                }
                onKeyDown={(e) => {
                  if (e.key === "Enter") onSearch();
                }}
                className="flex-1"
              />
              <Button color="primary" isLoading={searching} onPress={onSearch}>
                Search
              </Button>
            </div>

            <DataTable
              stateKey="settings-sources-search-results"
              data={searchResults}
              isLoading={searching}
              columns={columns}
              getRowKey={(release) => getReleaseKey(release)}
              defaultSortColumn="Seeders"
              defaultSortDirection="desc"
              searchPlaceholder="Filter results..."
              removeWrapper
              showItemCount
              fillHeight
              ariaLabel="Search all sources results"
            />
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Close
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
