<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { confirmState, settleConfirm } from "./confirm";

const confirmBtn = ref<HTMLButtonElement | null>(null);
const cancelBtn = ref<HTMLButtonElement | null>(null);

function accept() {
  settleConfirm(true);
}
function cancel() {
  settleConfirm(false);
}

function onKeydown(e: KeyboardEvent) {
  if (!confirmState.visible) return;
  if (e.key === "Escape") {
    e.preventDefault();
    cancel();
  }
}

// 打开后聚焦：危险操作聚焦取消按钮，避免误触；否则聚焦确认按钮
watch(
  () => confirmState.visible,
  async (visible) => {
    if (visible) {
      window.addEventListener("keydown", onKeydown);
      await nextTick();
      (confirmState.danger ? cancelBtn.value : confirmBtn.value)?.focus();
    } else {
      window.removeEventListener("keydown", onKeydown);
    }
  },
);
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="confirmState.visible"
        class="modal-mask"
        @click.self="cancel"
      >
        <div
          class="modal-dialog"
          role="alertdialog"
          aria-modal="true"
          :aria-label="confirmState.title"
        >
          <div class="modal-title">{{ confirmState.title }}</div>
          <div class="modal-message">{{ confirmState.message }}</div>
          <div class="modal-actions">
            <button
              ref="cancelBtn"
              v-if="confirmState.cancelText"
              class="btn btn-ghost"
              @click="cancel"
            >
              {{ confirmState.cancelText }}
            </button>
            <button
              ref="confirmBtn"
              class="btn"
              :class="confirmState.danger ? 'btn-danger' : 'btn-primary'"
              @click="accept"
            >
              {{ confirmState.confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-mask {
  position: fixed;
  inset: 0;
  z-index: 1000;
  background: rgba(15, 23, 42, 0.45);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
}
.modal-dialog {
  width: 100%;
  max-width: 420px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 14px;
  box-shadow: var(--shadow-pop);
  padding: 22px 24px 18px;
}
.modal-title {
  font-size: 15.5px;
  font-weight: 700;
  color: var(--text);
}
.modal-message {
  margin-top: 10px;
  font-size: 13px;
  line-height: 1.7;
  color: var(--text-2);
  white-space: pre-line;
  word-break: break-word;
}
.modal-actions {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
.btn-danger {
  background: var(--red);
  color: #fff;
  border: 1px solid var(--red);
}
.btn-danger:hover {
  filter: brightness(1.06);
}

/* 进出动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.16s ease;
}
.modal-enter-active .modal-dialog,
.modal-leave-active .modal-dialog {
  transition: transform 0.16s ease, opacity 0.16s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
.modal-enter-from .modal-dialog,
.modal-leave-to .modal-dialog {
  transform: translateY(10px) scale(0.97);
  opacity: 0;
}
</style>
