import type { ReactNode } from 'react'
import { Button } from '@heroui/button'

export interface SettingsHeaderProps {
  /** Main title for the settings page */
  title: string
  /** Optional subtitle/description */
  subtitle?: string
  /** Whether the save button should be disabled */
  isSaveDisabled?: boolean
  /** Whether the reset button should be disabled */
  isResetDisabled?: boolean
  /** Whether save is in progress */
  isSaving?: boolean
  /** Called when save is clicked */
  onSave?: () => void
  /** Called when reset is clicked */
  onReset?: () => void
  /** Show unsaved changes indicator */
  hasChanges?: boolean
  /** Custom actions to show in the header (used instead of save/reset) */
  actions?: ReactNode
}

/**
 * Sticky header for settings pages.
 * 
 * Provides a consistent header with title, subtitle, and Save/Reset buttons
 * that stays fixed at the top while content scrolls.
 */
export function SettingsHeader({
  title,
  subtitle,
  isSaveDisabled = false,
  isResetDisabled = false,
  isSaving = false,
  onSave,
  onReset,
  hasChanges = false,
  actions,
}: SettingsHeaderProps) {
  const showSaveReset = onSave || onReset

  return (
    <div className="sticky top-0 z-10 bg-background/80 backdrop-blur-md  -mx-4 px-4 py-4 mb-4">
      <div className="flex items-center justify-between gap-4">
        <div className="min-w-0">
          <h2 className="text-xl font-semibold">{title}</h2>
          {subtitle && (
            <p className="text-default-500 text-sm truncate">{subtitle}</p>
          )}
        </div>
        
        <div className="flex items-center gap-3 shrink-0">
          {hasChanges && (
            <span className="text-sm text-warning hidden sm:inline">
              Unsaved changes
            </span>
          )}
          
          {actions}
          
          {showSaveReset && !actions && (
            <>
              {onReset && (
                <Button
                  variant="flat"
                  size="sm"
                  onPress={onReset}
                  isDisabled={isResetDisabled || isSaving}
                >
                  Reset
                </Button>
              )}
              {onSave && (
                <Button
                  size="sm"
                  color="primary"
                  onPress={onSave}
                  isDisabled={isSaveDisabled}
                  isLoading={isSaving}
                >
                  Save
                </Button>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  )
}
