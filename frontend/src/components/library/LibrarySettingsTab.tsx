import { useState, useEffect, useCallback, useMemo } from 'react'
import { addToast } from '@heroui/toast'
import { LibrarySettingsForm, type LibrarySettingsValues } from './LibrarySettingsForm'
import { SettingsHeader } from '../shared'
import type { Library, UpdateLibraryInput } from '../../lib/graphql'

interface LibrarySettingsTabProps {
  library: Library
  onSave: (input: UpdateLibraryInput) => Promise<void>
  isLoading: boolean
}

export function LibrarySettingsTab({ library, onSave, isLoading }: LibrarySettingsTabProps) {
  // Convert codegen Library to form values (only fields present on schema; rest defaulted)
  const libraryToValues = useCallback((lib: Library): LibrarySettingsValues => ({
    name: lib.Name,
    path: lib.Path,
    libraryType: lib.LibraryType as LibrarySettingsValues['libraryType'],
    autoScan: lib.AutoScan,
    scanIntervalMinutes: lib.ScanIntervalMinutes,
    watchForChanges: lib.WatchForChanges,
    organizeFiles: false,
    namingPattern: null,
    autoAddDiscovered: lib.AutoAddDiscovered,
    autoDownload: lib.AutoDownload,
    autoHunt: lib.AutoHunt,
    allowedResolutions: [],
    allowedVideoCodecs: [],
    allowedAudioFormats: [],
    requireHdr: false,
    allowedHdrTypes: [],
    allowedSources: [],
    releaseGroupBlacklist: [],
    releaseGroupWhitelist: [],
  }), [])

  const [values, setValues] = useState<LibrarySettingsValues>(() => libraryToValues(library))
  const [hasChanges, setHasChanges] = useState(false)

  // Reset form when library changes
  useEffect(() => {
    setValues(libraryToValues(library))
    setHasChanges(false)
  }, [library, libraryToValues])

  // Track changes
  const originalValues = useMemo(() => libraryToValues(library), [library, libraryToValues])

  useEffect(() => {
    const arraysEqual = (a: string[], b: string[]) => 
      a.length === b.length && a.every((v, i) => v === b[i])
    
    const changed =
      values.name !== originalValues.name ||
      values.path !== originalValues.path ||
      values.autoScan !== originalValues.autoScan ||
      values.scanIntervalMinutes !== originalValues.scanIntervalMinutes ||
      values.watchForChanges !== originalValues.watchForChanges ||
      values.organizeFiles !== originalValues.organizeFiles ||
      values.namingPattern !== originalValues.namingPattern ||
      values.autoAddDiscovered !== originalValues.autoAddDiscovered ||
      values.autoDownload !== originalValues.autoDownload ||
      values.autoHunt !== originalValues.autoHunt ||
      !arraysEqual(values.allowedResolutions, originalValues.allowedResolutions) ||
      !arraysEqual(values.allowedVideoCodecs, originalValues.allowedVideoCodecs) ||
      !arraysEqual(values.allowedAudioFormats, originalValues.allowedAudioFormats) ||
      values.requireHdr !== originalValues.requireHdr ||
      !arraysEqual(values.allowedHdrTypes, originalValues.allowedHdrTypes) ||
      !arraysEqual(values.allowedSources, originalValues.allowedSources) ||
      !arraysEqual(values.releaseGroupBlacklist, originalValues.releaseGroupBlacklist) ||
      !arraysEqual(values.releaseGroupWhitelist, originalValues.releaseGroupWhitelist)
    
    setHasChanges(changed)
  }, [values, originalValues])

  const handleChange = useCallback((newValues: LibrarySettingsValues) => {
    setValues(newValues)
  }, [])

  const handleSubmit = async () => {
    if (!values.name || !values.path) {
      addToast({
        title: 'Validation Error',
        description: 'Name and path are required',
        color: 'danger',
      })
      return
    }

    await onSave({
      Name: values.name,
      Path: values.path,
      LibraryType: values.libraryType,
      AutoScan: values.autoScan,
      ScanIntervalMinutes: values.scanIntervalMinutes,
      WatchForChanges: values.watchForChanges,
      AutoAddDiscovered: values.autoAddDiscovered,
      AutoDownload: values.autoDownload,
      AutoHunt: values.autoHunt,
    })
  }

  const handleReset = useCallback(() => {
    setValues(libraryToValues(library))
  }, [library, libraryToValues])

  return (
    <div className="grow overflow-hidden overflow-y-auto pb-8 px-4" style={{ scrollbarGutter: 'stable' }}>
      <SettingsHeader
        title="Library Settings"
        subtitle="Configure how this library behaves"
        onSave={handleSubmit}
        onReset={handleReset}
        isSaveDisabled={!hasChanges || !values.name || !values.path}
        isResetDisabled={!hasChanges}
        isSaving={isLoading}
        hasChanges={hasChanges}
      />

      <LibrarySettingsForm
        initialValues={values}
        onChange={handleChange}
        mode="edit"
        useCards={true}
        qualityMode="full"
      />
    </div>
  )
}
