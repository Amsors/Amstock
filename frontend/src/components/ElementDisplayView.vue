<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { api } from '../api'
import type { ElementLookup, StockElement } from '../types'

const props = defineProps<{ identifier: string }>()
const lookup = ref<ElementLookup | null>(null)
const loading = ref(true)
const error = ref('')
const originalTitle = document.title
let requestVersion = 0

const parents = computed(() => lookup.value?.path.filter(entry => entry.serial !== lookup.value?.element.serial) ?? [])

function formatDate(value: string | null) {
  if (!value) return '—'
  const sqliteUtc = /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/.test(value)
    ? `${value.replace(' ', 'T')}Z`
    : value
  const date = new Date(sqliteUtc)
  return Number.isNaN(date.getTime()) ? value : new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit',
  }).format(date)
}

function matchesIdentifier(element: StockElement, identifier: string) {
  if (/^\d{1,6}$/.test(identifier)) return element.serial === Number(identifier)
  return element.code.toUpperCase() === identifier.toUpperCase()
}

async function load(identifier: string) {
  const version = ++requestVersion
  lookup.value = null
  error.value = ''
  loading.value = true
  const normalized = identifier.trim()
  if (!normalized) {
    error.value = 'URL 中缺少物资编号'
    loading.value = false
    return
  }
  try {
    const results = await api.lookup(normalized, true)
    if (version !== requestVersion) return
    lookup.value = results.find(entry => matchesIdentifier(entry.element, normalized)) ?? null
    if (!lookup.value) error.value = '未找到该物资'
    else document.title = `${lookup.value.element.name} · ${lookup.value.element.code}`
  } catch (cause) {
    if (version === requestVersion) error.value = (cause as Error).message
  } finally {
    if (version === requestVersion) loading.value = false
  }
}

watch(() => props.identifier, value => void load(value), { immediate: true })
onBeforeUnmount(() => {
  requestVersion += 1
  document.title = originalTitle
})
</script>

<template>
  <main class="display-page">
    <div v-if="loading" class="display-status" role="status">正在加载…</div>
    <div v-else-if="error" class="display-status display-error" role="alert">
      <span aria-hidden="true">!</span>
      <strong>{{ error }}</strong>
      <code>{{ identifier }}</code>
    </div>

    <article v-else-if="lookup" class="display-sheet" :class="{ deleted: lookup.element.deleted_at }">
      <header class="display-header">
        <div class="display-title">
          <div class="display-badges">
            <span class="display-kind">{{ lookup.element.kind === 'container' ? '容器' : '物品' }}</span>
            <span v-if="lookup.element.deleted_at" class="deleted-pill">已删除</span>
          </div>
          <h1>{{ lookup.element.name }}</h1>
          <code>{{ lookup.element.code }}</code>
        </div>
        <div class="display-quantity">
          <strong>{{ lookup.element.quantity }}</strong>
          <span v-if="lookup.element.unit">{{ lookup.element.unit }}</span>
        </div>
      </header>

      <div class="display-content">
        <section class="display-visual">
          <img
            v-if="lookup.element.has_image"
            :src="`/images/${lookup.element.serial}?v=${encodeURIComponent(lookup.element.updated_at)}`"
            :alt="lookup.element.name"
          />
          <div v-else class="display-image-placeholder" aria-hidden="true">
            {{ lookup.element.kind === 'container' ? '箱' : '物' }}
          </div>
        </section>

        <section class="display-details">
          <p class="display-description" :class="{ empty: !lookup.element.description }">
            {{ lookup.element.description || '—' }}
          </p>
          <dl class="display-field-grid">
            <div><dt>序列号</dt><dd>{{ String(lookup.element.serial).padStart(6, '0') }}</dd></div>
            <div><dt>类型</dt><dd>{{ lookup.element.kind === 'container' ? '容器' : '物品' }}</dd></div>
            <div><dt>分类 A</dt><dd>{{ lookup.element.tag_a }}</dd></div>
            <div><dt>编号 BB</dt><dd>{{ String(lookup.element.tag_b).padStart(2, '0') }}</dd></div>
            <div><dt>编号 CC</dt><dd>{{ String(lookup.element.tag_c).padStart(2, '0') }}</dd></div>
            <div><dt>数量 / 单位</dt><dd>{{ lookup.element.quantity }} {{ lookup.element.unit || '—' }}</dd></div>
            <div><dt>父容器序列号</dt><dd>{{ lookup.element.parent_serial == null ? '—' : String(lookup.element.parent_serial).padStart(6, '0') }}</dd></div>
            <div><dt>图片格式</dt><dd>{{ lookup.element.image_mime || '—' }}</dd></div>
            <div><dt>创建时间</dt><dd>{{ formatDate(lookup.element.created_at) }}</dd></div>
            <div><dt>更新时间</dt><dd>{{ formatDate(lookup.element.updated_at) }}</dd></div>
            <div v-if="lookup.element.deleted_at"><dt>删除时间</dt><dd>{{ formatDate(lookup.element.deleted_at) }}</dd></div>
          </dl>
        </section>
      </div>

      <section class="display-path" aria-label="父容器链条">
        <div class="display-path-origin">顶层</div>
        <template v-for="entry in parents" :key="entry.serial">
          <span class="display-path-arrow" aria-hidden="true">›</span>
          <div class="display-path-node">
            <span class="display-path-icon" aria-hidden="true">箱</span>
            <span><strong>{{ entry.name }}</strong><code>{{ entry.code }}</code></span>
            <span v-if="entry.deleted_at" class="deleted-pill">已删除</span>
          </div>
        </template>
        <span class="display-path-arrow" aria-hidden="true">›</span>
        <div class="display-path-node current">
          <span class="display-path-icon" aria-hidden="true">{{ lookup.element.kind === 'container' ? '箱' : '物' }}</span>
          <span><strong>{{ lookup.element.name }}</strong><code>{{ lookup.element.code }}</code></span>
        </div>
      </section>
    </article>
  </main>
</template>
