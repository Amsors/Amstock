<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import type { ElementLookup } from '../types'

defineProps<{ lookup: ElementLookup }>()
const emit = defineEmits<{ close: [] }>()
const closeButton = ref<HTMLButtonElement | null>(null)
const previousBodyOverflow = document.body.style.overflow

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close')
}

onMounted(() => {
  document.body.style.overflow = 'hidden'
  window.addEventListener('keydown', handleKeydown)
  void nextTick(() => closeButton.value?.focus())
})
onBeforeUnmount(() => {
  document.body.style.overflow = previousBodyOverflow
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div class="modal-backdrop path-backdrop" @mousedown.self="$emit('close')">
    <section class="modal path-modal" role="dialog" aria-modal="true" aria-labelledby="path-dialog-title">


      <ol class="containment-chain">
        <li class="chain-origin"><span>顶层</span></li>
        <li v-for="(entry, index) in lookup.path" :key="entry.serial" class="chain-step" :class="{ current: entry.serial === lookup.element.serial }">
          <span class="chain-connector" aria-hidden="true">↓</span>
          <span class="chain-icon" aria-hidden="true">{{ entry.kind === 'container' ? '箱' : '物' }}</span>
          <span class="chain-content">
            <small>{{ entry.serial === lookup.element.serial ? `当前${entry.kind === 'container' ? '容器' : '物品'}` : `第 ${index + 1} 层容器` }}</small>
            <strong>{{ entry.name }}</strong>
            <code>{{ entry.code }}</code>
          </span>
          <span v-if="entry.deleted_at" class="deleted-pill">已删除</span>
        </li>
      </ol>

      <div class="modal-actions path-dialog-actions">
        <button class="button primary" type="button" @click="$emit('close')">关闭</button>
      </div>
    </section>
  </div>
</template>
