import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { Button } from "@heroui/button";
import { Card, CardBody } from "@heroui/card";
import { Chip } from "@heroui/chip";
import {
  Table,
  TableHeader,
  TableColumn,
  TableBody,
  TableRow,
  TableCell,
} from "@heroui/table";
import { Spinner } from "@heroui/spinner";
import { Tooltip } from "@heroui/tooltip";
import { addToast } from "@heroui/toast";
import {
  IconPlus,
  IconPencil,
  IconTrash,
  IconAdjustmentsHorizontal,
} from "@tabler/icons-react";
import { QualityProfileEditorModal } from "../../components/settings";
import {
  QUALITY_PROFILES_QUERY,
  CREATE_QUALITY_PROFILE_MUTATION,
  UPDATE_QUALITY_PROFILE_MUTATION,
  DELETE_QUALITY_PROFILE_MUTATION,
  type QualityProfileNode,
  type QualityProfileFields,
  type QualityProfilesQuery,
  type CreateQualityProfileMutation,
  type CreateQualityProfileMutationVariables,
  type UpdateQualityProfileMutation,
  type UpdateQualityProfileMutationVariables,
  type DeleteQualityProfileMutation,
  type DeleteQualityProfileMutationVariables,
} from "../../lib/graphql/qualityProfiles";
import { useQuery, useMutation } from "../../lib/graphql/client";
import { sanitizeError } from "../../lib/format";

export const Route = createFileRoute("/settings/quality-profiles")({
  component: QualityProfilesSettingsPage,
});

function summarize(list: string[]): string {
  return list.length === 0 ? "Any" : list.join(", ");
}

function QualityProfilesSettingsPage() {
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingProfile, setEditingProfile] = useState<QualityProfileNode | null>(null);

  const profilesQuery = useQuery<QualityProfilesQuery>(QUALITY_PROFILES_QUERY, {
    variables: { page: { limit: 100, offset: 0 } },
    fetchPolicy: "cache-and-network",
  });

  const [createQualityProfile, { loading: creating }] = useMutation<
    CreateQualityProfileMutation,
    CreateQualityProfileMutationVariables
  >(CREATE_QUALITY_PROFILE_MUTATION);
  const [updateQualityProfile, { loading: updating }] = useMutation<
    UpdateQualityProfileMutation,
    UpdateQualityProfileMutationVariables
  >(UPDATE_QUALITY_PROFILE_MUTATION);
  const [deleteQualityProfile] = useMutation<
    DeleteQualityProfileMutation,
    DeleteQualityProfileMutationVariables
  >(DELETE_QUALITY_PROFILE_MUTATION);

  const profiles =
    profilesQuery.data?.qualityProfiles?.edges ??
    profilesQuery.previousData?.qualityProfiles?.edges ??
    [];

  const handleCreate = () => {
    setEditingProfile(null);
    setEditorOpen(true);
  };

  const handleEdit = (profile: QualityProfileNode) => {
    setEditingProfile(profile);
    setEditorOpen(true);
  };

  const handleSave = async (fields: QualityProfileFields) => {
    try {
      if (editingProfile) {
        const result = await updateQualityProfile({
          variables: { id: editingProfile.id, input: fields },
        });
        if (!result.data?.updateQualityProfile.success) {
          throw new Error(
            result.data?.updateQualityProfile.error ?? "Failed to update profile",
          );
        }
        addToast({ title: "Quality profile updated", color: "success" });
      } else {
        const result = await createQualityProfile({ variables: { input: fields } });
        if (!result.data?.createQualityProfile.success) {
          throw new Error(
            result.data?.createQualityProfile.error ?? "Failed to create profile",
          );
        }
        addToast({ title: "Quality profile created", color: "success" });
      }
      setEditorOpen(false);
      void profilesQuery.refetch();
    } catch (error) {
      addToast({
        title: "Error",
        description: sanitizeError(error),
        color: "danger",
      });
    }
  };

  const handleDelete = async (profile: QualityProfileNode) => {
    try {
      const result = await deleteQualityProfile({ variables: { id: profile.id } });
      if (!result.data?.deleteQualityProfile.success) {
        throw new Error(result.data?.deleteQualityProfile.error ?? "Delete failed");
      }
      addToast({ title: "Quality profile deleted", color: "success" });
      void profilesQuery.refetch();
    } catch (error) {
      addToast({
        title: "Error",
        description: sanitizeError(error),
        color: "danger",
      });
    }
  };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold">Quality Profiles</h2>
          <p className="text-default-500 text-sm">
            Define resolution/codec/HDR/source/release-group rules and assign them to
            libraries, shows, movies, albums, or audiobooks.
          </p>
        </div>
        <Button
          color="primary"
          startContent={<IconPlus size={16} />}
          onPress={handleCreate}
        >
          New Profile
        </Button>
      </div>

      <Card>
        <CardBody className="p-0">
          {profilesQuery.loading && profiles.length === 0 ? (
            <div className="flex items-center justify-center py-16">
              <Spinner size="lg" />
            </div>
          ) : profiles.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-16 text-default-400 gap-2">
              <IconAdjustmentsHorizontal size={40} />
              <p className="text-sm">No quality profiles yet</p>
            </div>
          ) : (
            <Table removeWrapper aria-label="Quality profiles">
              <TableHeader>
                <TableColumn>NAME</TableColumn>
                <TableColumn>KIND</TableColumn>
                <TableColumn>RESOLUTIONS</TableColumn>
                <TableColumn>CODECS</TableColumn>
                <TableColumn>HDR</TableColumn>
                <TableColumn> </TableColumn>
              </TableHeader>
              <TableBody>
                {profiles.map(({ node }) => (
                  <TableRow key={node.id}>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{node.name}</span>
                        {node.isDefault && (
                          <Chip size="sm" variant="flat" color="primary">
                            Default
                          </Chip>
                        )}
                      </div>
                    </TableCell>
                    <TableCell>
                      <Chip size="sm" variant="flat">
                        {node.mediaKind}
                      </Chip>
                    </TableCell>
                    <TableCell className="text-default-500 text-sm">
                      {summarize(node.allowedResolutions)}
                    </TableCell>
                    <TableCell className="text-default-500 text-sm">
                      {summarize(node.allowedVideoCodecs)}
                    </TableCell>
                    <TableCell>
                      {node.requireHdr ? (
                        <Chip size="sm" variant="flat" color="warning">
                          Required
                        </Chip>
                      ) : (
                        <span className="text-default-400 text-sm">Any</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center justify-end gap-1">
                        <Tooltip content="Edit">
                          <Button
                            isIconOnly
                            size="sm"
                            variant="light"
                            onPress={() => handleEdit(node)}
                          >
                            <IconPencil size={16} />
                          </Button>
                        </Tooltip>
                        <Tooltip content="Delete">
                          <Button
                            isIconOnly
                            size="sm"
                            variant="light"
                            color="danger"
                            onPress={() => void handleDelete(node)}
                          >
                            <IconTrash size={16} className="text-red-400" />
                          </Button>
                        </Tooltip>
                      </div>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardBody>
      </Card>

      <QualityProfileEditorModal
        isOpen={editorOpen}
        onClose={() => setEditorOpen(false)}
        profile={editingProfile}
        onSave={handleSave}
        isLoading={creating || updating}
      />
    </div>
  );
}
