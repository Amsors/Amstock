<script setup lang="ts">
import type { StockElement } from '../types'
import LabelPrintButtons from './LabelPrintButtons.vue'

defineProps<{ element: StockElement; selectionOrder?: number | null; selectable?: boolean }>()
defineEmits<{
  edit: [element: StockElement]
  remove: [element: StockElement]
  restore: [element: StockElement]
  toggleSelection: [element: StockElement]
}>()
</script>

<template>
  <article class="element-card" :class="{ deleted: element.deleted_at, selected: selectionOrder != null }">
    <label class="selection-control" :class="{ disabled: !selectable }" :title="selectable ? (selectionOrder == null ? '加入批量选择' : `第 ${selectionOrder} 个选中`) : '已删除条目不能批量修改'">
      <input type="checkbox" :checked="selectionOrder != null" :disabled="!selectable" :aria-label="`选择 ${element.name}`" @change="$emit('toggleSelection', element)" />
      <span class="selection-indicator" aria-hidden="true">{{ selectionOrder ?? '' }}</span>
    </label>
    <img v-if="element.has_image" class="thumb" :src="`/images/${element.serial}?v=${encodeURIComponent(element.updated_at)}`" :alt="element.name" />
    <div v-else class="thumb placeholder" aria-hidden="true">{{ element.kind === 'container' ? '箱' : '物' }}</div>
    <div class="element-main">
      <div class="title-row">
        <h3>{{ element.name }}</h3>
        <span class="kind-pill">{{ element.kind === 'container' ? '容器' : '物品' }}</span>
        <span v-if="element.deleted_at" class="deleted-pill">已删除</span>
      </div>
      <code>{{ element.code }}</code>
      <p v-if="element.description" class="element-description">{{ element.description }}</p>
      <p class="meta">
        <span>数量 {{ element.quantity }} {{ element.unit }}</span>
        <span>{{ element.parent_serial == null ? '未放入容器' : `父容器 ${String(element.parent_serial).padStart(6, '0')}` }}</span>
      </p>
    </div>
    <div class="card-actions">
      <LabelPrintButtons v-if="!element.deleted_at" :element="element" />
      <div class="management-actions">
        <button v-if="!element.deleted_at" class="button ghost small" @click="$emit('edit', element)">编辑</button>
        <button v-if="!element.deleted_at" class="button danger-ghost small" @click="$emit('remove', element)">移除</button>
        <button v-else class="button small" @click="$emit('restore', element)">恢复</button>
      </div>
    </div>
  </article>
</template>
