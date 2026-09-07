import { useState, useEffect } from "react";
import { Link } from "@tanstack/react-router";
import { Modal, ModalContent, ModalHeader, ModalBody, ModalFooter } from "@heroui/modal";
import { Button } from "@heroui/button";
import { Select, SelectItem } from "@heroui/select";
import { Switch } from "@heroui/switch";
import { QualityProfileSelector } from "../library/QualityProfileSelector";
import { AutoDownloadMode, type ShowDetailRouteQuery } from "../../lib/graphql/generated/graphql";

type ShowSettingsView = Pick<NonNullable<ShowDetailRouteQuery["show"]>,
  "name" | "autoDownload" | "autoDownloadMode" | "qualityProfileId">;

export interface ShowSettingsInput {
  autoDownload: boolean;
  autoDownloadMode: AutoDownloadMode;
  qualityProfileId: string | null;
}

export interface ShowSettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  show: ShowSettingsView | null;
  onSave: (settings: ShowSettingsInput) => Promise<void>;
  isLoading: boolean;
}

export function ShowSettingsModal({ isOpen, onClose, show, onSave, isLoading }: ShowSettingsModalProps) {
  const [autoDownload, setAutoDownload] = useState(false);
  const [autoDownloadMode, setAutoDownloadMode] = useState<AutoDownloadMode>(AutoDownloadMode.WANTED);
  const [qualityProfileId, setQualityProfileId] = useState<string | null>(null);

  useEffect(() => {
    if (isOpen && show) {
      setAutoDownload(show.autoDownload);
      setAutoDownloadMode(show.autoDownloadMode);
      setQualityProfileId(show.qualityProfileId ?? null);
    }
  }, [isOpen, show]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} size="2xl" scrollBehavior="inside" isDismissable={!isLoading}>
      <ModalContent>
        <ModalHeader>{show?.name} — Show Settings</ModalHeader>
        <ModalBody className="gap-6">
          <div className="space-y-3">
            <h4 className="text-sm font-semibold">Quality and Resolution</h4>
            <p className="text-sm text-default-500">
              Choose a profile for this show. Profiles define allowed resolutions,
              codecs, HDR, audio formats, and release sources. Inherit uses the library's profile.
            </p>
            <QualityProfileSelector value={qualityProfileId} onChange={setQualityProfileId} allowInherit isDisabled={isLoading} />
            <Button as={Link} to="/settings/quality-profiles" variant="light" size="sm">
              Manage quality profiles
            </Button>
          </div>
          <div className="space-y-4">
            <h4 className="text-sm font-semibold">Automatic Downloads</h4>
            <Switch isSelected={autoDownload} onValueChange={setAutoDownload} isDisabled={isLoading}>
              Auto Download
            </Switch>
            <Select label="Download episodes" selectedKeys={[autoDownloadMode]} isDisabled={isLoading}
              onSelectionChange={(keys) => {
                const value = Array.from(keys)[0];
                if (value === AutoDownloadMode.ALL || value === AutoDownloadMode.WANTED || value === AutoDownloadMode.NONE) setAutoDownloadMode(value);
              }}>
              <SelectItem key={AutoDownloadMode.ALL}>All missing episodes</SelectItem>
              <SelectItem key={AutoDownloadMode.WANTED}>Wanted episodes only</SelectItem>
              <SelectItem key={AutoDownloadMode.NONE}>None</SelectItem>
            </Select>
          </div>
        </ModalBody>
        <ModalFooter>
          <Button variant="flat" onPress={onClose} isDisabled={isLoading}>Cancel</Button>
          <Button color="primary" isLoading={isLoading}
            onPress={() => void onSave({ autoDownload, autoDownloadMode, qualityProfileId })}>
            Save Settings
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
}
