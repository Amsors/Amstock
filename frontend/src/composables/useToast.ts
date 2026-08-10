import { onBeforeUnmount, ref } from 'vue'
import { UI_CONFIG } from '../uiConfig'

export function useToast(defaultDurationMs = UI_CONFIG.toastVisibleMs) {
  const toastMessage = ref('')
  let hideTimer = 0

  function showToast(message: string, durationMs = defaultDurationMs) {
    window.clearTimeout(hideTimer)
    toastMessage.value = message
    hideTimer = window.setTimeout(() => { toastMessage.value = '' }, durationMs)
  }

  onBeforeUnmount(() => window.clearTimeout(hideTimer))
  return { toastMessage, showToast }
}

