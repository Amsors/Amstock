<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api'
import type { DeletePreview, StockElement } from '../types'

const props = defineProps<{ element: StockElement }>()
const emit = defineEmits<{ close: []; deleted: [] }>()
const preview = ref<DeletePreview[]>([])
const loading = ref(true)
const busy = ref(false)
const error = ref('')
const mode = ref<'move_to_parent' | 'move_to_container' | 'cascade'>('move_to_parent')
const target = ref('')
const cascadeConfirmed = ref(false)

onMounted(async () => {
  try { preview.value = await api.deletePreview(props.element.serial) }
  catch (e) { error.value = (e as Error).message }
  finally { loading.value = false }
})

async function remove() {
  if (preview.value.length && mode.value === 'cascade' && !cascadeConfirmed.value) { error.value = '请确认已查看递归删除清单'; return }
  busy.value = true; error.value = ''
  try {
    const targetSerial = mode.value === 'move_to_container' ? Number(target.value.trim().slice(-6)) : undefined
    if (mode.value === 'move_to_container' && (!/^([A-Za-z]-\d{2}-\d{2}-)?\d{6}$/.test(target.value.trim()))) throw new Error('请输入有效的目标容器编号')
    await api.remove(props.element.serial, preview.value.length ? mode.value : undefined, targetSerial)
    emit('deleted')
  } catch (e) { error.value = (e as Error).message }
  finally { busy.value = false }
}
</script>

<template>
  <div class="modal-backdrop" @mousedown.self="$emit('close')">
    <div class="modal delete-modal">
      <div class="modal-header"><div><p class="eyebrow">移除条目</p><h2>{{ element.name }}</h2><code>{{ element.code }}</code></div><button class="icon-button" @click="$emit('close')">×</button></div>
      <p v-if="loading" class="empty-state">正在检查容器内容…</p>
      <template v-else-if="preview.length">
        <p>该容器包含 {{ preview.length }} 个后代元素。请选择移动方式：</p>
        <div class="radio-stack">
          <label><input v-model="mode" type="radio" value="move_to_parent" /><span><strong>移动到上一级</strong><small>没有上一级时，子元素将成为孤立元素</small></span></label>
          <label><input v-model="mode" type="radio" value="move_to_container" /><span><strong>移动到已有容器</strong><small>所有直接子元素将移入指定容器</small></span></label>
          <input v-if="mode === 'move_to_container'" v-model="target" class="indented-input" placeholder="目标容器完整编号或六位序列号" />
          <label><input v-model="mode" type="radio" value="cascade" /><span><strong>递归删除全部内容</strong><small>所有下列后代会被软删除，序列号仍可恢复</small></span></label>
        </div>
        <div v-if="mode === 'cascade'" class="dfs-preview"><p class="eyebrow">DFS 递归删除清单</p><ol><li v-for="entry in preview" :key="entry.serial" :style="{ marginLeft: `${(entry.depth - 1) * 18}px` }"><span>{{ entry.kind === 'container' ? '▣' : '·' }}</span> {{ entry.name }} <code>{{ entry.code }}</code></li></ol><label class="confirm-check"><input v-model="cascadeConfirmed" type="checkbox" />我已检查以上全部条目</label></div>
      </template>
      <p v-else class="notice">该条目没有子元素，将直接移入“已删除”列表。</p>
      <p v-if="error" class="error-message">{{ error }}</p>
      <div class="modal-actions"><button class="button ghost" @click="$emit('close')">取消</button><button class="button danger" :disabled="busy || loading" @click="remove">{{ busy ? '处理中…' : '确认移除' }}</button></div>
    </div>
  </div>
</template>
