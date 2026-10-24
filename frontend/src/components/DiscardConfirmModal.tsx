import ConfirmModal from './ConfirmModal'

interface Props {
  onDiscard: () => void
  onCancel: () => void
  /** 标题（默认放弃未保存的更改？） */
  title?: string
  /** 说明文案 */
  message?: string
}

/** 未保存更改的放弃确认框：与项目视觉一致，供所有编辑弹窗复用。 */
export default function DiscardConfirmModal({
  onDiscard,
  onCancel,
  title = '放弃未保存的更改？',
  message = '关闭后本次修改将不会保存。',
}: Props) {
  return (
    <ConfirmModal
      title={title}
      message={message}
      confirmText="放弃更改"
      onConfirm={onDiscard}
      onCancel={onCancel}
    />
  )
}
