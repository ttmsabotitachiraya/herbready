<script setup lang="ts">
/**
 * AppSettingsDialog — Manage drug list and departments.
 */
import { ref, reactive, onMounted } from "vue";
import {
    Settings,
    Plus,
    Trash2,
    X,
    Save,
    Search,
    Loader,
    Upload,
    Download,
    Eye,
    EyeOff,
} from "lucide-vue-next";
import type { DrugConfig, DeptConfig } from "../types";
import { api } from "../api/tauri";
import { appConfig, saveAppConfig } from "../stores/appConfig";

const emit = defineEmits<{
    (e: "close"): void;
}>();

type Page = "drugs" | "depts";
const activePage = ref<Page>("drugs");
const saving = ref(false);

// Drug form
const drugForm = reactive<DrugConfig>({
    icode: "",
    abbr: "",
    course_days: 7,
    capsules: 0,
    drug_name: "",
    enabled: true,
});
const drugLookupDone = ref(false);
const drugLookupLoading = ref(false);
const drugs = ref<DrugConfig[]>([]);
const selectedDrugIdx = ref<number | null>(null);

// Dept form
const deptForm = reactive<DeptConfig>({ code: "", name: "" });
const deptLookupDone = ref(false);
const deptLookupLoading = ref(false);
const depts = ref<DeptConfig[]>([]);
const selectedDeptIdx = ref<number | null>(null);

onMounted(() => {
    drugs.value = appConfig.value.drugs.map((d) => ({ ...d }));
    depts.value = appConfig.value.departments.map((d) => ({ ...d }));
});

// ── Drug operations ────────────────────────────────────────────────────────

/** Called on every keystroke in icode field — resets lookup state */
function onIcodeInput() {
    if (drugLookupDone.value) {
        drugLookupDone.value = false;
        drugForm.drug_name = "";
    }
}

async function lookupDrugName() {
    if (!drugForm.icode.trim()) return;
    drugLookupLoading.value = true;
    drugLookupDone.value = false;
    drugForm.drug_name = "";
    try {
        const name = await api.lookupDrugName(drugForm.icode.trim());
        drugForm.drug_name = name;
        drugLookupDone.value = true;
    } catch {
        drugForm.drug_name = "";
        drugLookupDone.value = false;
    } finally {
        drugLookupLoading.value = false;
    }
}

function selectDrug(idx: number) {
    selectedDrugIdx.value = idx;
    const d = drugs.value[idx];
    drugForm.icode = d.icode;
    drugForm.abbr = d.abbr;
    drugForm.course_days = d.course_days;
    drugForm.capsules = d.capsules;
    drugForm.drug_name = d.drug_name;
    drugForm.enabled = d.enabled !== false;
    drugLookupDone.value = true; // existing entry already verified
}

function addOrUpdateDrug() {
    if (!drugForm.icode.trim() || !drugForm.drug_name.trim()) return;
    if (!drugLookupDone.value) return;
    const entry: DrugConfig = {
        icode: drugForm.icode.trim(),
        abbr: drugForm.abbr.trim(),
        course_days: Number(drugForm.course_days) || 7,
        capsules: Number(drugForm.capsules) || 0,
        drug_name: drugForm.drug_name.trim(),
        enabled: drugForm.enabled !== false,
    };
    if (selectedDrugIdx.value !== null) {
        drugs.value[selectedDrugIdx.value] = entry;
        selectedDrugIdx.value = null;
    } else {
        drugs.value.push(entry);
    }
    clearDrugForm();
}

function toggleDrugEnabled(idx: number) {
    const d = drugs.value[idx];
    drugs.value[idx] = { ...d, enabled: d.enabled === false ? true : false };
    // If this item is selected in the form, sync the form enabled state
    if (selectedDrugIdx.value === idx) {
        drugForm.enabled = drugs.value[idx].enabled !== false;
    }
}

function deleteDrug(idx: number) {
    drugs.value.splice(idx, 1);
    if (selectedDrugIdx.value === idx) clearDrugForm();
    else if (selectedDrugIdx.value !== null && selectedDrugIdx.value > idx) {
        selectedDrugIdx.value -= 1;
    }
}

function clearDrugForm() {
    Object.assign(drugForm, {
        icode: "",
        abbr: "",
        course_days: 7,
        capsules: 0,
        drug_name: "",
        enabled: true,
    });
    drugLookupDone.value = false;
    selectedDrugIdx.value = null;
}

// ── Dept operations ────────────────────────────────────────────────────────

/** Called on every keystroke in dept code field — resets lookup state */
function onDeptCodeInput() {
    if (deptLookupDone.value) {
        deptLookupDone.value = false;
        deptForm.name = "";
    }
}

async function lookupDeptName() {
    if (!deptForm.code.trim()) return;
    deptLookupLoading.value = true;
    deptLookupDone.value = false;
    deptForm.name = "";
    try {
        const name = await api.lookupDeptName(deptForm.code.trim());
        deptForm.name = name;
        deptLookupDone.value = true;
    } catch {
        deptForm.name = "";
        deptLookupDone.value = false;
    } finally {
        deptLookupLoading.value = false;
    }
}

function selectDept(idx: number) {
    selectedDeptIdx.value = idx;
    const d = depts.value[idx];
    deptForm.code = d.code;
    deptForm.name = d.name;
    deptLookupDone.value = true; // existing entry already verified
}

function addOrUpdateDept() {
    if (!deptForm.code.trim() || !deptForm.name.trim()) return;
    if (!deptLookupDone.value) return;
    const entry: DeptConfig = { ...deptForm };
    if (selectedDeptIdx.value !== null) {
        depts.value[selectedDeptIdx.value] = entry;
        selectedDeptIdx.value = null;
    } else {
        depts.value.push(entry);
    }
    clearDeptForm();
}

function deleteDept(idx: number) {
    depts.value.splice(idx, 1);
    if (selectedDeptIdx.value === idx) clearDeptForm();
    else if (selectedDeptIdx.value !== null && selectedDeptIdx.value > idx) {
        selectedDeptIdx.value -= 1;
    }
}

function clearDeptForm() {
    Object.assign(deptForm, { code: "", name: "" });
    deptLookupDone.value = false;
    selectedDeptIdx.value = null;
}

// ── Export config ──────────────────────────────────────────────────────────
async function exportConfig() {
    try {
        const jsonStr = await api.exportAppConfig();
        const filePath = await api.saveDialog("app_config.json", [
            { name: "JSON", extensions: ["json"] },
        ]);
        if (filePath) {
            const { writeTextFile } = await import("@tauri-apps/plugin-fs");
            await writeTextFile(filePath, jsonStr);
            alert("ส่งออกข้อมูลสำเร็จ");
        }
    } catch (err: unknown) {
        alert(
            `ส่งออกล้มเหลว: ${err instanceof Error ? err.message : String(err)}`,
        );
    }
}

// ── Import config ──────────────────────────────────────────────────────────
async function importConfig() {
    try {
        const filePath = await api.openDialog([
            { name: "JSON", extensions: ["json"] },
        ]);
        if (!filePath || Array.isArray(filePath)) return;
        const { readTextFile } = await import("@tauri-apps/plugin-fs");
        const content = await readTextFile(filePath);
        const cfg = await api.importAppConfig(content);
        drugs.value = cfg.drugs.map((d) => ({ ...d }));
        depts.value = cfg.departments.map((d) => ({ ...d }));
        alert("นำเข้าข้อมูลสำเร็จ");
    } catch (err: unknown) {
        alert(
            `นำเข้าล้มเหลว: ${err instanceof Error ? err.message : String(err)}`,
        );
    }
}

// ── Save ───────────────────────────────────────────────────────────────────
async function save() {
    saving.value = true;
    try {
        await saveAppConfig({ drugs: drugs.value, departments: depts.value });
        emit("close");
    } catch (err: unknown) {
        alert(
            `บันทึกล้มเหลว: ${err instanceof Error ? err.message : String(err)}`,
        );
    } finally {
        saving.value = false;
    }
}
</script>

<template>
    <div class="dialog-overlay" @click.self="emit('close')">
        <div class="dialog dialog--wide">
            <!-- Header -->
            <div class="dialog__header">
                <Settings :size="18" />
                <span class="dialog__title">ตั้งค่าแอปพลิเคชัน</span>
                <button
                    class="dialog__close"
                    @click="emit('close')"
                    type="button"
                >
                    <X :size="16" />
                </button>
            </div>

            <!-- Tab selector -->
            <div class="app-settings__tabs">
                <button
                    class="app-tab-btn"
                    :class="{ 'app-tab-btn--active': activePage === 'drugs' }"
                    @click="activePage = 'drugs'"
                    type="button"
                >
                    รายการยาสมุนไพร ({{ drugs.length }})
                </button>
                <button
                    class="app-tab-btn"
                    :class="{ 'app-tab-btn--active': activePage === 'depts' }"
                    @click="activePage = 'depts'"
                    type="button"
                >
                    แผนก ({{ depts.length }})
                </button>
            </div>

            <!-- Body -->
            <div class="dialog__body dialog__body--split">
                <!-- ── Drug page ────────────────────────────────────────────── -->
                <template v-if="activePage === 'drugs'">
                    <!-- Drug list -->
                    <div class="settings-list">
                        <div
                            v-for="(d, idx) in drugs"
                            :key="d.icode"
                            class="settings-list__item"
                            :class="{
                                'settings-list__item--selected':
                                    selectedDrugIdx === idx,
                            }"
                            @click="selectDrug(idx)"
                        >
                            <div class="settings-list__main">
                                <span class="settings-list__primary">{{
                                    d.drug_name
                                }}</span>
                                <span class="settings-list__secondary">
                                    {{ d.icode }} | abbr: {{ d.abbr }}
                                    <span
                                        v-if="d.enabled === false"
                                        class="disabled-badge"
                                        >ปิดใช้งาน</span
                                    >
                                </span>
                            </div>
                            <button
                                class="settings-list__toggle"
                                @click.stop="toggleDrugEnabled(idx)"
                                type="button"
                                :title="
                                    d.enabled === false
                                        ? 'เปิดใช้งาน'
                                        : 'ปิดใช้งาน'
                                "
                            >
                                <EyeOff v-if="d.enabled === false" :size="13" />
                                <Eye v-else :size="13" />
                            </button>
                            <button
                                class="settings-list__del"
                                @click.stop="deleteDrug(idx)"
                                type="button"
                                title="ลบ"
                            >
                                <Trash2 :size="13" />
                            </button>
                        </div>
                        <div
                            v-if="drugs.length === 0"
                            class="settings-list__empty"
                        >
                            ยังไม่มีรายการยา
                        </div>
                    </div>

                    <!-- Drug form -->
                    <div class="settings-form">
                        <h3 class="settings-form__title">
                            {{
                                selectedDrugIdx !== null
                                    ? "แก้ไขยา"
                                    : "เพิ่มยาใหม่"
                            }}
                        </h3>
                        <div class="form-row">
                            <label class="form-label">iCode</label>
                            <div class="icode-wrap">
                                <input
                                    v-model="drugForm.icode"
                                    class="form-input"
                                    type="text"
                                    placeholder="1580004"
                                    @input="onIcodeInput"
                                />
                                <button
                                    class="btn btn--ghost btn--sm"
                                    @click="lookupDrugName"
                                    :disabled="drugLookupLoading"
                                    type="button"
                                >
                                    <Loader
                                        v-if="drugLookupLoading"
                                        :size="12"
                                        class="spin"
                                    />
                                    <Search v-else :size="12" />
                                </button>
                            </div>
                        </div>
                        <div class="form-row">
                            <label class="form-label">ชื่อยา</label>
                            <!-- ชื่อยา — auto-filled, readonly -->
                            <input
                                v-model="drugForm.drug_name"
                                class="form-input form-input--readonly"
                                type="text"
                                placeholder="กดค้นหา iCode ก่อน"
                                readonly
                            />
                        </div>
                        <div class="form-row">
                            <label class="form-label">ตัวย่อ</label>
                            <input
                                v-model="drugForm.abbr"
                                class="form-input"
                                type="text"
                                placeholder="ฟ้า"
                                :disabled="!drugLookupDone"
                            />
                        </div>
                        <div class="form-row">
                            <label class="form-label">วันที่จ่าย</label>
                            <input
                                v-model.number="drugForm.course_days"
                                class="form-input"
                                type="number"
                                min="1"
                                :disabled="!drugLookupDone"
                            />
                            <span class="form-unit">วัน</span>
                        </div>
                        <div class="form-row">
                            <label class="form-label">จำนวน</label>
                            <input
                                v-model.number="drugForm.capsules"
                                class="form-input"
                                type="number"
                                min="0"
                                :disabled="!drugLookupDone"
                            />
                            <span class="form-unit">หน่วยนับ</span>
                        </div>
                        <div class="form-actions">
                            <button
                                class="btn btn--ghost btn--sm"
                                @click="clearDrugForm"
                                type="button"
                            >
                                ล้าง
                            </button>
                            <button
                                class="btn btn--primary btn--sm"
                                @click="addOrUpdateDrug"
                                :disabled="!drugLookupDone"
                                type="button"
                            >
                                <Plus :size="13" />
                                {{
                                    selectedDrugIdx !== null
                                        ? "อัพเดต"
                                        : "เพิ่ม"
                                }}
                            </button>
                        </div>
                    </div>
                </template>

                <!-- ── Dept page ────────────────────────────────────────────── -->
                <template v-if="activePage === 'depts'">
                    <!-- Dept list -->
                    <div class="settings-list">
                        <div
                            v-for="(d, idx) in depts"
                            :key="d.code"
                            class="settings-list__item"
                            :class="{
                                'settings-list__item--selected':
                                    selectedDeptIdx === idx,
                            }"
                            @click="selectDept(idx)"
                        >
                            <div class="settings-list__main">
                                <span class="settings-list__primary">{{
                                    d.name
                                }}</span>
                                <span class="settings-list__secondary"
                                    >รหัส: {{ d.code }}</span
                                >
                            </div>
                            <button
                                class="settings-list__del"
                                @click.stop="deleteDept(idx)"
                                type="button"
                                title="ลบ"
                            >
                                <Trash2 :size="13" />
                            </button>
                        </div>
                        <div
                            v-if="depts.length === 0"
                            class="settings-list__empty"
                        >
                            ยังไม่มีรายการแผนก
                        </div>
                    </div>

                    <!-- Dept form -->
                    <div class="settings-form">
                        <h3 class="settings-form__title">
                            {{
                                selectedDeptIdx !== null
                                    ? "แก้ไขแผนก"
                                    : "เพิ่มแผนกใหม่"
                            }}
                        </h3>
                        <div class="form-row">
                            <label class="form-label">รหัสแผนก</label>
                            <div class="icode-wrap">
                                <input
                                    v-model="deptForm.code"
                                    class="form-input"
                                    type="text"
                                    placeholder="011"
                                    @input="onDeptCodeInput"
                                />
                                <button
                                    class="btn btn--ghost btn--sm"
                                    @click="lookupDeptName"
                                    :disabled="deptLookupLoading"
                                    type="button"
                                >
                                    <Loader
                                        v-if="deptLookupLoading"
                                        :size="12"
                                        class="spin"
                                    />
                                    <Search v-else :size="12" />
                                </button>
                            </div>
                        </div>
                        <div class="form-row">
                            <label class="form-label">ชื่อแผนก</label>
                            <!-- ชื่อแผนก — auto-filled, readonly -->
                            <input
                                v-model="deptForm.name"
                                class="form-input form-input--readonly"
                                type="text"
                                placeholder="กดค้นหารหัสแผนกก่อน"
                                readonly
                            />
                        </div>
                        <div class="form-actions">
                            <button
                                class="btn btn--ghost btn--sm"
                                @click="clearDeptForm"
                                type="button"
                            >
                                ล้าง
                            </button>
                            <button
                                class="btn btn--primary btn--sm"
                                @click="addOrUpdateDept"
                                :disabled="!deptLookupDone"
                                type="button"
                            >
                                <Plus :size="13" />
                                {{
                                    selectedDeptIdx !== null
                                        ? "อัพเดต"
                                        : "เพิ่ม"
                                }}
                            </button>
                        </div>
                    </div>
                </template>
            </div>

            <!-- Footer -->
            <div class="dialog__footer">
                <div class="dialog__footer-left">
                    <button
                        class="btn btn--ghost"
                        @click="importConfig"
                        type="button"
                    >
                        <Upload :size="13" />
                        นำเข้า
                    </button>
                    <button
                        class="btn btn--ghost"
                        @click="exportConfig"
                        type="button"
                    >
                        <Download :size="13" />
                        ส่งออก
                    </button>
                </div>
                <div class="dialog__footer-spacer" />
                <div class="dialog__footer-right">
                    <button
                        class="btn btn--ghost"
                        @click="emit('close')"
                        type="button"
                    >
                        ยกเลิก
                    </button>
                    <button
                        class="btn btn--primary"
                        @click="save"
                        :disabled="saving"
                        type="button"
                    >
                        <Loader v-if="saving" :size="13" class="spin" />
                        <Save v-else :size="13" />
                        บันทึก
                    </button>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
}

.dialog {
    background: var(--bg-input);
    border: 1.5px solid var(--border-default);
    border-radius: 14px;
    width: 640px;
    max-width: 96vw;
    max-height: 92vh;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
}

.dialog--wide {
    width: 720px;
}

.dialog__header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 16px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border-subtle);
    color: var(--accent-primary-dark);
    flex-shrink: 0;
}

.dialog__title {
    flex: 1;
    font-size: 15px;
    font-weight: 700;
    color: var(--text-primary);
    font-family: var(--font-thai);
}

.dialog__close {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    padding: 3px;
    border-radius: 5px;
}
.dialog__close:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
}

/* Page tabs */
.app-settings__tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border-default);
    background: var(--bg-surface);
    flex-shrink: 0;
}

.app-tab-btn {
    padding: 8px 18px;
    border: none;
    background: none;
    font-family: var(--font-thai);
    font-size: 13px;
    color: var(--text-muted);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition:
        color 0.1s,
        border-color 0.1s;
}
.app-tab-btn:hover {
    color: var(--text-primary);
}
.app-tab-btn--active {
    color: var(--accent-primary-dark);
    border-bottom-color: var(--accent-primary);
    font-weight: 700;
}

/* Body split */
.dialog__body--split {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
}

/* List panel */
.settings-list {
    width: 340px;
    flex-shrink: 0;
    overflow-y: auto;
    border-right: 1px solid var(--border-subtle);
    padding: 6px 0;
}

.settings-list__item {
    display: flex;
    align-items: center;
    padding: 8px 12px;
    cursor: pointer;
    border-bottom: 1px solid var(--border-subtle);
    gap: 8px;
    transition: background 0.1s;
}
.settings-list__item:hover {
    background: var(--bg-row-alt);
}
.settings-list__item--selected {
    background: var(--drug-green-bg);
}
.settings-list__item--disabled {
    opacity: 0.55;
}
.settings-list__item--disabled .settings-list__primary {
    text-decoration: line-through;
    color: var(--text-muted);
}

.settings-list__main {
    flex: 1;
    min-width: 0;
}
.settings-list__primary {
    display: block;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    font-family: var(--font-thai);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.settings-list__secondary {
    display: block;
    font-size: 11px;
    color: var(--text-muted);
    font-family: var(--font-thai);
    margin-top: 1px;
}

.disabled-badge {
    display: inline-block;
    background: #fee2e2;
    color: #b91c1c;
    font-size: 10px;
    font-weight: 600;
    border-radius: 4px;
    padding: 0 5px;
    margin-left: 4px;
    vertical-align: middle;
}

.settings-list__toggle {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    padding: 4px;
    border-radius: 5px;
    opacity: 0.6;
    transition:
        opacity 0.1s,
        color 0.1s;
    flex-shrink: 0;
}
.settings-list__toggle:hover {
    opacity: 1;
    color: var(--accent-primary-dark);
}

.settings-list__del {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    padding: 4px;
    border-radius: 5px;
    opacity: 0.5;
    transition:
        opacity 0.1s,
        color 0.1s;
    flex-shrink: 0;
}
.settings-list__del:hover {
    opacity: 1;
    color: var(--accent-red);
}

.settings-list__empty {
    padding: 20px;
    text-align: center;
    color: var(--text-muted);
    font-family: var(--font-thai);
    font-size: 13px;
}

/* Form panel */
.settings-form {
    flex: 1;
    padding: 16px 18px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.settings-form__title {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    font-family: var(--font-thai);
    margin: 0 0 4px;
}

.form-row {
    display: flex;
    align-items: center;
    gap: 8px;
}

.form-label {
    width: 72px;
    flex-shrink: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    font-family: var(--font-thai);
}

.form-input {
    flex: 1;
    height: 32px;
    padding: 0 9px;
    border: 1.5px solid var(--border-default);
    border-radius: 7px;
    background: var(--bg-window);
    color: var(--text-primary);
    font-size: 13px;
    font-family: var(--font-thai);
    outline: none;
    transition: border-color 0.15s;
}
.form-input:focus {
    border-color: var(--border-focus);
}
.form-input:disabled {
    background: var(--bg-surface);
    color: var(--text-muted);
    cursor: not-allowed;
    opacity: 0.6;
}
.form-input--readonly {
    background: var(--bg-surface);
    color: var(--text-secondary);
    cursor: default;
}

/* Hide number input spinner arrows */
.form-input[type="number"]::-webkit-outer-spin-button,
.form-input[type="number"]::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
}
.form-input[type="number"] {
    -moz-appearance: textfield;
    appearance: textfield;
}

.form-unit {
    font-size: 12px;
    color: var(--text-muted);
    font-family: var(--font-thai);
    flex-shrink: 0;
}

.icode-wrap {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
}
.icode-wrap .form-input {
    flex: 1;
}

.form-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 4px;
}

/* Buttons */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 8px 16px;
    border-radius: 9999px;
    font-size: 13px;
    font-family: var(--font-thai);
    font-weight: 600;
    cursor: pointer;
    transition:
        background 0.15s,
        color 0.15s,
        transform 0.1s;
    border: none;
    outline: none;
}
.btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}
.btn--sm {
    padding: 4px 12px;
    font-size: 12px;
}
.btn--primary {
    background: #9fe870;
    color: #163300;
    border: none;
}
.btn--primary:hover:not(:disabled) {
    background: #cdffad;
    transform: scale(1.05);
}
.btn--primary:active:not(:disabled) {
    transform: scale(0.95);
}
.btn--ghost {
    background: transparent;
    color: #454745;
    border: 1px solid rgba(14, 15, 12, 0.15);
}
.btn--ghost:hover:not(:disabled) {
    background: rgba(14, 15, 12, 0.06);
    transform: scale(1.05);
}

/* Footer */
.dialog__footer-left,
.dialog__footer-right {
    display: flex;
    gap: 8px;
    align-items: center;
}

.dialog__footer-spacer {
    flex: 1;
}

.dialog__footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    justify-content: flex-end;
    flex-shrink: 0;
}

.spin {
    animation: spin 1s linear infinite;
}
@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
</style>
