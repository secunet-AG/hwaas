<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>

<template>
  <div class="flex h-full w-full items-center justify-center">
    <div class="flex w-[350px] flex-col items-center gap-1">
      <h4>No Contexts Found</h4>
      <span class="text-(--app-secondary-text) text-center font-light"
        >You currently do not have any contexts. Click below to get started</span
      >
      <button
        @click="router.push({ name: 'createContext' })"
        class="app-btn-primary mt-3"
        to="/contexts/create"
      >
        Create a new context
      </button>
    </div>
  </div>
</template>
