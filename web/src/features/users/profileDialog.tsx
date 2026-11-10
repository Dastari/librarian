import { useRouter, useRouterState, type AnyRouter } from "@tanstack/react-router";
import { useCallback } from "react";

import { useSession } from "@/lib/auth/useSession";

import { isUserSettingsTab, type UserSettingsTab } from "./useUserActions";
import { UserSettingsDialog } from "./UserSettingsDialog";

/** The search parameter that opens your profile on any page (`?profile=statistics`). */
export const PROFILE_PARAM = "profile";

/** Open or move your profile dialog: it lives in the URL, so Back closes it and links can open it. */
export function useOpenProfile() {
  const router = useRouter<AnyRouter>();
  return useCallback((tab: UserSettingsTab = "profile", replace = false) => {
    void router.navigate({
      to: router.state.location.pathname,
      search: ((previous: Record<string, unknown>) => ({ ...previous, [PROFILE_PARAM]: tab })) as never,
      replace,
      resetScroll: false,
    });
  }, [router]);
}

/** Your profile (the User settings dialog for your own account), wherever `?profile=` asks for it. */
export function ProfileDialogHost() {
  const router = useRouter<AnyRouter>();
  const { user } = useSession();
  const tab = useRouterState({ select: (state) => (state.location.search as Record<string, unknown>)[PROFILE_PARAM] });
  const openProfile = useOpenProfile();
  if (!user || !isUserSettingsTab(tab)) return null;
  const close = () => void router.navigate({
    to: router.state.location.pathname,
    search: ((previous: Record<string, unknown>) => ({ ...previous, [PROFILE_PARAM]: undefined })) as never,
    replace: true,
    resetScroll: false,
  });
  return <UserSettingsDialog userId={user.id} open tab={tab} onTabChange={(next) => openProfile(next, true)} onOpenChange={(open) => !open && close()} />;
}
