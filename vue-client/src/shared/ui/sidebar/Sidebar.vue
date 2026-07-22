<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import Tooltip from '../tooltip/Tooltip.vue'
import { SIDEBAR_CONFIG } from './models/sidebar.model'
import type { SidebarRoute } from './models/sidebar.model'
import { useRouter, useRoute } from 'vue-router'

const router = useRouter()

const route = useRoute()

const onSidebarClick = (item: SidebarRoute) => {
  router.push({ name: item.name })
}
</script>

<template>
  <div class="h-full">
    <div
      class="border-(--app-secondary-border) flex h-full w-full flex-col items-center justify-center border-r-[1px] px-3 py-6 sm:px-8"
    >
      <div class="flex h-full max-h-[512px] flex-col justify-between gap-3">
        <button v-on:click="onSidebarClick(item)" v-for="item in SIDEBAR_CONFIG" :key="item.name">
          <Tooltip
            :options="{ message: item.displayName, xOffsetOverride: -24, yOffsetOverride: 32 }"
            class="z-0"
          >
            <component
              :class="
                route.name === item.name
                  ? 'text-(--app-primary-text)'
                  : 'text-(--app-secondary-text) opacity-90'
              "
              class="hover: hover:text-(--app-primary-text) h-8 w-8 transition duration-300 hover:-translate-y-0.5"
              :is="item.icon"
            />
          </Tooltip>
        </button>
      </div>
    </div>
  </div>
</template>
