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
  // Convert library to form values
  const libraryToValues = useCallback((lib: Library): LibrarySettingsValues => ({
    name: lib.name,
    path: lib.path,
    libraryType: lib.libraryType,
    autoScan: lib.autoScan,
    scanIntervalMinutes: lib.scanIntervalMinutes,
    watchForChanges: lib.watchForChanges,
    postDownloadAction: lib.postDownloadAction,
    organizeFiles: lib.organizeFiles,
    namingPattern: lib.namingPattern,
    autoAddDiscovered: lib.autoAddDiscovered,
    autoDownload: lib.autoDownload,
    autoHunt: lib.autoHunt,
    allowedResolutions: lib.allowedResolutions || [],
    allowedVideoCodecs: lib.allowedVideoCodecs || [],
    allowedAudioFormats: lib.allowedAudioFormats || [],
    requireHdr: lib.requireHdr || false,
    allowedHdrTypes: lib.allowedHdrTypes || [],
    allowedSources: lib.allowedSources || [],
    releaseGroupBlacklist: lib.releaseGroupBlacklist || [],
    releaseGroupWhitelist: lib.releaseGroupWhitelist || [],
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
      values.postDownloadAction !== originalValues.postDownloadAction ||
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
      name: values.name,
      path: values.path,
      autoScan: values.autoScan,
      scanIntervalMinutes: values.scanIntervalMinutes,
      watchForChanges: values.watchForChanges,
      postDownloadAction: values.postDownloadAction,
      organizeFiles: values.organizeFiles,
      namingPattern: values.namingPattern || undefined,
      autoAddDiscovered: values.autoAddDiscovered,
      autoDownload: values.autoDownload,
      autoHunt: values.autoHunt,
      // Quality settings
      allowedResolutions: values.allowedResolutions,
      allowedVideoCodecs: values.allowedVideoCodecs,
      allowedAudioFormats: values.allowedAudioFormats,
      requireHdr: values.requireHdr,
      allowedHdrTypes: values.allowedHdrTypes,
      allowedSources: values.allowedSources,
      releaseGroupBlacklist: values.releaseGroupBlacklist,
      releaseGroupWhitelist: values.releaseGroupWhitelist,
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
