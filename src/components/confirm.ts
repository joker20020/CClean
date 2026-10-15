/**
 * 全局模态确认框（替代 window.confirm / window.alert）。
 *
 * 用法：
 *   import { confirmDialog } from "../components/confirm";
 *   const ok = await confirmDialog({
 *     title: t("..."), message: t("..."),
 *     confirmText: t("common.ok"), cancelText: t("common.cancel"),
 *     danger: true,
 *   });
 *
 * 组件 ConfirmModal.vue 需要在应用根组件中挂载一次；
 * 多个确认请求会排队依次展示。
 */
import { reactive } from "vue";

export interface ConfirmOptions {
  title: string;
  message: string;
  confirmText: string;
  cancelText?: string;
  /** 危险操作（如删除/永久清理）：确认按钮使用红色，初始焦点落在取消按钮上 */
  danger?: boolean;
}

interface PendingEntry {
  opts: ConfirmOptions;
  resolve: (value: boolean) => void;
}

export const confirmState = reactive({
  visible: false,
  title: "",
  message: "",
  confirmText: "",
  cancelText: "",
  danger: false,
  /** 仅由 ConfirmModal 内部使用 */
  resolve: null as ((value: boolean) => void) | null,
});

const queue: PendingEntry[] = [];

function open(entry: PendingEntry) {
  confirmState.title = entry.opts.title;
  confirmState.message = entry.opts.message;
  confirmState.confirmText = entry.opts.confirmText;
  confirmState.cancelText = entry.opts.cancelText ?? "";
  confirmState.danger = !!entry.opts.danger;
  confirmState.resolve = entry.resolve;
  confirmState.visible = true;
}

/** 弹出模态确认框；返回 Promise<boolean>，用户确认时 resolve(true)。 */
export function confirmDialog(opts: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    const entry: PendingEntry = { opts, resolve };
    if (confirmState.visible) {
      queue.push(entry);
    } else {
      open(entry);
    }
  });
}

/** 由 ConfirmModal 调用：关闭当前弹窗并处理队列中的下一个。 */
export function settleConfirm(accepted: boolean) {
  const resolve = confirmState.resolve;
  confirmState.visible = false;
  confirmState.resolve = null;
  resolve?.(accepted);
  const next = queue.shift();
  if (next) open(next);
}
