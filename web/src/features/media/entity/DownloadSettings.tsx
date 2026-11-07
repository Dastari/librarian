import { useMutation } from "@apollo/client/react";
import { useBoundField } from "glass-ux/form";
import { IconSearch } from "glass-ux/icons";
import { Segmented } from "glass-ux/segmented";
import { SettingRow } from "glass-ux/setting-row";
import { toast } from "glass-ux/toast";
import { useMemo, type ReactNode } from "react";

import { Button } from "@/components/ui";
import { MODE_HELP, MODE_SEGMENTS } from "@/features/acquisition/mode";
import { EntityLibraryGetDocument, QualityProfilesListDocument, SearchMissingDocument, type AutoDownloadMode, type SearchMissingInput } from "@/graphql/generated/graphql";
import { errorMessage } from "@/lib/graphql/errors";
import { useLiveQuery as useQuery } from "@/lib/graphql/live/hooks";

/** The quality profiles a title can use. Blank follows its library, whose profile is named. */
export function useQualityProfileOptions(libraryId: string | undefined, skip = false) {
  const library = useQuery(EntityLibraryGetDocument, { variables: { id: libraryId ?? "" }, skip: skip || !libraryId });
  const profiles = useQuery(QualityProfilesListDocument, { skip });
  const libraryProfileId = library.data?.library?.qualityProfileId ?? null;
  return useMemo(() => {
    const list = profiles.data?.qualityProfiles.edges.map((edge) => edge.node) ?? [];
    const inherited = list.find((profile) => profile.id === libraryProfileId)?.name ?? list.find((profile) => profile.isDefault)?.name ?? "default";
    return [
      { value: "", label: `Inherit from library (${inherited})` },
      ...list.map((profile) => ({ value: profile.id, label: profile.name })),
    ];
  }, [profiles.data, libraryProfileId]);
}

/** How much of a show, album or book is fetched by itself. Goes inside a form's `autoDownloadMode` field. */
export function AutoDownloadField() {
  const { field } = useBoundField<AutoDownloadMode>();
  return (
    <SettingRow
      label="Automatic downloads"
      description={MODE_HELP[field.state.value]}
      control={<Segmented size="sm" aria-label="Automatic downloads" options={MODE_SEGMENTS.map(({ key, label }) => ({ value: key, label }))} value={field.state.value} onValueChange={(value) => field.handleChange(value as AutoDownloadMode)} />}
    />
  );
}

/** A Downloads tab: how a title fetches files, which quality it accepts, and a search right now. */
export function DownloadSettingsFields({ libraryId, scope, active, mode, profile }: {
  libraryId: string;
  scope: SearchMissingInput;
  active: boolean;
  mode: ReactNode;
  profile: (options: ReturnType<typeof useQualityProfileOptions>) => ReactNode;
}) {
  const options = useQualityProfileOptions(libraryId, !active);
  return (
    <div className="flex flex-col gap-3">
      {mode}
      {profile(options)}
      <SearchNowRow scope={scope} />
    </div>
  );
}

/** Search for whatever a title is still missing, now, ignoring the usual retry delay. */
export function SearchNowRow({ scope }: { scope: SearchMissingInput }) {
  const [searchMissing, { loading }] = useMutation(SearchMissingDocument);
  const search = async () => {
    try {
      const result = (await searchMissing({ variables: { input: scope } })).data?.searchMissing;
      if (result?.success) toast.success(result.searched === 0 ? "Nothing is missing here" : `Searched ${result.searched}, grabbed ${result.queued}`);
      else toast.warning(result?.error ?? "The search did not run");
    } catch (error) {
      toast.danger(errorMessage(error));
    }
  };
  return (
    <SettingRow
      label="Search now"
      description="Searches your sources for everything still missing here, ignoring the usual retry delay."
      control={<Button variant="secondary" size="sm" icon={<IconSearch />} isPending={loading} onPress={() => void search()}>Search now</Button>}
    />
  );
}
