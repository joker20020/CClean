<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  customRulesDir,
  deleteCustomRule,
  getRules,
  listCustomRules,
  openPath,
  saveCustomRule,
  validateCustomRule,
  type RuleInfo,
  type UserRuleFile,
} from "../api";
import { useCategoryNames } from "../i18n";
import { confirmDialog } from "../components/confirm";

const { t, locale } = useI18n();
const categoryNames = useCategoryNames();

const builtinRules = ref<RuleInfo[]>([]);
const userFiles = ref<UserRuleFile[]>([]);
const rulesDir = ref("");
const errorMsg = ref("");

// 编辑器状态
const editingFilename = ref("");
const editorContent = ref("");
const isEditing = ref(false);
const validateMsg = ref("");
const validateOk = ref(false);
const saving = ref(false);

const TEMPLATE = computed(() => t("rules.template"));

const groupedBuiltin = computed(() => {
  const groups = new Map<string, RuleInfo[]>();
  for (const r of builtinRules.value) {
    if (!groups.has(r.category)) groups.set(r.category, []);
    groups.get(r.category)!.push(r);
  }
  return [...groups.entries()].map(([cat, list]) => ({
    category: cat,
    name: categoryNames.value[cat] ?? cat,
    list,
  }));
});

async function loadAll() {
  try {
    const [rules, files, dir] = await Promise.all([
      getRules(),
      listCustomRules(),
      customRulesDir(),
    ]);
    builtinRules.value = rules;
    userFiles.value = files;
    rulesDir.value = dir;
  } catch (e) {
    errorMsg.value = String(e);
  }
}

function newRule() {
  isEditing.value = true;
  editingFilename.value = "";
  editorContent.value = TEMPLATE.value;
  validateMsg.value = "";
  validateOk.value = false;
}

function editFile(f: UserRuleFile) {
  isEditing.value = true;
  editingFilename.value = f.filename;
  editorContent.value = f.content;
  validateMsg.value = "";
  validateOk.value = false;
}

async function validate() {
  try {
    const ids = await validateCustomRule(editorContent.value);
    validateOk.value = true;
    validateMsg.value = t("rules.editor.validateOk", {
      n: ids.length,
      ids: ids.join(locale.value === "zh-CN" ? "、" : ", "),
    });
  } catch (e) {
    validateOk.value = false;
    validateMsg.value = String(e);
  }
}

async function save() {
  saving.value = true;
  try {
    await saveCustomRule(editingFilename.value, editorContent.value);
    await loadAll();
    isEditing.value = false;
    validateMsg.value = "";
  } catch (e) {
    validateOk.value = false;
    validateMsg.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function remove(filename: string) {
  const ok = await confirmDialog({
    title: t("rules.editor.deleteTitle"),
    message: t("rules.editor.deleteConfirm", { name: filename }),
    confirmText: t("common.delete"),
    cancelText: t("common.cancel"),
    danger: true,
  });
  if (!ok) return;
  try {
    await deleteCustomRule(filename);
    if (editingFilename.value === filename) {
      isEditing.value = false;
    }
    await loadAll();
  } catch (e) {
    errorMsg.value = String(e);
  }
}

function openDir() {
  if (rulesDir.value) openPath(rulesDir.value).catch((e) => (errorMsg.value = String(e)));
}

onMounted(loadAll);
</script>

<template>
  <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>

  <div class="layout">
    <!-- 左：规则列表 -->
    <div class="panel list-panel">
      <div class="list-head">
        <b>{{ t("rules.title") }}</b>
        <button class="btn btn-primary btn-sm" @click="newRule">{{ t("rules.newRule") }}</button>
      </div>

      <div class="section">
        <div class="section-title">
          <span>{{ t("rules.myRules", { n: userFiles.length }) }}</span>
          <button class="link-btn" @click="openDir">{{ t("common.openDir") }}</button>
        </div>
        <div v-if="userFiles.length === 0" class="muted empty">
          {{ t("rules.noUserRules") }}
        </div>
        <div
          v-for="f in userFiles"
          :key="f.filename"
          class="user-row"
          :class="{ active: editingFilename === f.filename }"
        >
          <button class="file-btn" @click="editFile(f)">
            <svg viewBox="0 0 20 20" width="14" height="14" class="file-icon">
              <path d="M5 2.5h6.5L16 7v10.5H5z" fill="none" stroke="currentColor"
                stroke-width="1.4" stroke-linejoin="round" />
              <path d="M11.5 2.5V7H16" fill="none" stroke="currentColor"
                stroke-width="1.4" stroke-linejoin="round" />
            </svg>
            {{ f.filename }}
          </button>
          <button class="del-btn" @click="remove(f.filename)">{{ t("common.delete") }}</button>
        </div>
      </div>

      <div class="section">
        <div class="section-title"><span>{{ t("rules.builtinRules", { n: builtinRules.length }) }}</span></div>
        <div v-for="g in groupedBuiltin" :key="g.category" class="builtin-group">
          <details>
            <summary>
              {{ g.name }}
              <span class="builtin-count">{{ g.list.length }}</span>
            </summary>
            <div v-for="r in g.list" :key="r.id" class="builtin-row">
              <div class="builtin-line">
                <span class="tag" :class="`tag-${r.risk}`">{{ r.risk }}</span>
                <span class="rule-name">{{ r.name }}</span>
                <span v-if="r.action === 'report'" class="tag tag-L0">{{ t("rules.reportOnly") }}</span>
              </div>
              <div class="rule-id muted">{{ r.id }}<span v-if="r.description"> · {{ r.description }}</span></div>
            </div>
          </details>
        </div>
      </div>
    </div>

    <!-- 右：编辑器 -->
    <div class="panel editor-panel">
      <template v-if="isEditing">
        <div class="editor-head">
          <div>
            <b>{{ editingFilename ? t("rules.editor.editTitle") : t("rules.editor.newTitle") }}</b>
            <div class="muted editor-sub">
              {{ t("rules.editor.sub") }}
            </div>
          </div>
          <div class="editor-actions">
            <button class="btn btn-ghost btn-sm" @click="validate">{{ t("rules.editor.validate") }}</button>
            <button class="btn btn-primary btn-sm" :disabled="saving" @click="save">
              {{ saving ? t("rules.editor.saving") : t("common.save") }}
            </button>
            <button class="btn btn-ghost btn-sm" @click="isEditing = false">{{ t("rules.editor.cancel") }}</button>
          </div>
        </div>
        <textarea
          v-model="editorContent"
          class="yaml-editor"
          spellcheck="false"
          :placeholder="t('rules.editor.placeholder')"
        ></textarea>
        <div v-if="validateMsg" :class="validateOk ? 'msg-ok' : 'msg-err'">
          {{ validateMsg }}
        </div>
        <div class="muted tips">
          {{ t("rules.editor.tips") }}
        </div>
      </template>
      <div v-else class="placeholder">
        <div class="placeholder-icon">📝</div>
        <p>{{ t("rules.placeholder.title") }}</p>
        <p class="muted placeholder-path">
          {{ t("rules.placeholder.dir", { path: rulesDir }) }}
          <button class="link-btn" @click="openDir">{{ t("common.open") }}</button>
        </p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.layout {
  display: grid;
  grid-template-columns: 360px 1fr;
  gap: 16px;
  align-items: start;
}
.list-panel {
  padding: 16px 18px;
}
.list-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.section {
  margin-top: 16px;
}
.section-title {
  font-size: 13px;
  font-weight: 600;
  margin-bottom: 8px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.empty {
  font-size: 12.5px;
  padding: 4px 0;
}
.user-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-radius: 8px;
  padding: 2px 6px;
  transition: background-color 0.12s ease;
}
.user-row:hover {
  background: var(--panel-hover);
}
.user-row.active {
  background: var(--primary-soft);
}
.file-btn {
  border: 0;
  background: transparent;
  text-align: left;
  cursor: pointer;
  font-size: 13px;
  color: var(--primary-soft-text);
  padding: 7px 0;
  flex: 1;
  display: flex;
  align-items: center;
  gap: 7px;
  font-family: inherit;
}
.file-icon {
  opacity: 0.7;
  flex-shrink: 0;
}
.del-btn {
  border: 0;
  background: transparent;
  color: var(--red-text);
  cursor: pointer;
  font-size: 12px;
  font-family: inherit;
  padding: 4px 8px;
  border-radius: 6px;
}
.del-btn:hover {
  background: var(--red-soft);
}
.builtin-group details {
  margin-bottom: 4px;
}
.builtin-group summary {
  cursor: pointer;
  font-size: 13px;
  padding: 6px 8px;
  color: var(--text-2);
  border-radius: 8px;
  list-style: none;
  display: flex;
  align-items: center;
  gap: 8px;
}
.builtin-group summary::-webkit-details-marker {
  display: none;
}
.builtin-group summary::before {
  content: "";
  width: 6px;
  height: 6px;
  border-right: 1.5px solid currentColor;
  border-bottom: 1.5px solid currentColor;
  transform: rotate(-45deg);
  transition: transform 0.18s ease;
  flex-shrink: 0;
}
.builtin-group details[open] summary::before {
  transform: rotate(45deg);
}
.builtin-group summary:hover {
  background: var(--panel-hover);
  color: var(--text);
}
.builtin-count {
  margin-left: auto;
  font-size: 11px;
  color: var(--muted);
  background: var(--panel-2);
  border-radius: 20px;
  padding: 0 8px;
}
.builtin-row {
  padding: 7px 8px 7px 20px;
  border-bottom: 1px solid var(--border);
}
.builtin-line {
  display: flex;
  align-items: center;
  gap: 7px;
}
.rule-name {
  font-size: 12.5px;
  font-weight: 500;
}
.rule-id {
  font-size: 11.5px;
  margin-top: 3px;
  word-break: break-all;
  line-height: 1.5;
}
.editor-panel {
  min-height: 520px;
  display: flex;
  flex-direction: column;
}
.editor-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 10px;
  gap: 12px;
}
.editor-sub {
  font-size: 12px;
  margin-top: 3px;
}
.editor-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}
.yaml-editor {
  flex: 1;
  min-height: 380px;
  width: 100%;
  font-family: Consolas, "Courier New", monospace;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--text);
  background: var(--panel-2);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  padding: 12px;
  resize: vertical;
}
.msg-ok {
  margin-top: 8px;
  color: var(--green-text);
  background: var(--green-soft);
  border-radius: 9px;
  padding: 8px 12px;
  font-size: 12.5px;
}
.msg-err {
  margin-top: 8px;
  color: var(--red-text);
  background: var(--red-soft);
  border-radius: 9px;
  padding: 8px 12px;
  font-size: 12.5px;
}
.tips {
  margin-top: 10px;
  font-size: 12px;
  line-height: 1.7;
}
code {
  background: var(--panel-2);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 0 5px;
  font-size: 11.5px;
}
.placeholder {
  margin: auto;
  text-align: center;
  color: var(--muted);
  padding: 40px 20px;
}
.placeholder-icon {
  font-size: 38px;
  margin-bottom: 12px;
}
.placeholder-path {
  font-size: 12.5px;
  word-break: break-all;
}
.error-panel {
  color: var(--red-text);
}
</style>
