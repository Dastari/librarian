/**
 * Quality-profile UI aliases.
 *
 * Documents and operation types come exclusively from GraphQL Code Generator.
 * This module retains the established feature-facing names while avoiding
 * hand-authored DocumentNode ASTs.
 */
import type {
  CreateQualityProfileInput,
  QualityProfilesQuery,
} from "./generated/graphql";

export {
  ApproveQualityUpgradeDocument as APPROVE_QUALITY_UPGRADE_MUTATION,
  CreateQualityProfileDocument as CREATE_QUALITY_PROFILE_MUTATION,
  DeleteQualityProfileDocument as DELETE_QUALITY_PROFILE_MUTATION,
  QualityProfilesDocument as QUALITY_PROFILES_QUERY,
  UpdateQualityProfileDocument as UPDATE_QUALITY_PROFILE_MUTATION,
} from "./generated/graphql";

export type {
  ApproveQualityUpgradeMutation,
  ApproveQualityUpgradeMutationVariables,
  CreateQualityProfileMutation,
  CreateQualityProfileMutationVariables,
  DeleteQualityProfileMutation,
  DeleteQualityProfileMutationVariables,
  MediaKind,
  QualityProfilesQuery,
  QualityProfilesQueryVariables,
  UpdateQualityProfileMutation,
  UpdateQualityProfileMutationVariables,
} from "./generated/graphql";

export type QualityProfileNode =
  QualityProfilesQuery["qualityProfiles"]["edges"][number]["node"];

export type QualityProfileFields = CreateQualityProfileInput;
