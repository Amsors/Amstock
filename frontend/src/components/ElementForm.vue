<script setup lang="ts">
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue'
import { api, serialFromCode } from '../api'
import { useToast } from '../composables/useToast'
import type { CategoryMapping, ElementInput, MnemonicMapping, StockElement } from '../types'
import ToastNotice from './ToastNotice.vue'

const props = defineProps<{ element?: StockElement }>()
const emit = defineEmits<{ close: []; saved: [element: StockElement]; continued: [element: StockElement]; notice: [message: string] }>()

const form = reactive({
  kind: props.element?.kind ?? 'item', tag_a: props.element?.tag_a ?? '',
  tag_b: props.element ? String(props.element.tag_b).padStart(2, '0') : '',
  tag_c: props.element ? String(props.element.tag_c).padStart(2, '0') : '',
  name: props.element?.name ?? '', description: props.element?.description ?? '', quantity: props.element?.quantity ?? 1,
  unit: props.element?.unit ?? '', parent: props.element?.parent_serial == null ? '' : String(props.element.parent_serial).padStart(6, '0'),
})
const categories = ref<CategoryMapping[]>([])
const mnemonics = ref<MnemonicMapping[]>([])
const containers = ref<StockElement[]>([])
const image = ref<File | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)
const nameInput = ref<HTMLInputElement | null>(null)
const formElementNode = ref<HTMLFormElement | null>(null)
const removeExistingImage = ref(false)
const busy = ref(false)
const error = ref('')
const previewUrl = ref('')
const parentFocused = ref(false)
const parentSearchLoading = ref(false)
const title = computed(() => props.element ? `编辑 ${props.element.code}` : '添加物资')
const categoryOptions = computed(() => {
  const configured = new Map(categories.value.map(row => [row.tag_a, row.name]))
  return Array.from({ length: 26 }, (_, index) => {
    const value = String.fromCharCode(65 + index)
    return { value, name: configured.get(value) || null }
  })
})
const mnemonicOptions = computed(() => {
  const configured = new Map(mnemonics.value.map(row => [row.tag_b, row.name]))
  return Array.from({ length: 100 }, (_, value) => ({
    value: String(value).padStart(2, '0'), name: configured.get(value) || null,
  }))
})
type ContinueCcBehavior = 'keep' | 'reset' | 'increment'
type PrintAfterSave = 'none' | 'A1' | 'A2'
function readContinueCcBehavior(): ContinueCcBehavior {
  const value = document.cookie.split('; ').find(row => row.startsWith('amstock_continue_cc='))?.split('=')[1]
  return value === 'keep' || value === 'reset' || value === 'increment' ? value : 'increment'
}
const continueCcBehavior = ref<ContinueCcBehavior>(readContinueCcBehavior())
function readPrintAfterSave(): PrintAfterSave {
  const value = document.cookie.split('; ').find(row => row.startsWith('amstock_print_after_save='))?.split('=')[1]
  return value === 'A1' || value === 'A2' || value === 'none' ? value : 'none'
}
const printAfterSave = ref<PrintAfterSave>(readPrintAfterSave())
const { toastMessage, showToast } = useToast()
let parentSearchTimer = 0
let parentSearchSequence = 0
let suppressParentSearch = false

async function loadMnemonics() {
  if (!/^[A-Z]$/.test(form.tag_a)) { mnemonics.value = []; return }
  try { mnemonics.value = await api.mnemonics(form.tag_a) } catch { mnemonics.value = [] }
}
watch(() => form.tag_a, () => { void loadMnemonics() })
watch(continueCcBehavior, value => {
  document.cookie = `amstock_continue_cc=${value}; Max-Age=31536000; Path=/; SameSite=Lax`
})
watch(printAfterSave, value => {
  document.cookie = `amstock_print_after_save=${value}; Max-Age=31536000; Path=/; SameSite=Lax`
})
watch(() => form.parent, (value) => {
  window.clearTimeout(parentSearchTimer)
  containers.value = []
  if (suppressParentSearch) { suppressParentSearch = false; parentSearchLoading.value = false; return }
  const query = value.trim()
  if (!query) { parentSearchLoading.value = false; return }
  parentSearchLoading.value = true
  const sequence = ++parentSearchSequence
  parentSearchTimer = window.setTimeout(async () => {
    try {
      const matches = await api.search(query)
      if (sequence === parentSearchSequence && form.parent.trim() === query) {
        containers.value = matches.filter(e => e.kind === 'container' && e.serial !== props.element?.serial)
      }
    } catch { /* 输入建议查询失败时不阻断表单 */ }
    finally { if (sequence === parentSearchSequence) parentSearchLoading.value = false }
  }, 220)
})
watch(image, (file, old) => {
  if (previewUrl.value) URL.revokeObjectURL(previewUrl.value)
  previewUrl.value = file ? URL.createObjectURL(file) : ''
  if (file) removeExistingImage.value = false
  void old
})

onMounted(async () => {
  try {
    categories.value = await api.categories()
    await loadMnemonics()
  } catch (e) { error.value = (e as Error).message }
})

function guardDigitKey(event: KeyboardEvent) {
  if (!event.ctrlKey && !event.metaKey && !event.altKey && event.key.length === 1 && !/^\d$/.test(event.key)) event.preventDefault()
}

function inputTwoDigits(field: 'tag_b' | 'tag_c', event: Event) {
  const input = event.currentTarget as HTMLInputElement
  const value = input.value.replace(/\D/g, '').slice(0, 2)
  input.value = value
  form[field] = value
}

function formatTwoDigits(field: 'tag_b' | 'tag_c') {
  if (form[field]) form[field] = form[field].padStart(2, '0')
}

function selectParent(container: StockElement) {
  suppressParentSearch = true
  form.parent = container.code
  containers.value = []
  parentFocused.value = false
}

async function resetForNext(saved: StockElement, printNotice = '') {
  if (continueCcBehavior.value === 'reset') form.tag_c = '00'
  if (continueCcBehavior.value === 'increment') form.tag_c = String((Number(form.tag_c) + 1) % 100).padStart(2, '0')
  form.name = ''
  form.description = ''
  form.quantity = 1
  form.unit = ''
  form.parent = ''
  containers.value = []
  image.value = null
  if (fileInput.value) fileInput.value.value = ''
  removeExistingImage.value = false
  showToast(`已添加 ${saved.code}，可以继续录入下一条。${printNotice ? ` ${printNotice}` : ''}`)
  await nextTick()
  nameInput.value?.focus()
}

async function submit(continueAdding = false) {
  if (formElementNode.value && !formElementNode.value.checkValidity()) {
    formElementNode.value.reportValidity()
    return
  }
  formatTwoDigits('tag_b'); formatTwoDigits('tag_c')
  busy.value = true; error.value = ''
  try {
    const input: ElementInput = {
      kind: form.kind, tag_a: form.tag_a, tag_b: Number(form.tag_b), tag_c: Number(form.tag_c),
      name: form.name, description: form.description, quantity: Number(form.quantity), unit: form.unit, parent_serial: serialFromCode(form.parent),
    }
    const saved = props.element ? await api.update(props.element.serial, input) : await api.create(input)
    if (removeExistingImage.value && props.element?.has_image) await api.deleteImage(saved.serial)
    if (image.value) await api.uploadImage(saved.serial, image.value)
    const savedView = { ...saved, has_image: image.value ? true : removeExistingImage.value ? false : saved.has_image, updated_at: new Date().toISOString() }
    let printNotice = ''
    if (!props.element && printAfterSave.value !== 'none') {
      try {
        const result = await api.printLabel(saved.serial, printAfterSave.value)
        printNotice = result.mode === 'preview' ? `${printAfterSave.value} 标签预览已生成并打开。` : `${printAfterSave.value} 标签已发送到打印机。`
      } catch (printError) {
        printNotice = `打印失败：${(printError as Error).message}`
      }
    }
    if (continueAdding && !props.element) {
      emit('continued', savedView)
      await resetForNext(savedView, printNotice)
    } else {
      if (printNotice) emit('notice', printNotice.startsWith('打印失败') ? `条目 ${saved.code} 已保存，但${printNotice}` : printNotice)
      emit('saved', savedView)
    }
  } catch (e) { error.value = (e as Error).message }
  finally { busy.value = false }
}
</script>

<template>
  <div class="modal-backdrop" @mousedown.self="$emit('close')">
    <form ref="formElementNode" class="modal form-modal" autocomplete="off" @submit.prevent="submit(false)">
      <div class="modal-header"><div><p class="eyebrow">{{ element ? '修改条目' : '创建条目' }}</p><h2>{{ title }}</h2></div><button type="button" class="icon-button" @click="$emit('close')">×</button></div>
      <div class="form-grid">
        <label class="type-field">元素类型<span class="type-switch"><button type="button" :class="{ active: form.kind === 'item' }" :aria-pressed="form.kind === 'item'" @click="form.kind = 'item'">物品</button><button type="button" :class="{ active: form.kind === 'container' }" :aria-pressed="form.kind === 'container'" @click="form.kind = 'container'">容器</button></span></label>
        <label class="wide">名称<input ref="nameInput" v-model="form.name" required maxlength="120" autocomplete="off" placeholder="例如：M3 不锈钢螺母" autofocus /></label>
        <label class="full">描述（可选）<textarea v-model="form.description" maxlength="1000" autocomplete="off" rows="2" placeholder="记录规格、用途或其他备注"></textarea></label>
        <label>类别位 A<select v-model="form.tag_a" required><option disabled value="">请选择</option><optgroup label="已配置"><option v-for="option in categoryOptions.filter(item => item.name)" :key="option.value" :value="option.value">{{ option.value }} — {{ option.name }}</option></optgroup><optgroup label="未配置"><option v-for="option in categoryOptions.filter(item => !item.name)" :key="option.value" :value="option.value">{{ option.value }}</option></optgroup></select></label>
        <label>助记位 BB<select v-model="form.tag_b" required :disabled="!form.tag_a"><option disabled value="">请选择</option><optgroup label="已配置"><option v-for="option in mnemonicOptions.filter(item => item.name)" :key="option.value" :value="option.value">{{ option.value }} — {{ option.name }}</option></optgroup><optgroup label="未配置"><option v-for="option in mnemonicOptions.filter(item => !item.name)" :key="option.value" :value="option.value">{{ option.value }}</option></optgroup></select></label>
        <label>标记位 CC<input :value="form.tag_c" required inputmode="numeric" maxlength="2" autocomplete="off" @keydown="guardDigitKey" @input="inputTwoDigits('tag_c', $event)" @blur="formatTwoDigits('tag_c')" /></label>
        <label>数量<input v-model.number="form.quantity" required type="number" min="0" step="any" autocomplete="off" /></label>
        <label>单位<input v-model="form.unit" maxlength="24" autocomplete="off" /></label>
        <label class="wide parent-field">父容器（可选）<span class="parent-picker"><input v-model="form.parent" autocomplete="off" placeholder="输入名称、完整编号或六位序列号" @focus="parentFocused = true" @blur="parentFocused = false" /><span v-if="parentFocused && form.parent.trim()" class="parent-suggestions"><span v-if="parentSearchLoading" class="parent-suggestion-status">正在查询…</span><button v-for="container in containers" v-else :key="container.serial" type="button" @mousedown.prevent="selectParent(container)"><strong>{{ container.name }}</strong><code>{{ container.code }}</code></button><span v-if="!parentSearchLoading && !containers.length" class="parent-suggestion-status">没有匹配的容器</span></span></span></label>
      </div>
      <section class="image-picker">
        <div class="image-preview">
          <img v-if="previewUrl" :src="previewUrl" alt="新图片预览" />
          <img v-else-if="element?.has_image && !removeExistingImage" :src="`/images/${element.serial}?v=${element.updated_at}`" :alt="element.name" />
          <span v-else>暂无图片</span>
        </div>
        <div><label class="button ghost file-button">选择图片<input ref="fileInput" type="file" accept="image/jpeg,image/png,image/webp,image/gif" @change="image = ($event.target as HTMLInputElement).files?.[0] || null" /></label><button v-if="element?.has_image && !image" type="button" class="button danger-ghost" @click="removeExistingImage = !removeExistingImage">{{ removeExistingImage ? '保留原图' : '移除原图' }}</button><p class="hint">JPEG / PNG / WebP / GIF，图片仅作小尺寸预览</p></div>
      </section>
      <p v-if="error" class="error-message">{{ error }}</p>
      <div v-if="!element" class="form-settings">
        <label class="continue-setting">继续添加后 CC<select v-model="continueCcBehavior"><option value="keep">保留当前值</option><option value="reset">清零为 00</option><option value="increment">自增 1（99 后回到 00）</option></select></label>
        <label class="continue-setting">保存后打印<select v-model="printAfterSave"><option value="none">不打印</option><option value="A1">打印 A1 标签</option><option value="A2">打印 A2 标签</option></select></label>
      </div>
      <div class="modal-actions"><button type="button" class="button ghost" @click="$emit('close')">取消</button><button v-if="!element" type="button" class="button ghost continue-button" :disabled="busy" @click="submit(true)">{{ busy ? '保存中…' : '继续添加' }}</button><button class="button primary" :disabled="busy">{{ busy ? '保存中…' : '保存条目' }}</button></div>
      <ToastNotice :message="toastMessage" />
    </form>
  </div>
</template>
