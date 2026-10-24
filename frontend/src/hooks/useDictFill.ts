import { useCallback, type Dispatch, type SetStateAction } from 'react'
import type { Definition, DictEntry } from '../api'

/** 词典回填方式：覆盖全部 / 仅音标 / 追加释义 / 追加例句 */
export type DictApplyMode = 'overwrite' | 'phonetic' | 'definitions' | 'example'

/** 回填结果：added = 真正写入的条数，skipped = 因重复被跳过的条数（文案必须按真实结果说） */
export interface DictFillResult {
  added: number
  skipped: number
}

/** 词典例句 → 表单例句文本（有译文时换行拼接，两条回填路径共用）。 */
export function exampleText(entry: DictEntry): string {
  const example = entry.examples[0]
  if (!example) return ''
  return example.translation ? `${example.text}\n${example.translation}` : example.text
}

function definitionKey(definition: Definition): string {
  return `${definition.pos}\u0000${definition.meaning}`
}

interface Params {
  phonetic: string
  definitions: Definition[]
  example: string
  setPhonetic: Dispatch<SetStateAction<string>>
  setDefinitions: Dispatch<SetStateAction<Definition[]>>
  setExample: Dispatch<SetStateAction<string>>
}

/**
 * 词典回填：由「词典搜索」弹窗触发，明确区分覆盖与追加。
 *
 * 不自动抓取/自动填入：只有用户在弹窗里点击操作才会修改表单，
 * 避免输入过程中内容被悄悄替换。
 */
export function useDictFill({
  phonetic,
  definitions,
  example,
  setPhonetic,
  setDefinitions,
  setExample,
}: Params) {
  const hasContent =
    phonetic.trim() !== '' ||
    example.trim() !== '' ||
    definitions.some((item) => item.meaning.trim() !== '' || item.pos !== '')

  const applyFill = useCallback(
    (entry: DictEntry, mode: DictApplyMode): DictFillResult => {
      switch (mode) {
        case 'overwrite': {
          let added = 0
          if (entry.phonetic) {
            setPhonetic(entry.phonetic)
            added += 1
          }
          if (entry.definitions.length > 0) {
            setDefinitions(entry.definitions)
            added += 1
          }
          const text = exampleText(entry)
          if (text) {
            setExample(text)
            added += 1
          }
          return { added, skipped: 0 }
        }
        case 'phonetic': {
          if (!entry.phonetic) return { added: 0, skipped: 0 }
          // 与表单现值相同：不写入、不报「已填入」
          if (phonetic.trim() === entry.phonetic.trim()) return { added: 0, skipped: 1 }
          setPhonetic(entry.phonetic)
          return { added: 1, skipped: 0 }
        }
        case 'definitions': {
          if (entry.definitions.length === 0) return { added: 0, skipped: 0 }
          const base =
            definitions.length === 1 && definitions[0].meaning.trim() === '' && definitions[0].pos === ''
              ? []
              : definitions
          const seen = new Set(base.map(definitionKey))
          const fresh = entry.definitions.filter((item) => !seen.has(definitionKey(item)))
          if (fresh.length > 0) setDefinitions([...base, ...fresh])
          return { added: fresh.length, skipped: entry.definitions.length - fresh.length }
        }
        case 'example': {
          const text = exampleText(entry)
          if (!text) return { added: 0, skipped: 0 }
          // 例句已存在时不重复追加：整段相同，或词条的每一行（英文/中文）都已在表单里。
          // 注意不能拿整段「英文\n中文」去和单行数组比（永远 false，会无限追加重复例句）。
          const incoming = text.split('\n').map((line) => line.trim()).filter(Boolean)
          const existing = example.split('\n').map((line) => line.trim()).filter(Boolean)
          const duplicated =
            example.trim() === text.trim() ||
            (incoming.length > 0 && incoming.every((line) => existing.includes(line)))
          if (duplicated) return { added: 0, skipped: 1 }
          setExample(example.trim() ? `${example}\n${text}` : text)
          return { added: 1, skipped: 0 }
        }
      }
    },
    [phonetic, definitions, example, setPhonetic, setDefinitions, setExample],
  )

  return { hasContent, applyFill }
}
