<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { computed, ref } from 'vue'
import { dropHandler } from '../../lib/etc/drop-handler'
import Upload from '@/shared/ui/icons/Upload.vue'
import type { FileUploadState } from './file-upload.model'

const props = defineProps<{
  title: string
  fileTypeWarning: string
  maximumSizeWarning: string
}>()

const fileUploadState = defineModel<FileUploadState>({ required: true })

const emit = defineEmits<{
  (event: 'fileUploadEvent', file: File): void
}>()

const hasUploaded = ref(false)

async function onClickFileUpload(e: Event) {
  const input = e.target as HTMLInputElement
  const filesAsArray = Array.from(input?.files || [])

  emit('fileUploadEvent', filesAsArray[0])
}

async function onDragFileUpload(e: DragEvent) {
  const files = dropHandler(e)

  const top = files.pop()

  if (!top) throw new Error('No file found')

  hasUploaded.value = true

  emit('fileUploadEvent', top)
}
</script>

<template>
  <div
    class="w-full"
    :class="fileUploadState === 'disabled' ? 'cursor-not-allowed opacity-40' : ''"
  >
    <label>{{ props.title }}</label>
    <div
      id="drop_zone"
      v-if="fileUploadState === 'ready'"
      v-on:drop="onDragFileUpload"
      v-on:dragover="(e) => e.preventDefault()"
      class="border-(--app-primary-border) mt-3 flex min-h-[196px] w-full flex-col items-center justify-center rounded-lg border-[1px] border-dashed"
    >
      <Upload />
      <h6 class="pointer-events-none mt-3 touch-none select-none">Drag and drop file to upload</h6>
      <span
        class="text-(--app-secondary-text) pointer-events-none touch-none select-none underline"
        >{{ props.fileTypeWarning }}</span
      >
    </div>
    <div
      class="border-(--app-primary-border) mt-3 flex min-h-[196px] w-full flex-col items-center justify-center rounded-lg border-[1px] border-dashed"
      v-else-if="fileUploadState === 'failed'"
    >
      <h6 class="mt-3">File Was Not Parsed Succesfully</h6>
      <button
        @click="
          () => {
            fileUploadState = 'ready'
          }
        "
        class="text-(--app-secondary-text) underline"
      >
        Click To Try Again
      </button>
    </div>
    <div
      class="border-(--app-primary-border) mt-3 flex min-h-[196px] w-full flex-col items-center justify-center rounded-lg border-[1px] border-dashed"
      v-else-if="fileUploadState === 'success'"
    >
      <h6 class="mt-3">File Upload Successful</h6>
    </div>
    <div class="mt-4 flex flex-col gap-3">
      <div class="w-full">
        <label for="files" class="app-btn-primary h-[32px!important] text-sm"
          >Click To Upload File</label
        >
        <input @change="onClickFileUpload" id="files" style="display: none" type="file" />
      </div>
      <span class="text-(--app-secondary-text) text-sm">{{ props.maximumSizeWarning }}</span>
    </div>
  </div>
</template>
