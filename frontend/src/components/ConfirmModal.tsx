import Modal from './Modal'

interface Props {
  title: string
  message: string
  /** 确认按钮文案（默认“确认删除”） */
  confirmText?: string
  onConfirm: () => void
  onCancel: () => void
}

/** 确认弹窗：替换原生 window.confirm，与项目视觉一致 */
export default function ConfirmModal({
  title,
  message,
  confirmText = '确认删除',
  onConfirm,
  onCancel,
}: Props) {
  return (
    <Modal onClose={onCancel} maxWidth="max-w-sm">
      <h2 className="font-serif text-xl text-charcoal">{title}</h2>
      <p className="mt-3 text-sm text-charcoal/60 leading-relaxed">{message}</p>
      <div className="mt-8 flex justify-end gap-3">
        <button
          onClick={onCancel}
          className="px-5 py-2 rounded-full border border-charcoal/20 text-charcoal text-sm font-medium hover:bg-white hover:border-charcoal/40 transition-all"
        >
          取消
        </button>
        <button
          onClick={onConfirm}
          className="px-5 py-2 rounded-full bg-clay text-ivory text-sm font-medium hover:bg-clay/90 transition-all shadow-lg shadow-clay/20"
        >
          {confirmText}
        </button>
      </div>
    </Modal>
  )
}
