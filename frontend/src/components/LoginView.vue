<script setup lang="ts">
import { ref } from 'vue'
import { api } from '../api'

const emit = defineEmits<{ authenticated: [username: string] }>()
const username = ref('admin')
const password = ref('')
const submitting = ref(false)
const error = ref('')

async function submit() {
  if (!username.value || !password.value || submitting.value) return
  submitting.value = true
  error.value = ''
  try {
    const session = await api.login(username.value, password.value)
    password.value = ''
    emit('authenticated', session.username)
  } catch (cause) {
    error.value = (cause as Error).message
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <main class="login-page">
    <section class="login-card panel">
      <div class="login-brand"><span class="brand-mark">A</span><span><strong>Amstock</strong><small>家用物资管理</small></span></div>
      <h1>登录</h1>
      <p>请输入此 Amstock 实例的单用户凭据。</p>
      <form @submit.prevent="submit">
        <label>用户名<input v-model.trim="username" name="username" autocomplete="username" autofocus required /></label>
        <label>密码<input v-model="password" name="password" type="password" autocomplete="current-password" required /></label>
        <p v-if="error" class="error-message">{{ error }}</p>
        <button class="button primary" type="submit" :disabled="submitting || !username || !password">{{ submitting ? '正在登录…' : '登录' }}</button>
      </form>
    </section>
  </main>
</template>
