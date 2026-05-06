import { createFileRoute, redirect } from "@tanstack/react-router";
import { useState, useEffect, useCallback } from "react";
import { Card, CardBody } from "@heroui/card";
import { Button } from "@heroui/button";
import { Chip } from "@heroui/chip";
import { Divider } from "@heroui/divider";
import { Switch } from "@heroui/switch";
import { Spinner } from "@heroui/spinner";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
} from "@heroui/modal";
import { Input } from "@heroui/input";
import { Tooltip } from "@heroui/tooltip";
import { addToast } from "@heroui/toast";
import {
  IconPlus,
  IconTrash,
  IconEdit,
  IconServer,
  IconLock,
  IconLockOpen,
  IconArrowUp,
  IconArrowDown,
} from "@tabler/icons-react";
import {
  SettingsCreateUsenetServerDocument,
  SettingsDeleteUsenetServerDocument,
  SettingsUsenetServersDocument,
  SettingsUpdateUsenetServerDocument,
  type SettingsUsenetServersQuery,
} from "../../lib/graphql/generated/graphql";
import { apolloClient, useMutation } from "../../lib/graphql/client";
import { sanitizeError } from "../../lib/format";
import { InlineError } from "../../components/shared";
import { getStoredUser } from "../../lib/auth";

export const Route = createFileRoute("/settings/usenet")({
  beforeLoad: ({ context, location }) => {
    if (!context.auth.isAuthenticated) {
      throw redirect({
        to: "/",
        search: {
          signin: true,
          redirect: location.href,
        },
      });
    }
  },
  component: UsenetSettingsPage,
});

type UsenetServerRow =
  SettingsUsenetServersQuery["UsenetServers"]["Edges"][number]["Node"];

function UsenetSettingsPage() {
  const [servers, setServers] = useState<UsenetServerRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Modal state
  const { isOpen, onOpen, onClose } = useDisclosure();
  const [editingServer, setEditingServer] = useState<UsenetServerRow | null>(
    null,
  );
  const [formData, setFormData] = useState({
    name: "",
    host: "",
    port: 563,
    useSsl: true,
    username: "",
    password: "",
    connections: 10,
    retentionDays: null as number | null,
  });
  const [saving, setSaving] = useState(false);
  const [createUsenetServerEntity] = useMutation(
    SettingsCreateUsenetServerDocument,
  );
  const [updateUsenetServerEntity] = useMutation(
    SettingsUpdateUsenetServerDocument,
  );
  const [deleteUsenetServerEntity] = useMutation(
    SettingsDeleteUsenetServerDocument,
  );

  // Load servers
  const loadServers = useCallback(async () => {
    try {
      setLoading(true);
      const { data } = await apolloClient.query<SettingsUsenetServersQuery>({
        query: SettingsUsenetServersDocument,
        fetchPolicy: "network-only",
        variables: {
          OrderBy: [{ Priority: "ASC" }],
          Page: { limit: 200, offset: 0 },
        },
      });

      setServers(data?.UsenetServers?.Edges.map((edge) => edge.Node) || []);
      setError(null);
    } catch (err) {
      // Silently ignore auth errors - they can happen during login race conditions
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (errorMsg.toLowerCase().includes("authentication")) {
        // Clear any error for auth issues
        setError(null);
      } else {
        setError(sanitizeError(err));
      }
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadServers();
  }, [loadServers]);

  // Open modal for adding new server
  const handleAdd = () => {
    setEditingServer(null);
    setFormData({
      name: "",
      host: "",
      port: 563,
      useSsl: true,
      username: "",
      password: "",
      connections: 10,
      retentionDays: null,
    });
    onOpen();
  };

  // Open modal for editing server
  const handleEdit = (server: UsenetServerRow) => {
    setEditingServer(server);
    setFormData({
      name: server.Name,
      host: server.Host,
      port: server.Port,
      useSsl: server.UseSsl,
      username: server.Username || "",
      password: "",
      connections: server.Connections,
      retentionDays: server.RetentionDays ?? null,
    });
    onOpen();
  };

  // Save server (create or update)
  const handleSave = async () => {
    try {
      setSaving(true);

      if (editingServer) {
        const { data } = await updateUsenetServerEntity({
          variables: {
            Id: editingServer.Id,
            Input: {
              Name: formData.name || undefined,
              Host: formData.host || undefined,
              Port: formData.port,
              UseSsl: formData.useSsl,
              Username: formData.username || undefined,
              Connections: formData.connections,
              RetentionDays: formData.retentionDays,
            },
          },
        });

        if (!data?.UpdateUsenetServer?.Success) {
          throw new Error(
            data?.UpdateUsenetServer?.Error || "Failed to update",
          );
        }
        if (formData.password.trim().length > 0) {
          addToast({
            title: "Password unchanged",
            description:
              "Typed Usenet update does not currently support plaintext passwords.",
            color: "warning",
          });
        }
        addToast({ title: "Server updated", color: "success" });
      } else {
        const userId = getStoredUser()?.id;
        if (!userId) {
          throw new Error("Unable to determine current user");
        }
        const { data } = await createUsenetServerEntity({
          variables: {
            Input: {
              UserId: userId,
              Name: formData.name,
              Host: formData.host,
              Port: formData.port,
              UseSsl: formData.useSsl,
              Username: formData.username || undefined,
              Connections: formData.connections,
              Priority: servers.length + 1,
              RetentionDays: formData.retentionDays,
              Enabled: true,
              ErrorCount: 0,
            },
          },
        });

        if (!data?.CreateUsenetServer?.Success) {
          throw new Error(
            data?.CreateUsenetServer?.Error || "Failed to create",
          );
        }
        if (formData.password.trim().length > 0) {
          addToast({
            title: "Password not saved",
            description:
              "Typed Usenet create does not currently support plaintext passwords.",
            color: "warning",
          });
        }
        addToast({ title: "Server added", color: "success" });
      }

      onClose();
      loadServers();
    } catch (err) {
      addToast({
        title: "Error",
        description: sanitizeError(err),
        color: "danger",
      });
    } finally {
      setSaving(false);
    }
  };

  // Delete server
  const handleDelete = async (server: UsenetServerRow) => {
    if (!confirm(`Delete server "${server.Name}"?`)) return;

    try {
      const { data } = await deleteUsenetServerEntity({
        variables: { Id: server.Id },
      });

      if (!data?.DeleteUsenetServer?.Success) {
        throw new Error(data?.DeleteUsenetServer?.Error || "Failed to delete");
      }

      addToast({ title: "Server deleted", color: "success" });
      loadServers();
    } catch (err) {
      addToast({
        title: "Error",
        description: sanitizeError(err),
        color: "danger",
      });
    }
  };

  // Toggle enabled
  const handleToggleEnabled = async (server: UsenetServerRow) => {
    try {
      const { data } = await updateUsenetServerEntity({
        variables: {
          Id: server.Id,
          Input: { Enabled: !server.Enabled },
        },
      });

      if (!data?.UpdateUsenetServer?.Success) {
        throw new Error(data?.UpdateUsenetServer?.Error || "Failed to update");
      }

      loadServers();
    } catch (err) {
      addToast({
        title: "Error",
        description: sanitizeError(err),
        color: "danger",
      });
    }
  };

  // Move server up/down in priority
  const handleMove = async (index: number, direction: "up" | "down") => {
    const newIndex = direction === "up" ? index - 1 : index + 1;
    if (newIndex < 0 || newIndex >= servers.length) return;

    const newServers = [...servers];
    const [moved] = newServers.splice(index, 1);
    newServers.splice(newIndex, 0, moved);

    // Update UI optimistically
    setServers(newServers);

    // Save new order
    try {
      for (let i = 0; i < newServers.length; i += 1) {
        const targetPriority = i + 1;
        if (newServers[i].Priority === targetPriority) continue;

        const { data } = await updateUsenetServerEntity({
          variables: {
            Id: newServers[i].Id,
            Input: { Priority: targetPriority },
          },
        });

        if (!data?.UpdateUsenetServer?.Success) {
          throw new Error(
            data?.UpdateUsenetServer?.Error || "Failed to reorder servers",
          );
        }
      }
    } catch (err) {
      addToast({
        title: "Error reordering",
        description: sanitizeError(err),
        color: "danger",
      });
      loadServers(); // Reload to get correct order
    }
  };

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64">
        <Spinner size="lg" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Usenet Servers</h1>
          <p className="text-default-500">
            Configure your Usenet news server providers
          </p>
        </div>
        <Button
          color="primary"
          startContent={<IconPlus size={16} />}
          onPress={handleAdd}
        >
          Add Server
        </Button>
      </div>

      {error && <InlineError message={error} />}

      {servers.length === 0 ? (
        <Card>
          <CardBody className="text-center py-12">
            <IconServer size={48} className="mx-auto text-default-300 mb-4" />
            <p className="text-default-500">No Usenet servers configured</p>
            <p className="text-default-400 text-sm mb-4">
              Add a server to start downloading from Usenet
            </p>
            <Button color="primary" onPress={handleAdd}>
              Add Your First Server
            </Button>
          </CardBody>
        </Card>
      ) : (
        <div className="space-y-3">
          {servers.map((server, index) => (
            <Card key={server.Id}>
              <CardBody className="flex flex-row items-center gap-4">
                {/* Priority controls */}
                <div className="flex flex-col gap-0.5">
                  <Button
                    isIconOnly
                    size="sm"
                    variant="light"
                    isDisabled={index === 0}
                    onPress={() => handleMove(index, "up")}
                  >
                    <IconArrowUp size={14} />
                  </Button>
                  <Button
                    isIconOnly
                    size="sm"
                    variant="light"
                    isDisabled={index === servers.length - 1}
                    onPress={() => handleMove(index, "down")}
                  >
                    <IconArrowDown size={14} />
                  </Button>
                </div>

                {/* Server info */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium truncate">{server.Name}</span>
                    {server.UseSsl ? (
                      <Tooltip content="SSL/TLS enabled">
                        <IconLock size={14} className="text-green-500" />
                      </Tooltip>
                    ) : (
                      <Tooltip content="No SSL">
                        <IconLockOpen size={14} className="text-amber-500" />
                      </Tooltip>
                    )}
                    {server.ErrorCount > 0 && (
                      <Chip color="danger" size="sm" variant="flat">
                        {server.ErrorCount} errors
                      </Chip>
                    )}
                  </div>
                  <div className="text-sm text-default-500">
                    {server.Host}:{server.Port} • {server.Connections}{" "}
                    connections
                    {server.RetentionDays &&
                      ` • ${server.RetentionDays} days retention`}
                  </div>
                  {server.LastError && (
                    <div className="text-xs text-danger mt-1 truncate">
                      {server.LastError}
                    </div>
                  )}
                </div>

                {/* Actions */}
                <div className="flex items-center gap-2">
                  <Switch
                    isSelected={server.Enabled}
                    onValueChange={() => handleToggleEnabled(server)}
                    size="sm"
                  />
                  <Tooltip content="Edit">
                    <Button
                      isIconOnly
                      size="sm"
                      variant="light"
                      onPress={() => handleEdit(server)}
                    >
                      <IconEdit size={16} />
                    </Button>
                  </Tooltip>
                  <Tooltip content="Delete">
                    <Button
                      isIconOnly
                      size="sm"
                      variant="light"
                      color="danger"
                      onPress={() => handleDelete(server)}
                    >
                      <IconTrash size={16} />
                    </Button>
                  </Tooltip>
                </div>
              </CardBody>
            </Card>
          ))}
        </div>
      )}

      {/* Add/Edit Modal */}
      <Modal isOpen={isOpen} onClose={onClose} size="lg">
        <ModalContent>
          <ModalHeader>
            {editingServer ? "Edit Server" : "Add Server"}
          </ModalHeader>
          <ModalBody className="gap-4">
            <Input
              label="Name"
              placeholder="My Usenet Provider"
              value={formData.name}
              onChange={(e) =>
                setFormData({ ...formData, name: e.target.value })
              }
              isRequired
            />
            <div className="flex gap-4">
              <Input
                label="Host"
                placeholder="news.example.com"
                value={formData.host}
                onChange={(e) =>
                  setFormData({ ...formData, host: e.target.value })
                }
                isRequired
                className="flex-1"
              />
              <Input
                label="Port"
                type="number"
                value={formData.port.toString()}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    port: parseInt(e.target.value) || 563,
                  })
                }
                className="w-24"
              />
            </div>
            <Switch
              isSelected={formData.useSsl}
              onValueChange={(v) =>
                setFormData({ ...formData, useSsl: v, port: v ? 563 : 119 })
              }
            >
              Use SSL/TLS
            </Switch>
            <Divider />
            <div className="flex gap-4">
              <Input
                label="Username"
                placeholder="(optional)"
                value={formData.username}
                onChange={(e) =>
                  setFormData({ ...formData, username: e.target.value })
                }
                className="flex-1"
              />
              <Input
                label="Password"
                type="password"
                placeholder={editingServer ? "(unchanged)" : "(optional)"}
                value={formData.password}
                onChange={(e) =>
                  setFormData({ ...formData, password: e.target.value })
                }
                className="flex-1"
              />
            </div>
            <Divider />
            <div className="flex gap-4">
              <Input
                label="Connections"
                type="number"
                value={formData.connections.toString()}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    connections: parseInt(e.target.value) || 10,
                  })
                }
                description="Number of simultaneous connections"
                className="flex-1"
              />
              <Input
                label="Retention (days)"
                type="number"
                placeholder="(optional)"
                value={formData.retentionDays?.toString() || ""}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    retentionDays: e.target.value
                      ? parseInt(e.target.value)
                      : null,
                  })
                }
                description="How many days of articles the server keeps"
                className="flex-1"
              />
            </div>
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={onClose}>
              Cancel
            </Button>
            <Button
              color="primary"
              onPress={handleSave}
              isLoading={saving}
              isDisabled={!formData.name || !formData.host}
            >
              {editingServer ? "Save" : "Add"}
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </div>
  );
}
