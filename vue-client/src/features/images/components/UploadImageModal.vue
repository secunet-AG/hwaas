<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { TransitionRoot, TransitionChild, Dialog, DialogPanel } from '@headlessui/vue'
import FileUpload from '@/shared/ui/fileupload/FileUpload.vue'
import { fileToSHA256HashHex } from '@/shared/lib/etc/hash-image'
import { useImagesStore } from '@/core/stores/images-store'
import Progress from '@/shared/ui/loading/Progress.vue'
import type { FileUploadState } from '@/shared/ui/fileupload/file-upload.model'

const imageStore = useImagesStore()

const isOpen = ref(false)
const fileRef = ref<File | null>(null)

const uploadState = ref<FileUploadState>('ready')

const progressPercentage = ref(0)

const hasCancelled = ref(false)

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
}

function openModal() {
  isOpen.value = true
}

async function onFileUpload(e: File) {
  try {
    fileRef.value = e
    uploadState.value = 'hashing'

    const hash = await fileToSHA256HashHex(e, {
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
    uploadState.value = 'success'

    closeModal()
  } catch (e) {
    console.error(e)
    uploadState.value = 'failed'
  }
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
              <FileUpload
                v-if="uploadState === 'ready'"
                @file-upload-event="onFileUpload"
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
                  Cancel
                </button>
              </div>
            </DialogPanel>
          </TransitionChild>
        </div>
      </div>
    </Dialog>
  </TransitionRoot>
</template>
