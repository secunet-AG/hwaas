<!--
SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>

SPDX-License-Identifier: Apache-2.0
-->

<script setup lang="ts">
const completionPercentage = defineModel<number>()
</script>

<template>
  <div class="bg-(--app-secondary-border) w-full rounded-2xl">
    <div
      :style="{
        width: `${completionPercentage}%`,
      }"
      class="h-1 rounded-2xl bg-white transition-all"
    ></div>
  </div>
</template>
