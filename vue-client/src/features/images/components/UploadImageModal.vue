<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { TransitionRoot, TransitionChild, Dialog, DialogPanel } from '@headlessui/vue'
import FileUpload from '@/shared/ui/fileupload/FileUpload.vue'
import { fileToSHA256HashHex } from '@/shared/lib/etc/hash-image'
import { useImagesStore } from '@/core/stores/images-store'
import Progress from '@/shared/ui/loading/Progress.vue'
import type { FileUploadState } from '@/shared/ui/fileupload/file-upload.model'
import type { Architecture } from '@/shared/types/images.model'
import ImageMetadataForm from './ImageMetadataForm.vue'

const imageStore = useImagesStore()

const isOpen = ref(false)
const fileRef = ref<File | null>(null)

const uploadState = ref<FileUploadState>('ready')

const progressPercentage = ref(0)

const hasCancelled = ref(false)

const architecture = ref<Architecture | null>(null)
const tags = ref<string[]>([])
const errorMessage = ref<string | null>(null)

const knownTags = computed(() => imageStore.tags.map((x) => x.name))

onMounted(() => {
  progressPercentage.value = 0
  hasCancelled.value = false
  uploadState.value = 'ready'
})

function closeModal() {
  hasCancelled.value = true
  progressPercentage.value = 0
  isOpen.value = false
  uploadState.value = 'ready'
  fileRef.value = null
  architecture.value = null
  tags.value = []
  errorMessage.value = null
}

function openModal() {
  isOpen.value = true
}

// Selecting a file only stages it; the upload starts once metadata has been filled in.
function onFileSelected(e: File) {
  fileRef.value = e
  errorMessage.value = null
}

async function startUpload() {
  const e = fileRef.value
  if (!e) return
  hasCancelled.value = false
  errorMessage.value = null
  let hash: string | null
  try {
    uploadState.value = 'hashing'

    hash = await fileToSHA256HashHex(e, {
      updatePercentageCallback: (x) => {
        progressPercentage.value = x
      },
      checkForCancelationCallback: () => hasCancelled.value,
    })

    if (!hash) {
      console.warn('Null hash, perhaps the user cancelled hashing')
      uploadState.value = 'ready'
      hasCancelled.value = false
      return
    }

    uploadState.value = 'uploading'
    await imageStore.addImage(
      hash,
      e,
      (x) => {
        progressPercentage.value = x
      },
      () => hasCancelled.value,
    )
  } catch (e) {
    console.error(e)
    // Return to the staged file so the user can retry without re-entering metadata.
    uploadState.value = 'ready'
    errorMessage.value = `Upload failed: ${e instanceof Error ? e.message : e}`
    return
  }

  try {
    await imageStore.applyImageMetadata(hash, {
      architecture: architecture.value,
      tags: tags.value,
    })
  } catch (e) {
    console.error(e)
    // The image itself is stored, so there is nothing to retry from this dialog.
    uploadState.value = 'failed'
    errorMessage.value = `Image uploaded, but some metadata could not be saved: ${e instanceof Error ? e.message : e}`
    return
  }

  uploadState.value = 'success'
  closeModal()
}
</script>

<template>
  <div class="flex items-center justify-center">
    <button
      @click="openModal"
      class="border-(--app-primary-text) flex h-12 w-12 items-center justify-center rounded-full border-[1px]"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="currentColor"
        class="text-(--app-primary-text) size-8"
      >
        <path
          fill-rule="evenodd"
          d="M12 3.75a.75.75 0 0 1 .75.75v6.75h6.75a.75.75 0 0 1 0 1.5h-6.75v6.75a.75.75 0 0 1-1.5 0v-6.75H4.5a.75.75 0 0 1 0-1.5h6.75V4.5a.75.75 0 0 1 .75-.75Z"
          clip-rule="evenodd"
        />
      </svg>
    </button>
  </div>

  <TransitionRoot appear :show="isOpen" as="template">
    <Dialog as="div" :static="true" class="relative">
      <!-- Backdrop -->
      <div class="bg-(--app-bg)/40 fixed inset-0 backdrop-blur-lg transition-opacity" />

      <div class="fixed inset-0 overflow-y-auto">
        <div class="flex min-h-full items-center justify-center p-4 text-center">
          <TransitionChild
            as="template"
            enter="ease-out duration-300"
            enter-from="opacity-0 scale-95"
            enter-to="opacity-100 scale-100"
            leave="ease-in duration-200"
            leave-from="opacity-100 scale-100"
            leave-to="opacity-0 scale-95"
          >
            <DialogPanel
              :open="isOpen"
              class="border-(--app-secondary-border) bg-(--app-bg) w-full max-w-md transform overflow-hidden rounded-2xl border-[1px] p-6 text-left align-middle shadow-xl transition-all"
            >
              <div v-if="uploadState === 'ready' && fileRef" class="flex flex-col gap-4">
                <h3 class="text-lg">Upload Image</h3>
                <div
                  class="border-(--app-secondary-border) flex items-center justify-between gap-3 rounded-md border p-3"
                >
                  <span
                    class="min-w-0 overflow-hidden text-(--app-primary-text) text-ellipsis text-nowrap"
                    >{{ fileRef.name }}</span
                  >
                  <button
                    type="button"
                    class="text-(--app-secondary-text) hover:text-(--app-primary-text) shrink-0 cursor-pointer text-sm underline"
                    @click="fileRef = null"
                  >
                    Change file
                  </button>
                </div>
                <ImageMetadataForm
                  v-model:architecture="architecture"
                  v-model:tags="tags"
                  :known-tags="knownTags"
                />
                <p v-if="errorMessage" class="text-(--app-danger) text-sm">{{ errorMessage }}</p>
                <button type="button" class="app-btn-primary w-full" @click="startUpload">
                  Upload
                </button>
              </div>

              <div v-else-if="uploadState === 'failed'" class="flex flex-col gap-3">
                <h3 class="text-lg">Upload Image</h3>
                <p class="text-(--app-danger) text-sm">{{ errorMessage }}</p>
              </div>

              <FileUpload
                v-else-if="uploadState === 'ready'"
                @file-upload-event="onFileSelected"
                :title="'Upload Image'"
                :file-type-warning="'Must be a valid .img file'"
                :maximumSizeWarning="'Maximum Size 8GB'"
                :is-disabled="false"
                v-model="uploadState"
              />

              <div
                class="border-(--app-secondary-border) flex h-[196px] flex-col items-center justify-center gap-3 rounded-md border-[1px] border-dashed pb-6"
                v-else
              >
                <span v-if="uploadState == 'hashing'" class="text-(--app-secondary-text)"
                  >Hashing Image...</span
                >
                <span v-if="uploadState == 'uploading'" class="text-(--app-secondary-text)"
                  >Uploading Image...</span
                >
                <div class="w-full px-6">
                  <Progress v-model="progressPercentage" />
                </div>
              </div>

              <div class="mt-4 w-full">
                <button @click="closeModal" type="button" class="app-btn-secondary w-full">
                  {{ uploadState === 'failed' ? 'Close' : 'Cancel' }}
                </button>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
