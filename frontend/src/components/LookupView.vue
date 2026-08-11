<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { api } from '../api'
import { useElementSearch } from '../composables/useElementSearch'
import type { ElementLookup } from '../types'
import ContainerPathDialog from './ContainerPathDialog.vue'
import ElementCard from './ElementCard.vue'
import ElementSearchControls from './ElementSearchControls.vue'

function filtersFromUrl() {
  const url = new URL(window.location.href)
  let pathQuery = ''
  if (url.pathname.startsWith('/lookup/')) {
    try { pathQuery = decodeURIComponent(url.pathname.slice('/lookup/'.length)) } catch { /* malformed path is treated as empty */ }
  }
  return {
    query: url.searchParams.get('q') || url.searchParams.get('code') || url.searchParams.get('serial') || pathQuery,
    includeDeleted: url.searchParams.get('include_deleted') === 'true',
  }
}

function updateUrl(query: string, includeDeleted: boolean) {
  const url = new URL('/lookup', window.location.origin)
  if (query.trim()) url.searchParams.set('q', query.trim())
  if (includeDeleted) url.searchParams.set('include_deleted', 'true')
  window.history.replaceState({}, '', `${url.pathname}${url.search}`)
}

const initialFilters = filtersFromUrl()
const selectedPath = ref<ElementLookup | null>(null)
const { query, includeDeleted, elements, loading, error, search } = useElementSearch({
  load: api.lookup,
  initialQuery: initialFilters.query,
  initialIncludeDeleted: initialFilters.includeDeleted,
  onFiltersChanged: updateUrl,
})

function handlePopState() {
  const filters = filtersFromUrl()
  query.value = filters.query
  includeDeleted.value = filters.includeDeleted
}

onMounted(() => {
  window.addEventListener('popstate', handlePopState)
  void search()
})
onBeforeUnmount(() => window.removeEventListener('popstate', handlePopState))
</script>

<template>
  <ElementSearchControls v-model:query="query" v-model:include-deleted="includeDeleted" readonly :loading="loading" />
  <section class="results readonly-results">
    <div class="results-heading">
      <div><h2>{{ query ? '检索结果' : '最近更新' }}</h2><p>只读页面 · 查询条件会同步到当前 URL</p></div>
      <span>{{ elements.length }} 条</span>
    </div>
    <p v-if="loading" class="empty-state">正在查找…</p>
    <p v-else-if="error" class="error-message">{{ error }}</p>
    <p v-else-if="!elements.length" class="empty-state">没有找到匹配的物资。可以换个关键词继续查找。</p>
    <div v-else class="element-list">
      <ElementCard v-for="entry in elements" :key="entry.element.serial" :element="entry.element" :path="entry.path" readonly @show-path="selectedPath = entry" />
    </div>
  </section>
  <ContainerPathDialog v-if="selectedPath" :lookup="selectedPath" @close="selectedPath = null" />
</template>
