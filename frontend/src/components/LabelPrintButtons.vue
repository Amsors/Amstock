<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api'
import { useToast } from '../composables/useToast'
import type { LabelStyle, StockElement } from '../types'
import ToastNotice from './ToastNotice.vue'

const props = defineProps<{ element: StockElement }>()
const printing = ref<LabelStyle | null>(null)
const { toastMessage, showToast } = useToast()

const styles: LabelStyle[] = ['A1', 'A2', 'B1', 'B2']

async function print(style: LabelStyle) {
  printing.value = style
  try {
    const result = await api.printLabel(props.element.serial, style)
    showToast(result.mode === 'preview' ? `${style} 标签预览已生成并打开。` : `${style} 标签已发送到打印机。`)
  } catch (error) {
    showToast(`打印失败：${(error as Error).message}`)
  } finally {
    printing.value = null
  }
}
</script>

<template>
  <div class="label-print-actions" :class="{ 'item-label-actions': element.kind === 'item' }" aria-label="打印标签">
    <button
      v-for="style in styles"
      v-show="element.kind === 'container' || style === 'A1' || style === 'A2'"
      :key="style"
      type="button"
      class="button ghost small print-label-button"
      :disabled="printing !== null"
      :aria-label="printing === style ? `${style} 标签打印中` : `打印 ${style} 标签`"
      @click.stop="print(style)"
    ><span class="print-label-prefix">{{ printing === style ? '打印中' : '打印' }}</span><span>{{ printing === style ? '…' : style }}</span></button>
  </div>
  <ToastNotice :message="toastMessage" />
</template>
