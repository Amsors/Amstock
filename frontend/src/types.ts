export type ElementKind = 'item' | 'container'
export type LabelStyle = 'A1' | 'A2' | 'B1' | 'B2'

export interface StockElement {
  serial: number
  code: string
  kind: ElementKind
  tag_a: string
  tag_b: number
  tag_c: number
  name: string
  description: string
  quantity: number
  unit: string
  parent_serial: number | null
  image_mime: string | null
  has_image: boolean
  created_at: string
  updated_at: string
  deleted_at: string | null
}

export type ElementInput = Pick<
  StockElement,
  'kind' | 'tag_a' | 'tag_b' | 'tag_c' | 'name' | 'description' | 'quantity' | 'unit' | 'parent_serial'
>

export interface TreeNode extends StockElement { children: TreeNode[] }
export interface ElementLookup { element: StockElement; path: StockElement[] }
export interface CategoryMapping { tag_a: string; name: string | null }
export interface MnemonicMapping { tag_a: string; tag_b: number; name: string | null }
export interface DeletePreview extends StockElement { depth: number }
export interface PrintResult {
  schema_version: number
  mode: 'preview' | 'printer'
  style: LabelStyle
  identifier: string
  output?: string
}
