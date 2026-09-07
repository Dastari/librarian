import { useEffect, useState } from "react";
import { Select, SelectItem } from "@heroui/select";
import { Spinner } from "@heroui/spinner";
import { IconAdjustmentsHorizontal } from "@tabler/icons-react";
import {
  QUALITY_PROFILES_QUERY,
  type QualityProfileNode,
  type QualityProfilesQuery,
} from "../../lib/graphql/qualityProfiles";
import { apolloClient } from "../../lib/graphql/client";

export interface QualityProfileSelectorProps {
  /** Selected profile id, or null to inherit from the parent (library/default). */
  value: string | null;
  onChange: (profileId: string | null) => void;
  label?: string;
  /** When true, shows an "Inherit" option that resolves to `null`. */
  allowInherit?: boolean;
  isDisabled?: boolean;
}

const INHERIT_KEY = "__inherit__";

/**
 * A selector for `QualityProfile` records, used for `Library.qualityProfileId`
 * (primary) and the per-entity override fields on Show/Movie/Album/Audiobook.
 * See docs/tier1-features-plan.md §2.
 */
export function QualityProfileSelector({
  value,
  onChange,
  label = "Quality Profile",
  allowInherit = false,
  isDisabled = false,
}: QualityProfileSelectorProps) {
  const [profiles, setProfiles] = useState<QualityProfileNode[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const fetchProfiles = async () => {
      try {
        const { data } = await apolloClient.query<QualityProfilesQuery>({
          query: QUALITY_PROFILES_QUERY,
          fetchPolicy: "network-only",
          variables: { page: { limit: 200, offset: 0 } },
        });
        if (!cancelled && data?.qualityProfiles?.edges) {
          setProfiles(data.qualityProfiles.edges.map((e) => e.node));
        }
      } catch (error) {
        console.error("Failed to fetch quality profiles:", error);
      } finally {
        if (!cancelled) setIsLoading(false);
      }
    };
    void fetchProfiles();
    return () => {
      cancelled = true;
    };
  }, []);

  if (isLoading) {
    return (
      <div className="flex items-center gap-2">
        <Spinner size="sm" />
        <span className="text-sm text-default-500">Loading quality profiles...</span>
      </div>
    );
  }

  const selectedKey = value ?? (allowInherit ? INHERIT_KEY : "");
  const selectProfile = (key: string | undefined) => {
    if (key === INHERIT_KEY && allowInherit) onChange(null);
    else if (profiles.some(profile => profile.id === key)) onChange(key!);
  };

  return (
    <Select
      label={label}
      labelPlacement="outside"
      selectedKeys={selectedKey ? [selectedKey] : []}
      onSelectionChange={(keys) => {
        const selected = Array.from(keys)[0] as string | undefined;
        selectProfile(selected);
      }}
      onChange={event => selectProfile(event.target.value)}
      placeholder="Select a quality profile"
      startContent={<IconAdjustmentsHorizontal size={16} className="text-default-400" />}
      isDisabled={isDisabled}
      size="sm"
    >
      {[
        ...(allowInherit
          ? [
              <SelectItem key={INHERIT_KEY} textValue="Inherit from library">
                Inherit from library
              </SelectItem>,
            ]
          : []),
        ...profiles.map((profile) => (
          <SelectItem key={profile.id} textValue={profile.name}>
            <div className="flex items-center gap-2">
              <span>{profile.name}</span>
              {profile.isDefault && (
                <span className="text-xs text-primary">(Default)</span>
              )}
            </div>
          </SelectItem>
        )),
      ]}
    </Select>
  );
}

export default QualityProfileSelector;
