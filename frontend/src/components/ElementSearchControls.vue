<script setup lang="ts">
defineProps<{ readonly?: boolean; loading?: boolean; autofocus?: boolean }>()
const query = defineModel<string>('query', { required: true })
const includeDeleted = defineModel<boolean>('includeDeleted', { required: true })
defineEmits<{ create: [] }>()
</script>

<template>
  <section class="search-panel panel">
    <label class="search-box">
      <span>⌕</span>
      <input v-model="query" placeholder="输入名称、描述或编号，例如：电阻、M-03、000042" :autofocus="autofocus" />
      <button v-if="query" type="button" aria-label="清空" @click="query = ''">×</button>
    </label>
    <div class="search-controls">
      <label class="deleted-toggle"><input v-model="includeDeleted" type="checkbox" />包含已删除条目</label>
      <button v-if="!readonly" class="button primary create-button" type="button" @click="$emit('create')"><span>＋</span> 添加物资</button>
      <span v-else-if="loading" class="search-status" aria-live="polite">正在查找…</span>
    </div>
  </section>
</template>
