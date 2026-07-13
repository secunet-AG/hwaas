<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { RouterView } from 'vue-router'
import Sidebar from './shared/ui/sidebar/Sidebar.vue'
import ErrorBoundary from './core/error-handling/ErrorBoundary.vue'
import Toaster from './shared/ui/toaster/Toaster.vue'
import { useDebounceFn, useIntervalFn } from '@vueuse/core'
import { cacheContexts, useContextStore } from './core/stores/context-store'
import { storeToRefs } from 'pinia'
import Spinner from './shared/ui/loading/Spinner.vue'
import { onMounted } from 'vue'

const store = useContextStore()
const { contexts, activeContextIndex, activeContext } = storeToRefs(store)
const MAX = +import.meta.env.VITE_MAXIMUM_CONTEXT_LIFETIME_IN_SECONDS
const INTERACTION_LIFETIME_INCREMENT = 60 * 10
const INCREMENT_LIFETIME_IF_UNDER = 60 * 10

// The function used to automatically increment context lifetimes
const incrementTime = useDebounceFn(() => {
  const ctx = store.activeContext
  if (ctx && ctx.lifetime + INTERACTION_LIFETIME_INCREMENT <= MAX) {
    store.setLifetime(ctx.id, ctx.lifetime + INTERACTION_LIFETIME_INCREMENT)
  }
}, INTERACTION_LIFETIME_INCREMENT)

// The function used to cache contexts to local storage on change. This is due to API constraints rather than performance reasons.
store.$onAction(({ name, after }) => {
  after(() => {
    writeCache({ name, after })
  })
}, true)

const writeCache = useDebounceFn(({ name, _ }) => {
  cacheContexts({ contexts: contexts.value, activeContextIndex: activeContextIndex.value })
  // Avoid recursion here
  if (name !== 'setLifetime') {
    if (!activeContext.value?.lifetime) return
    if (activeContext.value?.lifetime <= INCREMENT_LIFETIME_IF_UNDER) {
      incrementTime()
    }
  }
}, 120)

const invalidateCache = useDebounceFn(() => {
  store.invalidateContextsCache()
}, 800)

// Interval fn to automatically cache contexts
useIntervalFn(
  () => {
    invalidateCache()
  },
  1000 * 60 * 5,
  { immediateCallback: true },
)
</script>

<template>
  <ErrorBoundary>
    <!-- Spinner and toaster use stores to manage loading and error states-->
    <Spinner />
    <Toaster />
    <div
      class="grid-cols-(--app-layout-grid) bg-(--app-bg) relative grid h-full w-full overflow-hidden"
    >
      <div
        aria-hidden="true"
        class="app-gradient pointer-events-none absolute inset-0 z-[9999] touch-none"
      ></div>
      <Sidebar />
      <div class="flex h-full w-full flex-col items-center overflow-auto">
        <div class="max-w-(--app-max-content-width) h-full w-full p-3 sm:p-6 md:p-8 md:px-12">
          <RouterView v-slot="{ Component, route }">
            <!-- View transitions between pages -->
            <Transition name="fade" mode="out-in">
              <!-- The page being rendered -->
              <component class="h-full w-full" :is="Component" :key="route.fullPath"></component>
            </Transition>
          </RouterView>
        </div>
      </div>
    </div>
  </ErrorBoundary>
</template>

<style>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 300ms ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
