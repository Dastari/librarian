import { createFileRoute } from "@tanstack/react-router";
import { useState, useEffect, useCallback, useMemo } from "react";
import { Input } from "@heroui/input";
import { Switch } from "@heroui/switch";
import { Accordion, AccordionItem } from "@heroui/accordion";
import { Button } from "@heroui/button";
import { addToast } from "@heroui/toast";
import { useMutation, useQuery } from "../../lib/graphql/client";
import {
  TorrentAppSettingsDocument,
  CreateAppSettingDocument,
  UpdateAppSettingDocument,
  type TorrentAppSettingsQuery,
} from "../../lib/graphql/generated/graphql";
import { FolderBrowserInput } from "../../components/FolderBrowserInput";
import { SettingsHeader } from "../../components/shared";
import { sanitizeError } from "../../lib/format";
import {
  IconFolder,
  IconNetwork,
  IconGauge,
  IconTestPipe,
  IconAlertTriangle,
  IconCheck,
  IconX,
} from "@tabler/icons-react";

interface UpnpResult {
  success: boolean;
  tcpForwarded: boolean;
  udpForwarded: boolean;
  localIp: string | null;
  externalIp: string | null;
  error: string | null;
}

interface PortTestResult {
  success: boolean;
  portOpen: boolean;
  externalIp: string | null;
  error: string | null;
}

const TORRENT_CATEGORY = "torrent";
const TORRENT_KEYS = {
  download_dir: "torrent.download_dir",
  session_dir: "torrent.session_dir",
  enable_dht: "torrent.enable_dht",
  listen_port: "torrent.listen_port",
  max_concurrent: "torrent.max_concurrent",
  upload_limit: "torrent.upload_limit",
  download_limit: "torrent.download_limit",
} as const;

/** Shape used by the form and for change detection (from AppSettings key/value store). */
interface TorrentSettingsShape {
  downloadDir: string;
  sessionDir: string;
  enableDht: boolean;
  listenPort: number;
  maxConcurrent: number;
  uploadLimit: number;
  downloadLimit: number;
}

function appSettingsToTorrentSettings(
  edges: TorrentAppSettingsQuery["AppSettings"]["Edges"],
): TorrentSettingsShape {
  const map = new Map(edges.map((e) => [e.Node.Key, e.Node.Value]));
  const get = (k: string, def: string) => map.get(k) ?? def;
  return {
    downloadDir: get(TORRENT_KEYS.download_dir, ""),
    sessionDir: get(TORRENT_KEYS.session_dir, ""),
    enableDht: get(TORRENT_KEYS.enable_dht, "true") === "true",
    listenPort: parseInt(get(TORRENT_KEYS.listen_port, "6881"), 10) || 6881,
    maxConcurrent: parseInt(get(TORRENT_KEYS.max_concurrent, "5"), 10) || 5,
    uploadLimit: parseInt(get(TORRENT_KEYS.upload_limit, "0"), 10) || 0,
    downloadLimit: parseInt(get(TORRENT_KEYS.download_limit, "0"), 10) || 0,
  };
}

/** Map from app setting key to node Id (for updates). */
function keyToIdMap(
  edges: TorrentAppSettingsQuery["AppSettings"]["Edges"],
): Map<string, string> {
  return new Map(edges.map((e) => [e.Node.Key, e.Node.Id]));
}

export const Route = createFileRoute("/settings/torrent")({
  component: TorrentSettingsPage,
});

function TorrentSettingsPage() {
  const [originalSettings, setOriginalSettings] =
    useState<TorrentSettingsShape | null>(null);
  const [settingIds, setSettingIds] = useState<Map<string, string>>(new Map());
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);

  const [downloadDir, setDownloadDir] = useState("");
  const [sessionDir, setSessionDir] = useState("");
  const [enableDht, setEnableDht] = useState(true);
  const [listenPort, setListenPort] = useState(6881);
  const [maxConcurrent, setMaxConcurrent] = useState(5);
  const [uploadLimit, setUploadLimit] = useState(0);
  const [downloadLimit, setDownloadLimit] = useState(0);

  const [upnpStatus, setUpnpStatus] = useState<UpnpResult | null>(null);
  const [portTestResult, setPortTestResult] = useState<PortTestResult | null>(
    null,
  );
  const [isTestingPort, setIsTestingPort] = useState(false);
  const [isAttemptingUpnp, setIsAttemptingUpnp] = useState(false);

  const hasChanges = useMemo(() => {
    if (!originalSettings) return false;
    return (
      downloadDir !== originalSettings.downloadDir ||
      sessionDir !== originalSettings.sessionDir ||
      enableDht !== originalSettings.enableDht ||
      listenPort !== originalSettings.listenPort ||
      maxConcurrent !== originalSettings.maxConcurrent ||
      uploadLimit !== originalSettings.uploadLimit ||
      downloadLimit !== originalSettings.downloadLimit
    );
  }, [
    originalSettings,
    downloadDir,
    sessionDir,
    enableDht,
    listenPort,
    maxConcurrent,
    uploadLimit,
    downloadLimit,
  ]);

  const {
    data: settingsData,
    loading: settingsLoading,
    error: settingsError,
    refetch: refetchSettings,
  } = useQuery<TorrentAppSettingsQuery>(TorrentAppSettingsDocument, {
    fetchPolicy: "network-only",
    notifyOnNetworkStatusChange: false,
    variables: {},
  });
  const [updateAppSetting] = useMutation(UpdateAppSettingDocument);
  const [createAppSetting] = useMutation(CreateAppSettingDocument);

  useEffect(() => {
    if (!settingsData?.AppSettings?.Edges) return;
    if (originalSettings !== null) return;
    const settings = appSettingsToTorrentSettings(
      settingsData.AppSettings.Edges,
    );
    setOriginalSettings(settings);
    setSettingIds(keyToIdMap(settingsData.AppSettings.Edges));
    setDownloadDir(settings.downloadDir);
    setSessionDir(settings.sessionDir);
    setEnableDht(settings.enableDht);
    setListenPort(settings.listenPort);
    setMaxConcurrent(settings.maxConcurrent);
    setUploadLimit(settings.uploadLimit);
    setDownloadLimit(settings.downloadLimit);
    setIsLoading(false);
  }, [originalSettings, settingsData]);

  useEffect(() => {
    if (settingsLoading) return;
    setIsLoading(false);
  }, [settingsLoading]);

  useEffect(() => {
    if (!settingsError) return;
    const isAuthError = settingsError.message
      .toLowerCase()
      .includes("authentication");
    if (!isAuthError) {
      addToast({
        title: "Error",
        description: sanitizeError(settingsError),
        color: "danger",
      });
    }
  }, [settingsError]);

  const fetchUpnpStatus = useCallback(async () => {
    setUpnpStatus(null);
  }, []);

  const testPortAccessibility = useCallback(async () => {
    setIsTestingPort(true);
    try {
      setPortTestResult({
        success: false,
        portOpen: false,
        externalIp: null,
        error: "Typed port-test resolver is not available yet.",
      });
      addToast({
        title: "Port test unavailable",
        description: `Port ${listenPort} test requires a typed backend resolver.`,
        color: "warning",
      });
    } finally {
      setIsTestingPort(false);
    }
  }, [listenPort]);

  const attemptUpnpForwarding = useCallback(async () => {
    setIsAttemptingUpnp(true);
    try {
      setUpnpStatus({
        success: false,
        tcpForwarded: false,
        udpForwarded: false,
        localIp: null,
        externalIp: null,
        error: "Typed UPnP resolver is not available yet.",
      });
      addToast({
        title: "UPnP test unavailable",
        description: `UPnP test for port ${listenPort} requires a typed backend resolver.`,
        color: "warning",
      });
    } finally {
      setIsAttemptingUpnp(false);
    }
  }, [listenPort]);

  useEffect(() => {
    fetchUpnpStatus();
  }, [fetchUpnpStatus]);

  const handleSave = async () => {
    setIsSaving(true);
    const pairs: [keyof typeof TORRENT_KEYS, string][] = [
      ["download_dir", downloadDir],
      ["session_dir", sessionDir],
      ["enable_dht", enableDht ? "true" : "false"],
      ["listen_port", String(listenPort)],
      ["max_concurrent", String(maxConcurrent)],
      ["upload_limit", String(uploadLimit)],
      ["download_limit", String(downloadLimit)],
    ];

    try {
      for (const [key, value] of pairs) {
        const settingKey = TORRENT_KEYS[key];
        const id = settingIds.get(settingKey);
        if (id) {
          const { data } = await updateAppSetting({
            variables: {
              Id: id,
              Input: { Value: value },
            },
          });
          if (!data?.UpdateAppSetting?.Success) {
            addToast({
              title: "Error",
              description: sanitizeError(
                data?.UpdateAppSetting?.Error ?? "Failed to save setting",
              ),
              color: "danger",
            });
            return;
          }
        } else {
          const { data } = await createAppSetting({
            variables: {
              Input: {
                Key: settingKey,
                Value: value,
                Category: TORRENT_CATEGORY,
              },
            },
          });
          if (!data?.CreateAppSetting?.Success) {
            addToast({
              title: "Error",
              description: sanitizeError(
                data?.CreateAppSetting?.Error ?? "Failed to create setting",
              ),
              color: "danger",
            });
            return;
          }
        }
      }
      addToast({
        title: "Settings Saved",
        description: "Restart the server for changes to take effect.",
        color: "success",
      });
      const refreshed = await refetchSettings();
      if (refreshed.data?.AppSettings?.Edges) {
        const settings = appSettingsToTorrentSettings(
          refreshed.data.AppSettings.Edges,
        );
        setOriginalSettings(settings);
        setSettingIds(keyToIdMap(refreshed.data.AppSettings.Edges));
        setDownloadDir(settings.downloadDir);
        setSessionDir(settings.sessionDir);
        setEnableDht(settings.enableDht);
        setListenPort(settings.listenPort);
        setMaxConcurrent(settings.maxConcurrent);
        setUploadLimit(settings.uploadLimit);
        setDownloadLimit(settings.downloadLimit);
      }
    } catch (e) {
      addToast({
        title: "Error",
        description: sanitizeError(e),
        color: "danger",
      });
    } finally {
      setIsSaving(false);
    }
  };

  const handleReset = useCallback(() => {
    if (originalSettings) {
      setDownloadDir(originalSettings.downloadDir);
      setSessionDir(originalSettings.sessionDir);
      setEnableDht(originalSettings.enableDht);
      setListenPort(originalSettings.listenPort);
      setMaxConcurrent(originalSettings.maxConcurrent);
      setUploadLimit(originalSettings.uploadLimit);
      setDownloadLimit(originalSettings.downloadLimit);
    }
  }, [originalSettings]);

  const formatSpeed = (bytesPerSec: number) => {
    if (bytesPerSec === 0) return "Unlimited";
    if (bytesPerSec >= 1024 * 1024)
      return `${(bytesPerSec / (1024 * 1024)).toFixed(1)} MB/s`;
    if (bytesPerSec >= 1024) return `${(bytesPerSec / 1024).toFixed(1)} KB/s`;
    return `${bytesPerSec} B/s`;
  };

  return (
    <div
      className="grow overflow-y-auto overflow-x-hidden pb-8"
      style={{ scrollbarGutter: "stable" }}
    >
      <SettingsHeader
        title="Torrent Client"
        subtitle="Configure the built-in torrent downloader"
        onSave={handleSave}
        onReset={handleReset}
        isSaveDisabled={!hasChanges || isLoading}
        isResetDisabled={!hasChanges}
        isSaving={isSaving}
        hasChanges={hasChanges}
      />

      <Accordion selectionMode="multiple" variant="splitted">
        <AccordionItem
          key="directories"
          aria-label="Directories"
          title={
            <div className="flex items-center gap-2">
              <IconFolder size={18} className="text-amber-400" />
              <span className="font-semibold">Directories</span>
            </div>
          }
          subtitle="Download and session storage locations"
        >
          <div className="space-y-4 pb-2">
            <FolderBrowserInput
              label="Download Directory"
              value={downloadDir}
              onChange={setDownloadDir}
              placeholder="/data/downloads"
              description="Where downloaded files are saved. Make sure this path is writable."
              modalTitle="Select Download Directory"
              isDisabled={isLoading}
            />
            <FolderBrowserInput
              label="Session Directory"
              value={sessionDir}
              onChange={setSessionDir}
              placeholder="/data/session"
              description="Where torrent session data (resume info, DHT cache) is stored."
              modalTitle="Select Session Directory"
              isDisabled={isLoading}
            />
          </div>
        </AccordionItem>

        <AccordionItem
          key="network"
          aria-label="Network"
          title={
            <div className="flex items-center gap-2">
              <IconNetwork size={18} className="text-blue-400" />
              <span className="font-semibold">Network</span>
            </div>
          }
          subtitle="DHT, ports, and connection settings"
        >
          <div className="space-y-4 pb-2">
            <div className="flex justify-between items-center">
              <div>
                <p className="font-medium">Enable DHT</p>
                <p className="text-xs text-default-400">
                  Distributed Hash Table for finding peers without trackers
                </p>
              </div>
              <Switch
                isSelected={enableDht}
                onValueChange={setEnableDht}
                isDisabled={isLoading}
              />
            </div>

            <Input
              type="number"
              label="Listen Port"
              labelPlacement="inside"
              variant="flat"
              description="Port for incoming connections. Set to 0 for random port."
              value={listenPort.toString()}
              onChange={(e) => setListenPort(parseInt(e.target.value) || 0)}
              placeholder="6881"
              className="max-w-xs"
              isDisabled={isLoading}
              classNames={{ label: "text-sm font-medium text-primary!" }}
            />

            <Input
              type="number"
              label="Max Concurrent Downloads"
              labelPlacement="inside"
              variant="flat"
              description="Maximum number of torrents downloading simultaneously"
              value={maxConcurrent.toString()}
              onChange={(e) => setMaxConcurrent(parseInt(e.target.value) || 1)}
              placeholder="5"
              className="max-w-xs"
              min={1}
              max={20}
              isDisabled={isLoading}
              classNames={{ label: "text-sm font-medium text-primary!" }}
            />

            <div className="space-y-3 p-4 bg-content2 rounded-lg">
              <div className="flex items-center justify-between">
                <div>
                  <p className="font-medium text-foreground">
                    UPnP Port Forwarding
                  </p>
                  <p className="text-xs text-default-500">
                    Automatic port forwarding via UPnP for better connectivity
                  </p>
                </div>
                <div className="flex gap-2">
                  {upnpStatus && (
                    <div className="flex items-center gap-1 text-xs">
                      {upnpStatus.tcpForwarded || upnpStatus.udpForwarded ? (
                        <>
                          <IconCheck size={14} className="text-green-400" />
                          <span className="text-success">Active</span>
                        </>
                      ) : (
                        <>
                          <IconX size={14} className="text-red-400" />
                          <span className="text-danger">Failed</span>
                        </>
                      )}
                    </div>
                  )}
                  <Button
                    size="sm"
                    variant="flat"
                    color="primary"
                    isLoading={isAttemptingUpnp}
                    onPress={attemptUpnpForwarding}
                    startContent={<IconTestPipe size={14} />}
                  >
                    Test UPnP
                  </Button>
                </div>
              </div>

              {upnpStatus && !upnpStatus.success && !isAttemptingUpnp && (
                <div className="flex items-start gap-3 p-3 bg-warning-50 border border-warning-200 rounded-md">
                  <IconAlertTriangle
                    size={16}
                    className="text-warning mt-0.5 shrink-0"
                  />
                  <div className="flex-1">
                    <p className="text-sm font-medium text-warning-800">
                      Port forwarding required
                    </p>
                    <p className="text-xs text-warning-700 mt-1">
                      UPnP port forwarding failed. To improve torrent
                      performance, create a port forwarding rule in your router
                      for port {listenPort} (TCP and UDP) to your backend's
                      local IP address.
                    </p>
                  </div>
                </div>
              )}

              <div className="flex items-center justify-between pt-2 border-t border-default-200">
                <div>
                  <p className="text-sm font-medium text-foreground">
                    Port Accessibility
                  </p>
                  <p className="text-xs text-default-500">
                    Test if port {listenPort} is accessible from the internet
                  </p>
                </div>
                <Button
                  size="sm"
                  variant="flat"
                  color="secondary"
                  isLoading={isTestingPort}
                  onPress={testPortAccessibility}
                  startContent={<IconTestPipe size={14} />}
                >
                  Test Port
                </Button>
              </div>

              {portTestResult && (
                <div
                  className={`flex items-center gap-2 p-2 rounded text-xs ${
                    portTestResult.portOpen
                      ? "bg-success-50 text-success-700"
                      : "bg-danger-50 text-danger-700"
                  }`}
                >
                  {portTestResult.portOpen ? (
                    <IconCheck size={14} />
                  ) : (
                    <IconX size={14} />
                  )}
                  <span>
                    Port {listenPort} is{" "}
                    {portTestResult.portOpen ? "accessible" : "not accessible"}{" "}
                    from the internet
                    {portTestResult.externalIp && (
                      <span className="ml-1 text-default-500">
                        (External IP: {portTestResult.externalIp})
                      </span>
                    )}
                  </span>
                </div>
              )}
            </div>
          </div>
        </AccordionItem>

        <AccordionItem
          key="limits"
          aria-label="Speed Limits"
          title={
            <div className="flex items-center gap-2">
              <IconGauge size={18} className="text-green-400" />
              <span className="font-semibold">Speed Limits</span>
            </div>
          }
          subtitle="Bandwidth throttling for uploads and downloads"
        >
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4 pb-2">
            <Input
              type="number"
              label={`Download Limit: ${formatSpeed(downloadLimit)}`}
              labelPlacement="inside"
              variant="flat"
              description="0 = unlimited"
              value={downloadLimit.toString()}
              onChange={(e) => setDownloadLimit(parseInt(e.target.value) || 0)}
              placeholder="0"
              endContent={<span className="text-default-400 text-sm">B/s</span>}
              isDisabled={isLoading}
              classNames={{ label: "text-sm font-medium text-primary!" }}
            />
            <Input
              type="number"
              label={`Upload Limit: ${formatSpeed(uploadLimit)}`}
              labelPlacement="inside"
              variant="flat"
              description="0 = unlimited"
              value={uploadLimit.toString()}
              onChange={(e) => setUploadLimit(parseInt(e.target.value) || 0)}
              placeholder="0"
              endContent={<span className="text-default-400 text-sm">B/s</span>}
              isDisabled={isLoading}
              classNames={{ label: "text-sm font-medium text-primary!" }}
            />
          </div>
        </AccordionItem>
      </Accordion>
    </div>
  );
}
