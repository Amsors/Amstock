<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { api } from './api'
import { useToast } from './composables/useToast'
import type { StockElement } from './types'
import DeleteDialog from './components/DeleteDialog.vue'
import BatchEditDialog from './components/BatchEditDialog.vue'
import ElementCard from './components/ElementCard.vue'
import ElementForm from './components/ElementForm.vue'
import MappingManager from './components/MappingManager.vue'
import LoginView from './components/LoginView.vue'
import TreeView from './components/TreeView.vue'
import ToastNotice from './components/ToastNotice.vue'

type Page = 'home' | 'tree' | 'mappings'
type BatchMode = 'parent' | 'tags'
const isAndroidApp = /\bAmstockAndroid\//.test(navigator.userAgent)
const page = ref<Page>('home')
const checkingSession = ref(true)
const authenticated = ref(false)
const currentUsername = ref('')
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

function resetAuthenticatedUi() {
  authenticated.value = false
  currentUsername.value = ''
  elements.value = []
  formOpen.value = false
  deleting.value = null
  batchMode.value = null
  clearSelection()
}

function handleUnauthorized() { resetAuthenticatedUi() }
function handleAppLogout() { void logout() }

async function acceptAuthentication(username: string) {
  authenticated.value = true
  currentUsername.value = username
  await search()
}

async function logout() {
  try { await api.logout() } finally { resetAuthenticatedUi() }
}

onMounted(async () => {
  window.addEventListener('amstock:unauthorized', handleUnauthorized)
  window.addEventListener('amstock:logout', handleAppLogout)
  try {
    const session = await api.session()
    await acceptAuthentication(session.username)
  } catch {
    resetAuthenticatedUi()
  } finally {
    checkingSession.value = false
  }
})
onBeforeUnmount(() => {
  window.removeEventListener('amstock:unauthorized', handleUnauthorized)
  window.removeEventListener('amstock:logout', handleAppLogout)
})
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
  <div v-if="checkingSession" class="session-loading"><span class="brand-mark">A</span><p>正在检查登录状态…</p></div>
  <LoginView v-else-if="!authenticated" @authenticated="acceptAuthentication" />
  <div v-else class="app-shell" :class="{ 'android-app-shell': isAndroidApp }">
    <header class="topbar">
      <button class="brand" @click="page = 'home'"><span class="brand-mark">A</span><span><strong>Amstock</strong><small>家用物资管理</small></span></button>
      <nav class="primary-nav" aria-label="主要导航">
        <button :class="{ active: page === 'home' }" :aria-current="page === 'home' ? 'page' : undefined" @click="page = 'home'">
          <span class="nav-icon-wrap"><svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="m20 20-4.4-4.4m2.4-5.1a7.5 7.5 0 1 1-15 0 7.5 7.5 0 0 1 15 0Z" /></svg></span>
          <span>检索与创建</span>
        </button>
        <button :class="{ active: page === 'tree' }" :aria-current="page === 'tree' ? 'page' : undefined" @click="page = 'tree'">
          <span class="nav-icon-wrap"><svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 4v5m0 0H6v5m6-5h6v5M3.5 14h5v5h-5v-5Zm8.5 0h5v5h-5v-5Zm7.5 0h-5v5h5v-5Z" /></svg></span>
          <span>收纳树</span>
        </button>
        <button :class="{ active: page === 'mappings' }" :aria-current="page === 'mappings' ? 'page' : undefined" @click="page = 'mappings'">
          <span class="nav-icon-wrap"><svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M8 7h11m-3-3 3 3-3 3M16 17H5m3 3-3-3 3-3" /></svg></span>
          <span>编号映射</span>
        </button>
        <button v-if="!isAndroidApp" class="logout-button" :title="`当前用户：${currentUsername}`" @click="logout">
          <svg class="nav-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M14 8V5a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h7a2 2 0 0 0 2-2v-3m-3-4h10m-3-3 3 3-3 3" /></svg>
          <span>退出</span>
        </button>
      </nav>
    </header>

    <main>
      <template v-if="page === 'home'">
        <section class="search-panel panel">
          <label class="search-box"><span>⌕</span><input v-model="query" placeholder="输入名称或编号，例如：电阻、M-03、000042" :autofocus="!isAndroidApp" /><button v-if="query" aria-label="清空" @click="query = ''">×</button></label>
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
