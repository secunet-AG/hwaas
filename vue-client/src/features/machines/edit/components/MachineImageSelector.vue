<script setup lang="ts">
import { Listbox, ListboxButton, ListboxOptions, ListboxOption } from '@headlessui/vue'
import { useImagesStore } from '@/core/stores/images-store'
import { ref } from 'vue'
import type { LocalImage } from '@/shared/types/images.model'
import FadeInOut from '@/shared/ui/transitions/FadeInOut.vue'
import type { LocalMachine } from '@/shared/types/contexts.model'

const props = defineProps<{
  activeMachine: LocalMachine
}>()

const imageStore = useImagesStore()
const selectedImage = ref<LocalImage | null>(props.activeMachine.activeImage ?? null)

const emit = defineEmits<{
  (event: 'onImageSelection', payload: LocalImage): void
}>()
</script>

<template>
  <div class="border-(--app-secondary-border) relative rounded-md border-[1px] p-3">
    <Listbox
      v-model="selectedImage"
      @update:model-value="() => selectedImage && emit('onImageSelection', selectedImage)"
    >
      <ListboxButton class="flex w-full items-center justify-between overflow-clip">
        <div class="flex items-center gap-3">
          <h6 class="text-left">{{ selectedImage?.file_name }}</h6>
          <div class="flex gap-1">
            <span class="text-(--app-secondary-text) text-left text-sm">
              {{ selectedImage?.image_hash }}
            </span>
          </div>
        </div>
        <button class="transition duration-300 hover:-translate-y-0.5 hover:brightness-200">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            fill="none"
            viewBox="0 0 24 24"
            stroke-width="1.0"
            stroke="currentColor"
            class="text-(--app-primary-text) h-6 w-6"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              d="M8.25 15 12 18.75 15.75 15m-7.5-6L12 5.25 15.75 9"
            />
          </svg>
        </button>
      </ListboxButton>

      <FadeInOut>
        <ListboxOptions
          class="bg-(--app-bg) border-(--app-secondary-border) app-primary-shadow absolute left-0 top-12 mt-3 flex w-full flex-col rounded-md border-2"
        >
          <ListboxOption
            class="border-(--app-secondary-border) text-(--app-secondary-text) hover:text-(--app-primary-text) flex h-[42px] cursor-pointer items-center overflow-hidden border-b-[1px] pl-3 transition-all duration-300"
            v-for="image in imageStore.images"
            :key="image.image_hash"
            :value="image"
          >
            <div class="jgap-3 flex w-full grow-0 flex-nowrap items-center gap-3 text-nowrap pr-3">
              <span class="text-(--app-primary-text)"> {{ image.file_name }}</span
              ><span class="text-ellipsis text-nowrap text-sm">
                {{ image.image_hash }}
              </span>
            </div>
          </ListboxOption>
        </ListboxOptions>
      </FadeInOut>
    </Listbox>
  </div>
</template>
