import { useEffect, useState } from "react";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { useForm, Controller, type Resolver } from "react-hook-form";
import { Input } from "@heroui/input";
import { Select, SelectItem } from "@heroui/select";
import { Switch } from "@heroui/switch";
import { Divider } from "@heroui/divider";
import { Button } from "@heroui/button";
import { Accordion, AccordionItem } from "@heroui/accordion";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
  useDisclosure,
} from "@heroui/modal";
import { FolderBrowserInput } from "../FolderBrowserInput";
import { NamingPatternSelector } from "./NamingPatternSelector";
import { addToast } from "@heroui/toast";

import {
  LIBRARY_TYPES,
  configureNetworkPath,
  type LibraryType,
} from "../../lib/graphql";
import { IconFolder, IconRefresh, IconSettings } from "@tabler/icons-react";

// =============================================================================
// Schema & Types
// =============================================================================

const LIBRARY_TYPE_VALUES = [
  "MOVIES",
  "TV",
  "MUSIC",
  "AUDIOBOOKS",
  "OTHER",
] as const;

const librarySettingsSchema = z.object({
  Name: z.string().min(1, "Name is required"),
  Path: z.string().min(1, "Path is required"),
  LibraryType: z.enum(LIBRARY_TYPE_VALUES),
  AutoScan: z.boolean(),
  ScanIntervalMinutes: z.number().min(5).max(1440),
  WatchForChanges: z.boolean(),
  AutoOrganize: z.boolean(),
  NamingPattern: z.string().nullable(),
  NetworkAuthEnabled: z.boolean(),
  NetworkUsername: z.string(),
  NetworkPassword: z.string(),
  NetworkMountPoint: z.string(),
  PersistNetworkCredentials: z.boolean(),
});

export type LibrarySettingsFormValues = z.infer<typeof librarySettingsSchema>;

export const DEFAULT_LIBRARY_SETTINGS: LibrarySettingsFormValues = {
  Name: "",
  Path: "",
  LibraryType: "TV",
  AutoScan: true,
  ScanIntervalMinutes: 60,
  WatchForChanges: false,
  AutoOrganize: true,
  NamingPattern: null,
  NetworkAuthEnabled: false,
  NetworkUsername: "",
  NetworkPassword: "",
  NetworkMountPoint: "/mnt",
  PersistNetworkCredentials: true,
};

// =============================================================================
// Props
// =============================================================================

export interface LibrarySettingsFormProps {
  initialValues?: Partial<LibrarySettingsFormValues>;
  onChange: (values: LibrarySettingsFormValues, isValid: boolean) => void;
  mode: "create" | "edit";
  useCards?: boolean;
  closeSignal?: number;
  runtimePlatform?: string;
  supportsUncCredentials?: boolean;
  supportsSambaMount?: boolean;
  defaultLinuxMountBase?: string | null;
}

// =============================================================================
// Helper Components
// =============================================================================

interface SettingRowProps {
  label: string;
  description: string;
  children: React.ReactNode;
}

function SettingRow({ label, description, children }: SettingRowProps) {
  return (
    <div className="flex items-center justify-between">
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-xs text-default-500">{description}</p>
      </div>
      {children}
    </div>
  );
}

// =============================================================================
// Main Component
// =============================================================================

export function LibrarySettingsForm({
  initialValues,
  onChange,
  mode,
  useCards = false,
  closeSignal,
  runtimePlatform,
  supportsUncCredentials = false,
  supportsSambaMount = false,
  defaultLinuxMountBase,
}: LibrarySettingsFormProps) {
  const {
    control,
    watch,
    setValue,
    formState: { errors, isValid },
  } = useForm<LibrarySettingsFormValues>({
    resolver: zodResolver(
      librarySettingsSchema as unknown as Parameters<typeof zodResolver>[0],
    ) as unknown as Resolver<LibrarySettingsFormValues>,
    defaultValues: { ...DEFAULT_LIBRARY_SETTINGS, ...initialValues },
    mode: "onChange",
  });
  const sambaModal = useDisclosure();
  const [isMountingSamba, setIsMountingSamba] = useState(false);
  const [navigateFolderBrowserSignal, setNavigateFolderBrowserSignal] =
    useState<number | undefined>(undefined);

  // Watch all values and notify parent on change
  const formValues = watch();

  useEffect(() => {
    onChange(formValues, isValid);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    formValues.Name,
    formValues.Path,
    formValues.LibraryType,
    formValues.AutoScan,
    formValues.ScanIntervalMinutes,
    formValues.WatchForChanges,
    formValues.AutoOrganize,
    formValues.NamingPattern,
    formValues.NetworkAuthEnabled,
    formValues.NetworkUsername,
    formValues.NetworkPassword,
    formValues.NetworkMountPoint,
    formValues.PersistNetworkCredentials,
    isValid,
    onChange,
  ]);

  // Current library type for conditional rendering
  const libraryType = formValues.LibraryType;

  // ==========================================================================
  // Section Renderers
  // ==========================================================================

  const renderGeneralSection = () => (
    <>
      <Controller
        name="Name"
        control={control}
        render={({ field }) => (
          <Input
            label="Library Name"
            labelPlacement="inside"
            variant="flat"
            placeholder="e.g., Movies, TV Shows"
            value={field.value}
            onChange={field.onChange}
            onBlur={field.onBlur}
            isInvalid={!!errors.Name}
            errorMessage={errors.Name?.message}
            classNames={{
              label: "text-sm font-medium text-primary!",
            }}
          />
        )}
      />

      <Controller
        name="Path"
        control={control}
        render={({ field }) => (
          <FolderBrowserInput
            label={
              mode === "create" && runtimePlatform === "windows"
                ? "Path or UNC Path"
                : "Path"
            }
            value={field.value}
            onChange={field.onChange}
            placeholder="/data/media/TV"
            description={
              errors.Path?.message
                ? String(errors.Path.message)
                : "Full path to the media folder"
            }
            modalTitle="Select Library Folder"
            modalInlineAction={
              mode === "create" &&
              runtimePlatform === "linux" &&
              supportsSambaMount ? (
                <Button size="sm" variant="flat" onPress={sambaModal.onOpen}>
                  Mount Samba Path
                </Button>
              ) : undefined
            }
            navigateToValueSignal={navigateFolderBrowserSignal}
          />
        )}
      />

      {mode === "create" && runtimePlatform === "windows" && (
        <>
          <Controller
            name="NetworkAuthEnabled"
            control={control}
            render={({ field }) => (
              <SettingRow
                label="Supply username/password"
                description="Optional for UNC paths"
              >
                <Switch
                  aria-label="Supply username/password"
                  isSelected={field.value}
                  onValueChange={field.onChange}
                />
              </SettingRow>
            )}
          />
          {formValues.NetworkAuthEnabled && supportsUncCredentials && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <Controller
                name="NetworkUsername"
                control={control}
                render={({ field }) => (
                  <Input
                    label="UNC Username"
                    labelPlacement="inside"
                    variant="flat"
                    value={field.value}
                    onChange={field.onChange}
                    classNames={{ label: "text-sm font-medium text-primary!" }}
                  />
                )}
              />
              <Controller
                name="NetworkPassword"
                control={control}
                render={({ field }) => (
                  <Input
                    type="password"
                    label="UNC Password"
                    labelPlacement="inside"
                    variant="flat"
                    value={field.value}
                    onChange={field.onChange}
                    classNames={{ label: "text-sm font-medium text-primary!" }}
                  />
                )}
              />
            </div>
          )}
        </>
      )}

      <Controller
        name="LibraryType"
        control={control}
        render={({ field }) => (
          <Select
            label="Library Type"
            aria-label="Library Type"
            selectedKeys={[field.value]}
            onChange={(e) => field.onChange(e.target.value as LibraryType)}
            isDisabled={mode === "edit"}
            description={
              mode === "edit"
                ? "Library type cannot be changed after creation"
                : undefined
            }
          >
            {LIBRARY_TYPES.map((type) => (
              <SelectItem key={type.value} textValue={type.label}>
                <div className="flex items-center gap-2">
                  <type.Icon className="w-4 h-4" />
                  {type.label}
                </div>
              </SelectItem>
            ))}
          </Select>
        )}
      />

      <Modal isOpen={sambaModal.isOpen} onClose={sambaModal.onClose}>
        <ModalContent>
          <ModalHeader>Mount Samba Path</ModalHeader>
          <ModalBody className="space-y-3">
            <Controller
              name="Path"
              control={control}
              render={({ field }) => (
                <Input
                  label="UNC Path"
                  labelPlacement="inside"
                  variant="flat"
                  value={field.value}
                  onChange={field.onChange}
                  placeholder="//server/share"
                />
              )}
            />
            <Controller
              name="NetworkMountPoint"
              control={control}
              render={({ field }) => (
                <Input
                  label="Mount Point"
                  labelPlacement="inside"
                  variant="flat"
                  value={field.value}
                  onChange={field.onChange}
                  placeholder={defaultLinuxMountBase || "/mnt"}
                />
              )}
            />
            <Controller
              name="NetworkUsername"
              control={control}
              render={({ field }) => (
                <Input
                  label="Username"
                  labelPlacement="inside"
                  variant="flat"
                  value={field.value}
                  onChange={field.onChange}
                />
              )}
            />
            <Controller
              name="NetworkPassword"
              control={control}
              render={({ field }) => (
                <Input
                  type="password"
                  label="Password"
                  labelPlacement="inside"
                  variant="flat"
                  value={field.value}
                  onChange={field.onChange}
                />
              )}
            />
            <Controller
              name="PersistNetworkCredentials"
              control={control}
              render={({ field }) => (
                <SettingRow
                  label="Reconnect on startup"
                  description="Save credentials and reconnect when backend starts"
                >
                  <Switch
                    aria-label="Reconnect on startup"
                    isSelected={field.value}
                    onValueChange={field.onChange}
                  />
                </SettingRow>
              )}
            />
          </ModalBody>
          <ModalFooter>
            <Button variant="flat" onPress={sambaModal.onClose}>
              Close
            </Button>
            <Button
              color="primary"
              onPress={async () => {
                setIsMountingSamba(true);
                try {
                  const configured = await configureNetworkPath({
                    path: formValues.Path,
                    username: formValues.NetworkUsername || undefined,
                    password: formValues.NetworkPassword || undefined,
                    mountPoint: formValues.NetworkMountPoint || undefined,
                    persist: formValues.PersistNetworkCredentials,
                    attemptConnect: true,
                  });

                  if (!configured.success) {
                    addToast({
                      title: "Mount Failed",
                      description:
                        configured.error ??
                        "Failed to create samba mount point",
                      color: "danger",
                    });
                    return;
                  }

                  setValue("Path", configured.resolvedPath, {
                    shouldDirty: true,
                    shouldValidate: true,
                  });
                  setNavigateFolderBrowserSignal((v) => (v ?? 0) + 1);
                  addToast({
                    title: "Mount Created",
                    description:
                      configured.message ??
                      `Using mount path: ${configured.resolvedPath}`,
                    color: "success",
                  });
                  sambaModal.onClose();
                } finally {
                  setIsMountingSamba(false);
                }
              }}
              isLoading={isMountingSamba}
            >
              Create Mount Point
            </Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </>
  );

  const renderScanningSection = () => (
    <>
      <Controller
        name="AutoScan"
        control={control}
        render={({ field }) => (
          <SettingRow
            label="Auto-scan"
            description="Automatically scan for new files periodically"
          >
            <Switch
              aria-label="Auto-scan"
              isSelected={field.value}
              onValueChange={field.onChange}
            />
          </SettingRow>
        )}
      />

      {formValues.AutoScan && (
        <Controller
          name="ScanIntervalMinutes"
          control={control}
          render={({ field }) => (
            <Input
              type="number"
              label="Scan Interval (minutes)"
              labelPlacement="inside"
              variant="flat"
              placeholder="60"
              description="How often to scan for new files (5-1440 minutes)"
              value={field.value.toString()}
              onChange={(e) => field.onChange(parseInt(e.target.value) || 60)}
              min={5}
              max={1440}
              isInvalid={!!errors.ScanIntervalMinutes}
              errorMessage={errors.ScanIntervalMinutes?.message}
              classNames={{
                label: "text-sm font-medium text-primary!",
              }}
            />
          )}
        />
      )}

      <Divider />

      <Controller
        name="WatchForChanges"
        control={control}
        render={({ field }) => (
          <SettingRow
            label="Watch for changes"
            description="Use filesystem notifications for instant detection"
          >
            <Switch
              aria-label="Watch for changes"
              isSelected={field.value}
              onValueChange={field.onChange}
            />
          </SettingRow>
        )}
      />
    </>
  );

  const renderOrganizationSection = () => {
    const organizeDescriptions: Record<
      LibrarySettingsFormValues["LibraryType"],
      string
    > = {
      TV: "Organize downloaded files into show/season folders",
      MOVIES: "Organize downloaded files into movie folders",
      MUSIC: "Organize downloaded files into artist/album folders",
      AUDIOBOOKS: "Organize downloaded files into author/book folders",
      OTHER: "Organize downloaded files into folders",
    };
    const organizeDescription = organizeDescriptions[libraryType];

    return (
      <>
        <Controller
          name="AutoOrganize"
          control={control}
          render={({ field }) => (
            <SettingRow
              label="Organize files"
              description={organizeDescription}
            >
              <Switch
                aria-label="Organize files"
                isSelected={field.value}
                onValueChange={field.onChange}
              />
            </SettingRow>
          )}
        />

        {formValues.AutoOrganize && (
          <Controller
            name="NamingPattern"
            control={control}
            render={({ field }) => (
              <NamingPatternSelector
                value={field.value}
                onChange={field.onChange}
                libraryType={libraryType.toLowerCase()}
                closeSignal={closeSignal}
                autoSelectDefaultForLibraryType={mode === "create"}
              />
            )}
          />
        )}
      </>
    );
  };

  // ==========================================================================
  // Render with Accordions (for settings page)
  // ==========================================================================

  if (useCards) {
    const accordionItems = [
      <AccordionItem
        key="general"
        aria-label="General"
        title={
          <div className="flex items-center gap-2">
            <IconFolder size={18} className="text-amber-400" />
            <span className="font-semibold">General</span>
          </div>
        }
        subtitle="Library name, path, and type"
      >
        <div className="space-y-4 pb-2">{renderGeneralSection()}</div>
      </AccordionItem>,

      <AccordionItem
        key="scanning"
        aria-label="Scanning"
        title={
          <div className="flex items-center gap-2">
            <IconRefresh size={18} className="text-blue-400" />
            <span className="font-semibold">Scanning</span>
          </div>
        }
        subtitle="How the library detects new files"
      >
        <div className="space-y-4 pb-2">{renderScanningSection()}</div>
      </AccordionItem>,

      <AccordionItem
        key="organization"
        aria-label="Organization"
        title={
          <div className="flex items-center gap-2">
            <IconSettings size={18} className="text-secondary" />
            <span className="font-semibold">Organization</span>
          </div>
        }
        subtitle="File organization and naming"
      >
        <div className="space-y-4 pb-2">{renderOrganizationSection()}</div>
      </AccordionItem>,
    ];

    return (
      <Accordion selectionMode="multiple" variant="splitted">
        {accordionItems}
      </Accordion>
    );
  }

  // ==========================================================================
  // Render Flat (for modals)
  // ==========================================================================

  return (
    <div className="space-y-4">
      {renderGeneralSection()}

      <Divider />

      {renderScanningSection()}

      <Divider />

      {renderOrganizationSection()}
    </div>
  );
}
