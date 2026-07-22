<script setup lang="ts">
import { RouterView } from 'vue-router'
import Sidebar from './shared/ui/sidebar/Sidebar.vue'
import ErrorBoundary from './core/error-handling/ErrorBoundary.vue'
import Toaster from './shared/ui/toaster/Toaster.vue'
import Spinner from './shared/ui/loading/Spinner.vue'
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
