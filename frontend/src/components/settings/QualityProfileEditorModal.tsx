import { useEffect, useState } from "react";
import {
  Modal,
  ModalContent,
  ModalHeader,
  ModalBody,
  ModalFooter,
} from "@heroui/modal";
import { Button } from "@heroui/button";
import { Input } from "@heroui/input";
import { Select, SelectItem } from "@heroui/select";
import { Switch } from "@heroui/switch";
import { Divider } from "@heroui/divider";
import {
  QualitySettingsCard,
  type QualitySettings,
  DEFAULT_QUALITY_SETTINGS,
  RESOLUTION_OPTIONS,
} from "./QualitySettingsCard";
import type {
  MediaKind,
  QualityProfileFields,
  QualityProfileNode,
} from "../../lib/graphql/qualityProfiles";

export interface QualityProfileEditorModalProps {
  isOpen: boolean;
  onClose: () => void;
  /** Null = create mode */
  profile: QualityProfileNode | null;
  onSave: (fields: QualityProfileFields) => Promise<void>;
  isLoading: boolean;
}

function profileToSettings(profile: QualityProfileNode | null): QualitySettings {
  if (!profile) return DEFAULT_QUALITY_SETTINGS;
  return {
    allowedResolutions: profile.allowedResolutions,
    allowedVideoCodecs: profile.allowedVideoCodecs,
    allowedAudioFormats: profile.allowedAudioFormats,
    requireHdr: profile.requireHdr,
    allowedHdrTypes: profile.allowedHdrTypes,
    allowedSources: profile.allowedSources,
    releaseGroupBlacklist: profile.releaseGroupBlacklist,
    releaseGroupWhitelist: profile.releaseGroupWhitelist,
  };
}

export function QualityProfileEditorModal({
  isOpen,
  onClose,
  profile,
  onSave,
  isLoading,
}: QualityProfileEditorModalProps) {
  const [name, setName] = useState("");
  const [mediaKind, setMediaKind] = useState<MediaKind>("VIDEO");
  const [settings, setSettings] = useState<QualitySettings>(DEFAULT_QUALITY_SETTINGS);
  const [cutoffResolution, setCutoffResolution] = useState<string | null>(null);
  const [upgradeUntilCutoff, setUpgradeUntilCutoff] = useState(false);
  const [isDefault, setIsDefault] = useState(false);

  useEffect(() => {
    if (isOpen) {
      setName(profile?.name ?? "");
      setMediaKind(profile?.mediaKind ?? "VIDEO");
      setSettings(profileToSettings(profile));
      setCutoffResolution(profile?.cutoffResolution ?? null);
      setUpgradeUntilCutoff(profile?.upgradeUntilCutoff ?? false);
      setIsDefault(profile?.isDefault ?? false);
    }
  }, [isOpen, profile]);

  const handleSave = async () => {
    await onSave({
      name: name.trim(),
      mediaKind,
      allowedResolutions: settings.allowedResolutions,
      allowedVideoCodecs: settings.allowedVideoCodecs,
      allowedAudioFormats: settings.allowedAudioFormats,
      requireHdr: settings.requireHdr,
      allowedHdrTypes: settings.allowedHdrTypes,
      allowedSources: settings.allowedSources,
      releaseGroupBlacklist: settings.releaseGroupBlacklist,
      releaseGroupWhitelist: settings.releaseGroupWhitelist,
      cutoffResolution,
      upgradeUntilCutoff,
      isDefault,
    });
  };

  const isValid = name.trim().length > 0;

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="3xl" scrollBehavior="inside">
      <ModalContent>
        <ModalHeader>{profile ? "Edit Quality Profile" : "New Quality Profile"}</ModalHeader>
        <ModalBody className="gap-6">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <Input
              label="Name"
              labelPlacement="outside"
              placeholder="e.g. 1080p HEVC"
              value={name}
              onChange={(e) => setName(e.target.value)}
              isRequired
              size="sm"
            />
            <Select
              label="Media Kind"
              labelPlacement="outside"
              selectedKeys={[mediaKind]}
              onSelectionChange={(keys) => {
                const value = Array.from(keys)[0] as MediaKind;
                if (value) setMediaKind(value);
              }}
              size="sm"
              description="Video profiles evaluate resolution/codec/HDR/source; audio profiles evaluate audio format only"
              isDisabled={Boolean(profile)}
            >
              <SelectItem key="VIDEO">Video (Movies/TV)</SelectItem>
              <SelectItem key="AUDIO">Audio (Music/Audiobooks)</SelectItem>
            </Select>
          </div>

          <Divider />

          <QualitySettingsCard
            settings={settings}
            onChange={setSettings}
            title="Quality Rules"
            description="Leave empty to accept any value"
            noCard
            libraryType={mediaKind === "AUDIO" ? "MUSIC" : "TV"}
          />

          <Divider />

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <Select
              label="Upgrade Cutoff"
              labelPlacement="outside"
              selectedKeys={cutoffResolution ? [cutoffResolution] : []}
              onSelectionChange={(keys) => {
                const value = Array.from(keys)[0] as string | undefined;
                setCutoffResolution(value ?? null);
              }}
              size="sm"
              description="Stop seeking upgrades once this resolution is reached"
            >
              {RESOLUTION_OPTIONS.map((opt) => (
                <SelectItem key={opt.value}>{opt.label}</SelectItem>
              ))}
            </Select>
            <div className="flex flex-col gap-2 justify-center">
              <Switch
                isSelected={upgradeUntilCutoff}
                onValueChange={setUpgradeUntilCutoff}
                size="sm"
                isDisabled={!cutoffResolution}
              >
                Seek upgrades until cutoff
              </Switch>
              <Switch isSelected={isDefault} onValueChange={setIsDefault} size="sm">
                Default profile
              </Switch>
            </div>
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose}>
            Cancel
          </Button>
          <Button
            color="primary"
            onPress={handleSave}
            isLoading={isLoading}
            isDisabled={!isValid}
          >
            Save
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}

export default QualityProfileEditorModal;
