<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import {
  deserializeContextConfig,
  type ContextConfig,
} from '@/shared/lib/validators/context-validation'
import type { ContextConfigurationMachine } from '@/shared/types/contexts.model'
import type { FileUploadState } from '@/shared/ui/fileupload/file-upload.model'
import FileUpload from '@/shared/ui/fileupload/FileUpload.vue'
import { ref } from 'vue'

const props = defineProps<{
  disabled: boolean
}>()

const emit = defineEmits<{
  (event: 'contextUpload', payload: ContextConfigurationMachine): void
  (event: 'contextUploadFail'): void
}>()

const uploadContextState = ref<FileUploadState>(props.disabled ? 'disabled' : 'ready')

function parseContext(config: File): Promise<ContextConfig | null> {
  return new Promise((resolve, reject) => {
    if (!config) {
      reject(null)
      return
    }
    const reader = new FileReader()
    reader.onload = () => {
      if (typeof reader.result !== 'string') return resolve(null)
      try {
        const parsed = deserializeContextConfig(reader.result)
        resolve(parsed)
      } catch {
        reject(null)
      }
    }
    reader.readAsText(config)
  })
}

function onContextUploadReset(newValue: FileUploadState) {
  uploadContextState.value = newValue
}

async function onFileUpload(file: File) {
  try {
    const config = await parseContext(file)
    if (!config) {
      uploadContextState.value = 'failed'
      throw new Error('Parsing returned null')
    }
    uploadContextState.value = 'success'
    emit('contextUpload', config.machines)
  } catch {
    uploadContextState.value = 'failed'
    emit('contextUploadFail')
  }
}
</script>

<template>
  <div>
    <FileUpload
      @file-upload-event="onFileUpload"
      :title="'Upload Context'"
      :file-type-warning="'Must be a valid .JSON file'"
      :maximumSizeWarning="'Maximum Context Size 8GB'"
      v-model="uploadContextState"
    ></FileUpload>
  </div>
</template>
