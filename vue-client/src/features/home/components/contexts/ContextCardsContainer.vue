<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->
<!---->
<!-- SPDX-License-Identifier: Apache-2.0 -->

<script setup lang="ts">
import { useContextStore } from '@/core/stores/context-store'
import ContextCard from './ContextCard.vue'

const contextsStore = useContextStore()

const emits = defineEmits<{
  (e: 'onOpenSerial', name: string): void
  (e: 'onOpenKVM', name: string): void
}>()
</script>

<template>
  <div class="flex gap-6 pt-6">
    <ContextCard
      @on-open-serial="(name) => emits('onOpenSerial', name)"
      @on-open-k-v-m="(name: string) => emits('onOpenKVM', name)"
      :machine="item"
      v-for="item of contextsStore.activeMachines"
    ></ContextCard>
  </div>
</template>
