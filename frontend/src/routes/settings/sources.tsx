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
  CreateSourceDocument,
  UpdateSourceDocument,
  DeleteSourceDocument,
  TestSourceDocument,
  UpdateSourcePrioritiesDocument,
  type SourcesQuery,
  type AvailableSourceDefinitionsQuery,
  type SourceSettingDefinitionsQuery,
} from "../../lib/graphql/generated/graphql";
import { apolloClient, useMutation, useQuery } from "../../lib/graphql/client";
import { SearchSourcesModal } from "../../components/SearchSourcesModal";

type SourceNode = SourcesQuery["sources"]["edges"][number]["node"];
type SourceDefinitionInfo =
  AvailableSourceDefinitionsQuery["availableSourceDefinitions"][number];
type SourceSettingDefinition =
  SourceSettingDefinitionsQuery["sourceSettingDefinitions"][number];

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
    rssFeed: "warning",
  };
  const labelMap: Record<string, string> = {
    TorrentIndexer: "Torrent Indexer",
    UsenetIndexer: "Usenet Indexer",
    rssFeed: "RSS Feed",
  };
  return (
    <Chip size="sm" variant="flat" color={colorMap[type] ?? "default"}>
      {labelMap[type] ?? type}
    </Chip>
  );
}

function StatusChip({ source }: { source: SourceNode }) {
  if (!source.enabled) {
    return (
      <Chip size="sm" variant="flat" color="default">
        Disabled
      </Chip>
    );
  }
  if (source.errorCount > 0) {
    return (
      <Tooltip content={source.lastError ?? "Unknown error"}>
        <Chip size="sm" variant="flat" color="danger">
          Error
        </Chip>
      </Tooltip>
    );
  }
  if (source.lastSuccessAt) {
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
      variables: { orderBy: [{ priority: "ASC" }] },
      fetchPolicy: "cache-and-network",
    },
  );
  const [testSource] = useMutation(TestSourceDocument);
  const [deleteSource] = useMutation(DeleteSourceDocument);
  const [updateSource] = useMutation(UpdateSourceDocument);
  const [updateSourcePriorities] = useMutation(UpdateSourcePrioritiesDocument);

  const sources = useMemo(() => {
    const d = data ?? previousData;
    return d?.sources?.edges?.map((e) => e.node) ?? [];
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

  // Handle test connection
  const handleTest = useCallback(
    async (id: string) => {
      setTestingId(id);
      setTestResult(null);
      const { data: result } = await testSource({ variables: { id: id } });
      if (result?.testSource) {
        setTestResult({
          id,
          success: result.testSource.success,
          message: result.testSource.success
            ? `Found ${result.testSource.releasesFound ?? 0} releases in ${result.testSource.elapsedMs ?? 0}ms`
            : (result.testSource.error ?? "Connection failed"),
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
      await deleteSource({ variables: { id: id } });
      refetch();
    },
    [deleteSource, refetch],
  );

  // Handle toggle enabled
  const handleToggleEnabled = useCallback(
    async (source: SourceNode) => {
      await updateSource({
        variables: {
          id: source.id,
          input: { enabled: !source.enabled },
        },
      });
      refetch();
    },
    [refetch, updateSource],
  );

  // Handle priority change
  const handleMovePriority = useCallback(
    async (sourceId: string, direction: "up" | "down") => {
      const idx = sources.findIndex((s) => s.id === sourceId);
      if (idx < 0) return;
      const swapIdx = direction === "up" ? idx - 1 : idx + 1;
      if (swapIdx < 0 || swapIdx >= sources.length) return;

      const newOrder = [...sources];
      const [removed] = newOrder.splice(idx, 1);
      newOrder.splice(swapIdx, 0, removed);

      await updateSourcePriorities({
        variables: {
          input: { sourceIds: newOrder.map((s) => s.id) },
        },
      });
      refetch();
    },
    [sources, refetch, updateSourcePriorities],
  );

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
              onPress={() => handleMovePriority(source.id, "up")}
            >
              <IconArrowUp size={14} />
            </Button>
            <Button
              isIconOnly
              size="sm"
              variant="light"
              isDisabled={index === sources.length - 1}
              onPress={() => handleMovePriority(source.id, "down")}
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
            <span className="font-medium">{source.name}</span>
            <span className="text-xs text-default-400">
              {source.definitionId}
            </span>
          </div>
        ),
      },
      {
        key: "sourceType",
        label: "Type",
        sortable: true,
        width: 180,
        render: ({ source }) => <SourceTypeChip type={source.sourceType} />,
      },
      {
        key: "mediaTypes",
        label: "Media",
        sortable: true,
        width: 140,
        render: ({ source }) => (
          <Chip size="sm" variant="flat">
            {source.mediaTypes}
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
            {testResult?.id === source.id ? (
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
            isSelected={source.enabled}
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
        isDisabled: ({ source }) => testingId === source.id,
        onAction: ({ source }) => void handleTest(source.id),
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
        onAction: ({ source }) => void handleDelete(source.id),
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
            Manage the torrent indexers and feeds available in this build
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
            getRowKey={(row) => row.source.id}
            fillHeight={false}
            ariaLabel="Sources table"
            toolbarQueryPlaceholder="Search sources..."
            showItemCount
            isLoading={loading && sourceRows.length === 0}
            emptyContent={
              <div className="flex flex-col items-center justify-center py-12 text-center">
                <IconWorldSearch size={48} className="text-default-300 mb-4" />
                <p className="text-default-500 text-lg font-medium">
                  No sources configured
                </p>
                <p className="text-default-400 text-sm mt-1">
                  Add an available torrent indexer or feed to get started
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
  const definitions = defsData?.availableSourceDefinitions ?? [];

  // Filter definitions by selected type
  const filteredDefinitions = useMemo(
    () =>
      definitions.filter((d) => !selectedType || d.sourceType === selectedType),
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
          definitionId: selectedDefinition.id,
        },
      })
      .then(({ data }) => {
        if (data?.sourceSettingDefinitions) {
          setSettingDefs(data.sourceSettingDefinitions);
          // Set defaults
          const defaults: Record<string, string> = {};
          for (const s of data.sourceSettingDefinitions) {
            if (s.defaultValue) defaults[s.key] = s.defaultValue;
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
    setSelectedType(def.sourceType);
    setName(def.name);
    setSiteUrl(def.siteLink);
    // Init credential fields
    const creds: Record<string, string> = {};
    for (const key of def.requiredCredentials) {
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
        input: {
          name: name,
          sourceType: selectedDefinition.sourceType,
          definitionId: selectedDefinition.id,
          enabled: true,
          priority: 100,
          mediaTypes: mediaTypes,
          siteUrl: siteUrl || null,
          supportsSearch: true,
          supportsTvSearch: true,
          supportsMovieSearch: true,
          supportsMusicSearch: true,
          supportsBookSearch: true,
          credentials:
            Object.keys(credMap).length > 0 ? JSON.stringify(credMap) : "",
          settings:
            Object.keys(settingsMap).length > 0
              ? JSON.stringify(settingsMap)
              : null,
          errorCount: 0,
        },
      },
    });

    if (mutationError) {
      setError(mutationError.message);
      setSaving(false);
      return;
    }

    if (result?.createSource && !result.createSource.success) {
      setError(result.createSource.error ?? "Failed to create source");
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
            : `Configure ${selectedDefinition?.name ?? "Source"}`}
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
                      key={def.id}
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
                            <span className="font-semibold">{def.name}</span>
                            <SourceTypeChip type={def.sourceType} />
                            <Chip size="sm" variant="flat" color="default">
                              {def.trackerType}
                            </Chip>
                          </div>
                          <p className="text-sm text-default-500 mt-0.5 truncate">
                            {def.description}
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

              {selectedDefinition?.requiredCredentials.map((key) => (
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
                    if (def.settingType === "Checkbox") {
                      return (
                        <Switch
                          key={def.key}
                          isSelected={settings[def.key] === "true"}
                          onValueChange={(val) =>
                            setSettings((prev) => ({
                              ...prev,
                              [def.key]: val ? "true" : "false",
                            }))
                          }
                        >
                          {def.label}
                        </Switch>
                      );
                    }
                    if (def.settingType === "Select" && def.options) {
                      return (
                        <Select
                          key={def.key}
                          label={def.label}
                          selectedKeys={
                            settings[def.key] ? [settings[def.key]] : []
                          }
                          onSelectionChange={(keys) => {
                            const key = Array.from(keys)[0];
                            if (key)
                              setSettings((prev) => ({
                                ...prev,
                                [def.key]: String(key),
                              }));
                          }}
                        >
                          {def.options.map((opt) => (
                            <SelectItem key={opt.value}>{opt.label}</SelectItem>
                          ))}
                        </Select>
                      );
                    }
                    return (
                      <Input
                        key={def.key}
                        label={def.label}
                        type={
                          def.settingType === "Password" ? "password" : "text"
                        }
                        value={settings[def.key] ?? ""}
                        onChange={(e) =>
                          setSettings((prev) => ({
                            ...prev,
                            [def.key]: e.target.value,
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
  const [name, setName] = useState(source.name);
  const [mediaTypes, setMediaTypes] = useState(source.mediaTypes);
  const [siteUrl, setSiteUrl] = useState(source.siteUrl ?? "");
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const [settings, setSettings] = useState<Record<string, string>>(() => {
    try {
      return source.settings ? JSON.parse(source.settings) : {};
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
      defsData?.availableSourceDefinitions?.find(
        (d) => d.id === source.definitionId,
      ),
    [defsData, source.definitionId],
  );

  // Load setting definitions
  useEffect(() => {
    if (!source.definitionId || !isOpen) return;
    apolloClient
      .query<SourceSettingDefinitionsQuery>({
        query: SourceSettingDefinitionsDocument,
        fetchPolicy: "network-only",
        variables: {
          definitionId: source.definitionId,
        },
      })
      .then(({ data }) => {
        if (data?.sourceSettingDefinitions) {
          setSettingDefs(data.sourceSettingDefinitions);
        }
      });
  }, [source.definitionId, isOpen]);

  // Init credential fields (empty for edit - "leave blank to keep existing")
  useEffect(() => {
    if (definition) {
      const creds: Record<string, string> = {};
      for (const key of definition.requiredCredentials) {
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

    if (name !== source.name) input.name = name;
    if (mediaTypes !== source.mediaTypes) input.mediaTypes = mediaTypes;
    if (siteUrl !== (source.siteUrl ?? "")) input.siteUrl = siteUrl || null;

    // If any credential fields were filled, send the whole credentials JSON
    const credMap: Record<string, string> = {};
    for (const [key, value] of Object.entries(credentials)) {
      if (value.trim()) credMap[key] = value;
    }
    if (Object.keys(credMap).length > 0) {
      input.credentials = JSON.stringify(credMap);
    }

    // Settings
    const settingsMap: Record<string, string> = {};
    for (const [key, value] of Object.entries(settings)) {
      if (value.trim()) settingsMap[key] = value;
    }
    if (Object.keys(settingsMap).length > 0) {
      input.settings = JSON.stringify(settingsMap);
    }

    const { data: result, error: mutationError } = await updateSource({
      variables: {
        id: source.id,
        input: input,
      },
    });

    if (mutationError) {
      setError(mutationError.message);
      setSaving(false);
      return;
    }

    if (result?.updateSource && !result.updateSource.success) {
      setError(result.updateSource.error ?? "Failed to update source");
      setSaving(false);
      return;
    }

    setSaving(false);
    onSuccess();
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl">
      <ModalContent>
        <ModalHeader>Edit {source.name}</ModalHeader>
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

            {definition?.requiredCredentials.map((key) => (
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
                  if (def.settingType === "Checkbox") {
                    return (
                      <Switch
                        key={def.key}
                        isSelected={settings[def.key] === "true"}
                        onValueChange={(val) =>
                          setSettings((prev) => ({
                            ...prev,
                            [def.key]: val ? "true" : "false",
                          }))
                        }
                      >
                        {def.label}
                      </Switch>
                    );
                  }
                  if (def.settingType === "Select" && def.options) {
                    return (
                      <Select
                        key={def.key}
                        label={def.label}
                        selectedKeys={
                          settings[def.key] ? [settings[def.key]] : []
                        }
                        onSelectionChange={(keys) => {
                          const key = Array.from(keys)[0];
                          if (key)
                            setSettings((prev) => ({
                              ...prev,
                              [def.key]: String(key),
                            }));
                        }}
                      >
                        {def.options.map((opt) => (
                          <SelectItem key={opt.value}>{opt.label}</SelectItem>
                        ))}
                      </Select>
                    );
                  }
                  return (
                    <Input
                      key={def.key}
                      label={def.label}
                      type={
                        def.settingType === "Password" ? "password" : "text"
                      }
                      value={settings[def.key] ?? ""}
                      onChange={(e) =>
                        setSettings((prev) => ({
                          ...prev,
                          [def.key]: e.target.value,
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
