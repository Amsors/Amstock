<script setup lang="ts">
import { UI_CONFIG } from '../uiConfig'

withDefaults(defineProps<{ message: string; fadeMs?: number }>(), {
  fadeMs: UI_CONFIG.toastFadeMs,
})
</script>

<template>
  <Teleport to="body">
    <Transition name="toast">
      <div v-if="message" class="toast-notice" :style="{ '--toast-fade-ms': `${fadeMs}ms` }" role="status" aria-live="polite">{{ message }}</div>
    </Transition>
  </Teleport>
</template>

