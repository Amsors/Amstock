<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { api } from '../api'
import { useToast } from '../composables/useToast'
import type { CategoryMapping, MnemonicMapping } from '../types'
import ToastNotice from './ToastNotice.vue'

const categories = ref<CategoryMapping[]>([])
const selected = ref('')
const mnemonics = ref<MnemonicMapping[]>([])
const newTag = ref('')
const newCategoryName = ref('')
const newMnemonic = ref(0)
const newMnemonicName = ref('')
const error = ref('')
const { toastMessage, showToast } = useToast()

async function loadCategories(select?: string) {
  categories.value = await api.categories()
  selected.value = select || selected.value || categories.value[0]?.tag_a || ''
  if (selected.value) await loadMnemonics()
}
async function loadMnemonics() { mnemonics.value = selected.value ? await api.mnemonics(selected.value) : [] }
async function action(run: () => Promise<void>, notify = true) {
  error.value = ''
  try { await run(); if (notify) showToast('已保存') } catch (e) { error.value = (e as Error).message }
}
async function addCategory() {
  await action(async () => {
    const tag = newTag.value.trim().toUpperCase()
    await api.saveCategory(tag, newCategoryName.value || null)
    newTag.value = ''; newCategoryName.value = ''
    await loadCategories(tag)
  })
}
async function saveCategory(category: CategoryMapping) {
  await action(async () => { await api.saveCategory(category.tag_a, category.name); await loadCategories(category.tag_a) })
}
async function removeCategory(category: CategoryMapping) {
  if (!confirm(`删除类别标记 ${category.tag_a} 的映射？`)) return
  await action(async () => { await api.deleteCategory(category.tag_a); selected.value = ''; await loadCategories() })
}
async function addMnemonic() {
  await action(async () => {
    await api.saveMnemonic(selected.value, Number(newMnemonic.value), newMnemonicName.value || null)
    newMnemonic.value = 0; newMnemonicName.value = ''; await loadMnemonics()
  })
}
async function saveMnemonic(row: MnemonicMapping) {
  await action(async () => { await api.saveMnemonic(row.tag_a, row.tag_b, row.name); await loadMnemonics() })
}
async function removeMnemonic(row: MnemonicMapping) {
  if (!confirm(`删除 ${row.tag_a}-${String(row.tag_b).padStart(2, '0')} 的映射？`)) return
  await action(async () => { await api.deleteMnemonic(row.tag_a, row.tag_b); await loadMnemonics() })
}
async function selectCategory(tag: string) { selected.value = tag; await action(loadMnemonics, false) }
onMounted(() => action(() => loadCategories(), false))
</script>

<template>
  <section class="panel mapping-panel">
    <div class="section-header"><div><p class="eyebrow">编号字典</p><h2>标签映射管理</h2></div></div>
    <p class="section-intro">每个类别标记可对应一个名称，并拥有独立的助记标记表。名称均可留空；编号中的第三段数字不维护名称映射。</p>
    <div class="mapping-layout">
      <div class="mapping-column">
        <h3>类别标记</h3>
        <form class="inline-form" @submit.prevent="addCategory"><input v-model="newTag" class="short-code" required maxlength="1" pattern="[A-Za-z]" placeholder="字母" /><input v-model="newCategoryName" placeholder="名称（可留空）" /><button class="button primary small">添加</button></form>
        <div class="mapping-list">
          <div v-for="category in categories" :key="category.tag_a" class="mapping-row" :class="{ selected: selected === category.tag_a }" @click="selectCategory(category.tag_a)">
            <code>{{ category.tag_a }}</code><input v-model="category.name" placeholder="未命名" @click.stop /><button class="text-button" @click.stop="saveCategory(category)">保存</button><button class="text-button danger-text" @click.stop="removeCategory(category)">删除</button>
          </div>
          <p v-if="!categories.length" class="empty-state compact">暂无类别标记</p>
        </div>
      </div>
      <div class="mapping-column">
        <h3>助记标记 <span v-if="selected">— 类别 {{ selected }}</span></h3>
        <template v-if="selected">
          <form class="inline-form" @submit.prevent="addMnemonic"><input v-model.number="newMnemonic" class="short-code" required type="number" min="0" max="99" /><input v-model="newMnemonicName" placeholder="名称（可留空）" /><button class="button primary small">添加</button></form>
          <div class="mapping-list">
            <div v-for="row in mnemonics" :key="row.tag_b" class="mapping-row"><code>{{ String(row.tag_b).padStart(2, '0') }}</code><input v-model="row.name" placeholder="未命名" /><button class="text-button" @click="saveMnemonic(row)">保存</button><button class="text-button danger-text" @click="removeMnemonic(row)">删除</button></div>
            <p v-if="!mnemonics.length" class="empty-state compact">该类别还没有助记标记</p>
          </div>
        </template>
        <p v-else class="empty-state">请先添加或选择类别标记</p>
      </div>
    </div>
    <p v-if="error" class="error-message">{{ error }}</p>
    <ToastNotice :message="toastMessage" />
  </section>
</template>
