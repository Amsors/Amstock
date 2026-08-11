import type { CategoryMapping, DeletePreview, ElementInput, LabelStyle, MnemonicMapping, PrintResult, StockElement, TreeNode } from './types'

async function request<T>(url: string, init?: RequestInit, notifyUnauthorized = true): Promise<T> {
  const response = await fetch(url, { credentials: 'same-origin', ...init })
  if (!response.ok) {
    if (response.status === 401 && notifyUnauthorized) {
      window.dispatchEvent(new Event('amstock:unauthorized'))
    }
    let message = `请求失败 (${response.status})`
    try { message = (await response.json()).error || message } catch { /* 非 JSON 错误 */ }
    throw new Error(message)
  }
  if (response.status === 204) return undefined as T
  return response.json() as Promise<T>
}

const json = (method: string, body?: unknown): RequestInit => ({
  method,
  headers: { 'Content-Type': 'application/json' },
  body: body === undefined ? undefined : JSON.stringify(body),
})

export const api = {
  session: () => request<{ username: string }>('/api/auth/session', undefined, false),
  login: (username: string, password: string) =>
    request<{ username: string }>('/api/auth/login', json('POST', { username, password }), false),
  logout: () => request<void>('/api/auth/logout', json('POST'), false),
  search: (q = '', includeDeleted = false) => request<StockElement[]>(`/api/elements?q=${encodeURIComponent(q)}&include_deleted=${includeDeleted}`),
  create: (data: ElementInput) => request<StockElement>('/api/elements', json('POST', data)),
  update: (serial: number, data: ElementInput) => request<StockElement>(`/api/elements/${serial}`, json('PUT', data)),
  restore: (serial: number) => request<StockElement>(`/api/elements/${serial}/restore`, json('POST')),
  printLabel: (serial: number, style: LabelStyle) => request<PrintResult>(`/api/elements/${serial}/print`, json('POST', { style })),
  remove: (serial: number, mode?: string, target_serial?: number) =>
    request<{ deleted: number }>(`/api/elements/${serial}`, json('DELETE', { mode, target_serial })),
  deletePreview: (serial: number) => request<DeletePreview[]>(`/api/elements/${serial}/delete-preview`),
  tree: () => request<TreeNode[]>('/api/tree'),
  uploadImage: (serial: number, file: File) => request<void>(`/api/elements/${serial}/image`, { method: 'PUT', headers: { 'Content-Type': file.type }, body: file }),
  deleteImage: (serial: number) => request<void>(`/api/elements/${serial}/image`, { method: 'DELETE' }),
  categories: () => request<CategoryMapping[]>('/api/mappings/categories'),
  saveCategory: (tag: string, name: string | null) => request<CategoryMapping>(`/api/mappings/categories/${tag}`, json('PUT', { name })),
  deleteCategory: (tag: string) => request<void>(`/api/mappings/categories/${tag}`, { method: 'DELETE' }),
  mnemonics: (tag: string) => request<MnemonicMapping[]>(`/api/mappings/categories/${tag}/mnemonics`),
  saveMnemonic: (tag: string, value: number, name: string | null) => request<MnemonicMapping>(`/api/mappings/categories/${tag}/mnemonics/${value}`, json('PUT', { name })),
  deleteMnemonic: (tag: string, value: number) => request<void>(`/api/mappings/categories/${tag}/mnemonics/${value}`, { method: 'DELETE' }),
}

export function serialFromCode(value: string): number | null {
  const trimmed = value.trim()
  if (!trimmed) return null
  const match = trimmed.match(/(?:^|-)\d{6}$/)
  if (!match) throw new Error('父容器请输入六位序列号或完整编号')
  return Number(trimmed.slice(-6))
}
