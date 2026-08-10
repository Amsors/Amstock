<script setup lang="ts">
import { ref } from 'vue'
import type { TreeNode } from '../types'
import LabelPrintButtons from './LabelPrintButtons.vue'

defineProps<{ node: TreeNode; depth?: number }>()
defineEmits<{ select: [node: TreeNode] }>()
const open = ref(true)
</script>

<template>
  <li class="tree-item">
    <div class="tree-row" :style="{ paddingLeft: `${(depth || 0) * 18 + 8}px` }">
      <button v-if="node.children.length" class="tree-toggle" :aria-label="open ? '折叠' : '展开'" @click="open = !open">{{ open ? '⌄' : '›' }}</button>
      <span v-else class="tree-toggle muted">·</span>
      <button class="tree-content" @click="$emit('select', node)">
        <span class="tree-icon">{{ node.kind === 'container' ? '▣' : '◇' }}</span>
        <span><strong>{{ node.name }}</strong><code>{{ node.code }}</code></span>
        <span class="tree-quantity">{{ node.quantity }} {{ node.unit }}</span>
      </button>
      <LabelPrintButtons :element="node" />
    </div>
    <ul v-if="open && node.children.length">
      <TreeBranch v-for="child in node.children" :key="child.serial" :node="child" :depth="(depth || 0) + 1" @select="$emit('select', $event)" />
    </ul>
  </li>
</template>
