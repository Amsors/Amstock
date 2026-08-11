import { onBeforeUnmount, ref, watch } from 'vue'

export interface ElementSearchOptions<T> {
  load: (query: string, includeDeleted: boolean) => Promise<T[]>
  initialQuery?: string
  initialIncludeDeleted?: boolean
  onFiltersChanged?: (query: string, includeDeleted: boolean) => void
}

/** Shared search state for both editable and read-only result pages. */
export function useElementSearch<T>(options: ElementSearchOptions<T>) {
  const query = ref(options.initialQuery ?? '')
  const includeDeleted = ref(options.initialIncludeDeleted ?? false)
  const elements = ref<T[]>([])
  const loading = ref(false)
  const error = ref('')
  let timer = 0
  let requestId = 0

  async function search() {
    const currentRequest = ++requestId
    loading.value = true
    error.value = ''
    try {
      const result = await options.load(query.value, includeDeleted.value)
      if (currentRequest === requestId) elements.value = result
    } catch (cause) {
      if (currentRequest === requestId) error.value = (cause as Error).message
    } finally {
      if (currentRequest === requestId) loading.value = false
    }
  }

  function filtersChanged() {
    options.onFiltersChanged?.(query.value, includeDeleted.value)
    window.clearTimeout(timer)
    timer = window.setTimeout(search, 220)
  }

  watch([query, includeDeleted], filtersChanged)
  onBeforeUnmount(() => {
    window.clearTimeout(timer)
    requestId++
  })

  return { query, includeDeleted, elements, loading, error, search }
}
