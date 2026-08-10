<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { api } from '../api'
import type { ElementInput, StockElement } from '../types'

type BatchMode = 'parent' | 'tags'
type NumberMode = 'same' | 'sequence'

const props = defineProps<{ elements: StockElement[]; mode: BatchMode }>()
const emit = defineEmits<{ close: []; saved: [count: number] }>()

const busy = ref(false)
const error = ref('')
const parentQuery = ref('')
const parentMatches = ref<StockElement[]>([])
const selectedParent = ref<StockElement | null>(null)
const parentFocused = ref(false)
const parentLoading = ref(false)
const setA = ref(false)
const tagA = ref(props.elements[0]?.tag_a ?? 'A')
const setB = ref(false)
const tagBMode = ref<NumberMode>('same')
const tagBStart = ref(props.elements[0]?.tag_b ?? 0)
const setC = ref(false)
const tagCMode = ref<NumberMode>('same')
const tagCStart = ref(props.elements[0]?.tag_c ?? 0)
let parentTimer = 0
let parentSequence = 0

const categoryOptions = Array.from({ length: 26 }, (_, index) => String.fromCharCode(65 + index))
const hasTagChange = computed(() => setA.value || setB.value || setC.value)

function batchValue(start: number, mode: NumberMode, index: number) {
  return start + (mode === 'sequence' ? index : 0)
}

const rangeError = computed(() => {
  const lastIndex = Math.max(0, props.elements.length - 1)
  if (setB.value && batchValue(tagBStart.value, tagBMode.value, lastIndex) > 99) return 'BB 连续值不能超过 99，请减小起始值。'
  if (setC.value && batchValue(tagCStart.value, tagCMode.value, lastIndex) > 99) return 'CC 连续值不能超过 99，请减小起始值。'
  return ''
})

const preview = computed(() => props.elements.map((element, index) => ({
  element,
  tagA: setA.value ? tagA.value : element.tag_a,
  tagB: setB.value ? batchValue(tagBStart.value, tagBMode.value, index) : element.tag_b,
  tagC: setC.value ? batchValue(tagCStart.value, tagCMode.value, index) : element.tag_c,
})))

watch(parentQuery, (value) => {
  window.clearTimeout(parentTimer)
  if (selectedParent.value?.code === value) { parentLoading.value = false; return }
  selectedParent.value = null
  parentMatches.value = []
  const query = value.trim()
  if (!query) { parentLoading.value = false; return }
  parentLoading.value = true
  const sequence = ++parentSequence
  parentTimer = window.setTimeout(async () => {
    try {
      const selectedSerials = new Set(props.elements.map(element => element.serial))
      const matches = await api.search(query)
      if (sequence === parentSequence && parentQuery.value.trim() === query) {
        parentMatches.value = matches.filter(element => element.kind === 'container' && !element.deleted_at && !selectedSerials.has(element.serial))
      }
    } catch { /* 搜索建议失败不覆盖批量提交错误 */ }
    finally { if (sequence === parentSequence) parentLoading.value = false }
  }, 220)
})

function chooseParent(parent: StockElement) {
  selectedParent.value = parent
  parentQuery.value = parent.code
  parentMatches.value = []
  parentFocused.value = false
}

function toInput(element: StockElement, index: number): ElementInput {
  return {
    kind: element.kind,
    tag_a: setA.value ? tagA.value : element.tag_a,
    tag_b: setB.value ? batchValue(tagBStart.value, tagBMode.value, index) : element.tag_b,
    tag_c: setC.value ? batchValue(tagCStart.value, tagCMode.value, index) : element.tag_c,
    name: element.name,
    description: element.description,
    quantity: element.quantity,
    unit: element.unit,
    parent_serial: props.mode === 'parent' ? selectedParent.value!.serial : element.parent_serial,
  }
}

async function submit() {
  error.value = ''
  if (props.mode === 'parent' && !selectedParent.value) { error.value = '请从搜索结果中选择一个父容器。'; return }
  if (props.mode === 'tags' && !hasTagChange.value) { error.value = '请至少勾选 A、BB 或 CC 中的一项。'; return }
  if (rangeError.value) { error.value = rangeError.value; return }
  busy.value = true
  let completed = 0
  try {
    for (const [index, element] of props.elements.entries()) {
      await api.update(element.serial, toInput(element, index))
      completed += 1
    }
    emit('saved', completed)
  } catch (exception) {
    const prefix = completed ? `已成功更新前 ${completed} 条；第 ${completed + 1} 条失败：` : ''
    error.value = `${prefix}${(exception as Error).message}`
  } finally { busy.value = false }
}
</script>

<template>
  <div class="modal-backdrop" @mousedown.self="$emit('close')">
    <form class="modal batch-modal" @submit.prevent="submit">
      <div class="modal-header">
        <div><p class="eyebrow">批量操作 · FIFO 顺序</p><h2>{{ mode === 'parent' ? '设置共同父容器' : '批量设置编号' }}</h2></div>
        <button type="button" class="icon-button" aria-label="关闭" @click="$emit('close')">×</button>
      </div>

      <p class="batch-summary">将按勾选顺序处理 {{ elements.length }} 条物资。编号连续值从第 <strong>1</strong> 条开始递增。</p>

      <template v-if="mode === 'parent'">
        <label class="batch-parent-field">共同父容器
          <span class="parent-picker">
            <input v-model="parentQuery" required autocomplete="off" placeholder="输入容器名称或完整编号后选择" @focus="parentFocused = true" @blur="parentFocused = false" />
            <span v-if="parentFocused && parentQuery.trim() && !selectedParent" class="parent-suggestions">
              <span v-if="parentLoading" class="parent-suggestion-status">正在查询…</span>
              <button v-for="parent in parentMatches" v-else :key="parent.serial" type="button" @mousedown.prevent="chooseParent(parent)"><strong>{{ parent.name }}</strong><code>{{ parent.code }}</code></button>
              <span v-if="!parentLoading && !parentMatches.length" class="parent-suggestion-status">没有可用的匹配容器</span>
            </span>
          </span>
        </label>
        <p v-if="selectedParent" class="selected-parent">将统一放入：<strong>{{ selectedParent.name }}</strong> <code>{{ selectedParent.code }}</code></p>
      </template>

      <div v-else class="batch-tag-settings">
        <section class="batch-tag-row">
          <label class="batch-enable"><input v-model="setA" type="checkbox" />设置 A</label>
          <label>相同值<select v-model="tagA" :disabled="!setA"><option v-for="option in categoryOptions" :key="option" :value="option">{{ option }}</option></select></label>
        </section>
        <section class="batch-tag-row">
          <label class="batch-enable"><input v-model="setB" type="checkbox" />设置 BB</label>
          <label>方式<select v-model="tagBMode" :disabled="!setB"><option value="same">全部相同</option><option value="sequence">从起始值连续递增</option></select></label>
          <label>{{ tagBMode === 'sequence' ? '起始值' : '相同值' }}<input v-model.number="tagBStart" :disabled="!setB" required type="number" min="0" max="99" /></label>
        </section>
        <section class="batch-tag-row">
          <label class="batch-enable"><input v-model="setC" type="checkbox" />设置 CC</label>
          <label>方式<select v-model="tagCMode" :disabled="!setC"><option value="same">全部相同</option><option value="sequence">从起始值连续递增</option></select></label>
          <label>{{ tagCMode === 'sequence' ? '起始值' : '相同值' }}<input v-model.number="tagCStart" :disabled="!setC" required type="number" min="0" max="99" /></label>
        </section>
        <p v-if="rangeError" class="inline-validation">{{ rangeError }}</p>
      </div>

      <section class="batch-preview">
        <h3>{{ mode === 'tags' ? '变更预览' : '处理顺序' }}</h3>
        <ol>
          <li v-for="(row, index) in preview" :key="row.element.serial">
            <span class="order-badge">{{ index + 1 }}</span>
            <span class="batch-element-name">{{ row.element.name }}</span>
            <code v-if="mode === 'tags'">{{ row.tagA }}-{{ String(row.tagB).padStart(2, '0') }}-{{ String(row.tagC).padStart(2, '0') }}-{{ String(row.element.serial).padStart(6, '0') }}</code>
            <code v-else>{{ row.element.code }}</code>
          </li>
        </ol>
      </section>

      <p v-if="error" class="error-message">{{ error }}</p>
      <div class="modal-actions"><button type="button" class="button ghost" :disabled="busy" @click="$emit('close')">取消</button><button class="button primary" :disabled="busy || !!rangeError">{{ busy ? '正在批量保存…' : `确认修改 ${elements.length} 条` }}</button></div>
    </form>
  </div>
</template>
