<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { api } from './api'
import type { StockElement } from './types'
import DeleteDialog from './components/DeleteDialog.vue'
import ElementCard from './components/ElementCard.vue'
import ElementForm from './components/ElementForm.vue'
import MappingManager from './components/MappingManager.vue'
import TreeView from './components/TreeView.vue'

type Page = 'home' | 'tree' | 'mappings'
const page = ref<Page>('home')
const query = ref('')
const includeDeleted = ref(false)
const elements = ref<StockElement[]>([])
const loading = ref(false)
const error = ref('')
const formElement = ref<StockElement | undefined>()
const formOpen = ref(false)
const deleting = ref<StockElement | null>(null)
let searchTimer = 0
let formNeedsRefresh = false

async function search() {
  loading.value = true; error.value = ''
  try { elements.value = await api.search(query.value, includeDeleted.value) }
  catch (e) { error.value = (e as Error).message }
  finally { loading.value = false }
}
watch([query, includeDeleted], () => { window.clearTimeout(searchTimer); searchTimer = window.setTimeout(search, 220) })
onMounted(search)
function createElement() { formElement.value = undefined; formNeedsRefresh = false; formOpen.value = true }
function editElement(element: StockElement) { formElement.value = element; formNeedsRefresh = false; formOpen.value = true }
function continued() { formNeedsRefresh = true }
function closeForm() {
  formOpen.value = false
  if (formNeedsRefresh) { formNeedsRefresh = false; void search() }
}
function saved() { formNeedsRefresh = false; formOpen.value = false; void search() }
async function restore(element: StockElement) {
  try { await api.restore(element.serial); await search() } catch (e) { error.value = (e as Error).message }
}
function selectFromTree(element: StockElement) { page.value = 'home'; query.value = element.code; editElement(element) }
</script>

<template>
  <div class="app-shell">
    <header class="topbar">
      <button class="brand" @click="page = 'home'"><span class="brand-mark">A</span><span><strong>Amstock</strong><small>家用物资管理</small></span></button>
      <nav aria-label="主要导航">
        <button :class="{ active: page === 'home' }" @click="page = 'home'">检索与创建</button>
        <button :class="{ active: page === 'tree' }" @click="page = 'tree'">收纳树</button>
        <button :class="{ active: page === 'mappings' }" @click="page = 'mappings'">编号映射</button>
      </nav>
    </header>

    <main>
      <template v-if="page === 'home'">
        <section class="search-panel panel">
          <label class="search-box"><span>⌕</span><input v-model="query" placeholder="输入名称或编号，例如：电阻、M-03、000042" autofocus /><button v-if="query" aria-label="清空" @click="query = ''">×</button></label>
          <div class="search-controls"><label class="deleted-toggle"><input v-model="includeDeleted" type="checkbox" />包含已删除条目</label><button class="button primary create-button" @click="createElement"><span>＋</span> 添加物资</button></div>
        </section>
        <section class="results">
          <div class="results-heading"><h2>{{ query ? '检索结果' : '最近更新' }}</h2><span>{{ elements.length }} 条</span></div>
          <p v-if="loading" class="empty-state">正在查找…</p>
          <p v-else-if="error" class="error-message">{{ error }}</p>
          <p v-else-if="!elements.length" class="empty-state">没有找到匹配的物资。可以换个关键词，或创建新条目。</p>
          <div v-else class="element-list"><ElementCard v-for="element in elements" :key="element.serial" :element="element" @edit="editElement" @remove="deleting = $event" @restore="restore" /></div>
        </section>
      </template>
      <TreeView v-else-if="page === 'tree'" @select="selectFromTree" />
      <MappingManager v-else />
    </main>

    <ElementForm v-if="formOpen" :element="formElement" @close="closeForm" @saved="saved" @continued="continued" />
    <DeleteDialog v-if="deleting" :element="deleting" @close="deleting = null" @deleted="deleting = null; search()" />
  </div>
</template>
