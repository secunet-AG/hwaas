<script setup lang="ts">
import { nextTick, ref, useTemplateRef } from 'vue'
import type { TooltipOptions } from './tooltip.model'
import { useDebounceFn } from '@vueuse/core'
import { useToolTip } from '@/core/plugins/tooltip-plugin'

const tooltip = useToolTip()

const props = defineProps<{
  options: TooltipOptions
}>()

const isVisible = ref(false)

const containerRef = useTemplateRef('containerRef')
const messageRef = useTemplateRef('messageRef')

const offset = ref<{ x: number; y: number }>({ x: 0, y: 36 })

const updateOffset = () => {
  const screenWidth = window.innerWidth

  const containerRects = containerRef.value?.getBoundingClientRect()

  const messageRects = messageRef.value?.getBoundingClientRect()

  if (!containerRects || !messageRects) return

  const midpointTranslationX = (-1 * messageRects.width + containerRects.width) / 2

  if (props.options.xOffsetOverride !== undefined) {
    offset.value.x = props.options.xOffsetOverride
  } else {
    offset.value.x =
      midpointTranslationX + messageRects.right <= screenWidth - 32
        ? midpointTranslationX
        : screenWidth - (messageRects.right + 32)
  }

  if (props.options.yOffsetOverride !== undefined) {
    offset.value.y = props.options.yOffsetOverride
  } else {
    offset.value.y = containerRects.height + 12
  }
}

const onMouseEnter = useDebounceFn(async () => {
  if (tooltip.isOpen.value) return
  isVisible.value = true
  await nextTick()
  updateOffset()
}, 200)

const onMouseLeave = useDebounceFn(async () => {
  isVisible.value = false
  await nextTick()
  tooltip.setIsOpen(false)
}, 200)
</script>

<template>
  <slot v-if="props.options.isDisabled"></slot>
  <div v-else @pointerenter="onMouseEnter" @pointerleave="onMouseLeave" class="relative">
    <transition
      enter-active-class="transition-opacity duration-300 ease-out"
      enter-from-class="transition-opacity opacity-0"
      enter-to-class="transition-opacity opacity-100"
      leave-active-class="transition-opacity duration-300 ease-out"
      leave-from-class="transition-opacity opacity-100"
      leave-to-class="transition-opacity opacity-0"
      @after-leave="
        () => {
          offset = { x: 0, y: 0 }
        }
      "
    >
      <div
        v-show="isVisible"
        ref="messageRef"
        :style="{ transform: `translate(${offset.x}px, ${offset.y}px)` }"
        class="app-card z-2 absolute overflow-visible"
      >
        <h6 class="text-(--app-primary-text) text-nowrap">{{ props.options.message }}</h6>
      </div>
    </transition>

    <div ref="containerRef">
      <slot></slot>
    </div>
  </div>
</template>
