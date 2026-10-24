import type { Definition } from './api'

/** 词典抓取结果带入「添加单词」表单时的初始草稿（不含标签）。 */
export interface WordDraft {
  spelling: string
  phonetic?: string
  definitions: Definition[]
  example?: string
}
