<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { api } from './api'
import { useToast } from './composables/useToast'
import type { StockElement } from './types'
import DeleteDialog from './components/DeleteDialog.vue'
import BatchEditDialog from './components/BatchEditDialog.vue'
import ElementCard from './components/ElementCard.vue'
import ElementForm from './components/ElementForm.vue'
import MappingManager from './components/MappingManager.vue'
import TreeView from './components/TreeView.vue'
import ToastNotice from './components/ToastNotice.vue'

type Page = 'home' | 'tree' | 'mappings'
type BatchMode = 'parent' | 'tags'
const page = ref<Page>('home')
const query = ref('')
const includeDeleted = ref(false)
const elements = ref<StockElement[]>([])
const loading = ref(false)
const error = ref('')
const formElement = ref<StockElement | undefined>()
const formOpen = ref(false)
const deleting = ref<StockElement | null>(null)
const selectedSerials = ref<number[]>([])
const batchMode = ref<BatchMode | null>(null)
let searchTimer = 0
let formNeedsRefresh = false
const { toastMessage, showToast } = useToast()
const selectedElements = computed(() => {
  const bySerial = new Map(elements.value.map(element => [element.serial, element]))
  return selectedSerials.value.map(serial => bySerial.get(serial)).filter((element): element is StockElement => !!element && !element.deleted_at)
})

function selectionOrder(serial: number) {
  const index = selectedSerials.value.indexOf(serial)
  return index < 0 ? null : index + 1
}

function toggleSelection(element: StockElement) {
  if (element.deleted_at) return
  const index = selectedSerials.value.indexOf(element.serial)
  if (index < 0) selectedSerials.value.push(element.serial)
  else selectedSerials.value.splice(index, 1)
}

function clearSelection() { selectedSerials.value = [] }

async function search() {
  loading.value = true; error.value = ''
  try { elements.value = await api.search(query.value, includeDeleted.value) }
  catch (e) { error.value = (e as Error).message }
  finally { loading.value = false }
}
watch([query, includeDeleted], () => { clearSelection(); window.clearTimeout(searchTimer); searchTimer = window.setTimeout(search, 220) })
onMounted(search)
function createElement() { formElement.value = undefined; formNeedsRefresh = false; formOpen.value = true }
function editElement(element: StockElement) { formElement.value = element; formNeedsRefresh = false; formOpen.value = true }
function continued() { formNeedsRefresh = true }
function closeForm() {
  formOpen.value = false
  if (formNeedsRefresh) { formNeedsRefresh = false; void search() }
}
function saved() { formNeedsRefresh = false; formOpen.value = false; void search() }
function batchSaved(count: number) {
  const action = batchMode.value === 'parent' ? '父容器' : '编号'
  batchMode.value = null
  clearSelection()
  showToast(`已批量更新 ${count} 条物资的${action}。`)
  void search()
}
function closeBatch() { batchMode.value = null; clearSelection(); void search() }
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
          <div v-if="selectedElements.length" class="batch-toolbar panel">
            <div><strong>已选 {{ selectedElements.length }} 条</strong><span>数字表示 FIFO 处理顺序</span></div>
            <div class="batch-toolbar-actions"><button class="button ghost small" @click="clearSelection">清空选择</button><button class="button ghost small" @click="batchMode = 'parent'">设置共同父容器</button><button class="button primary small" @click="batchMode = 'tags'">批量设置 A / BB / CC</button></div>
          </div>
          <p v-if="loading" class="empty-state">正在查找…</p>
          <p v-else-if="error" class="error-message">{{ error }}</p>
          <p v-else-if="!elements.length" class="empty-state">没有找到匹配的物资。可以换个关键词，或创建新条目。</p>
          <div v-else class="element-list"><ElementCard v-for="element in elements" :key="element.serial" :element="element" :selectable="!element.deleted_at" :selection-order="selectionOrder(element.serial)" @toggle-selection="toggleSelection" @edit="editElement" @remove="deleting = $event" @restore="restore" /></div>
        </section>
      </template>
      <TreeView v-else-if="page === 'tree'" @select="selectFromTree" />
      <MappingManager v-else />
    </main>

    <ElementForm v-if="formOpen" :element="formElement" @close="closeForm" @saved="saved" @continued="continued" @notice="showToast" />
    <BatchEditDialog v-if="batchMode && selectedElements.length" :elements="selectedElements" :mode="batchMode" @close="closeBatch" @saved="batchSaved" />
    <DeleteDialog v-if="deleting" :element="deleting" @close="deleting = null" @deleted="deleting = null; search()" />
    <ToastNotice :message="toastMessage" />
  </div>
</template>
