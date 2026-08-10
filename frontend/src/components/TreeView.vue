<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api'
import type { StockElement, TreeNode } from '../types'
import TreeBranch from './TreeBranch.vue'

const emit = defineEmits<{ select: [element: StockElement] }>()
const nodes = ref<TreeNode[]>([])
const loading = ref(true)
const error = ref('')

async function load() {
  loading.value = true; error.value = ''
  try { nodes.value = await api.tree() }
  catch (e) { error.value = (e as Error).message }
  finally { loading.value = false }
}
onMounted(load)
defineExpose({ load })
</script>

<template>
  <section class="panel tree-panel">
    <div class="section-header"><div><p class="eyebrow">收纳结构</p><h2>容器树</h2></div><button class="button ghost small" @click="load">刷新</button></div>
    <p v-if="loading" class="empty-state">正在载入收纳结构…</p>
    <p v-else-if="error" class="error-message">{{ error }}</p>
    <p v-else-if="!nodes.length" class="empty-state">还没有物资。先从首页创建第一个条目吧。</p>
    <ul v-else class="tree-root"><TreeBranch v-for="node in nodes" :key="node.serial" :node="node" @select="emit('select', $event)" /></ul>
  </section>
</template>
