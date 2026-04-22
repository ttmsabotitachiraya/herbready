<script setup lang="ts">
/**
 * HistoryTab — ประวัติรับยาสมุนไพร
 * Shows patient herb dispensing history in a tree view grouped by year → month → date.
 */
import { ref, computed } from "vue";
import {
    Search,
    X,
    User,
    Calendar,
    ChevronRight,
    ChevronDown,
    Loader,
    Clock,
} from "lucide-vue-next";
import type { DrugDispenseItem, PatientRecord } from "../types";
import { api } from "../api/tauri";
import { formatDateThaiShort, THAI_MONTHS_SHORT } from "../utils/dateHelper";
import { hnPadded } from "../utils/drugParser";

// ── Types ─────────────────────────────────────────────────────────────────
interface TreeDate {
    dateISO: string;
    dateLabel: string;
    drugs: DrugDispenseItem[];
}
interface TreeMonth {
    monthLabel: string;
    monthKey: string; // "YYYY-MM"
    dates: TreeDate[];
    expanded: boolean;
}
interface TreeYear {
    beYear: number;
    months: TreeMonth[];
    expanded: boolean;
    totalVisits: number;
}

// ── Range options ─────────────────────────────────────────────────────────
const RANGE_OPTIONS = [
    { label: "1 ปี", years: 1 },
    { label: "2 ปี", years: 2 },
    { label: "3 ปี", years: 3 },
    { label: "5 ปี", years: 5 },
    { label: "ทั้งหมด", years: null },
];

// ── State ─────────────────────────────────────────────────────────────────
const searchText = ref("");
const selectedRangeIdx = ref(0);
const loading = ref(false);
const hint = ref("พิมพ์ HN หรือชื่อผู้ป่วย แล้วกด Enter");
const hintOk = ref(false);
const hintError = ref(false);

const currentHn = ref("");
const linkedName = ref("");
const currentRecord = ref<PatientRecord | null>(null);

// Candidate list shown when name search returns multiple patients
const patientCandidates = ref<
    Array<{ hn: string; cid: string; pt_name: string; pttype_name: string }>
>([]);
// Controls whether we're viewing a patient's history (true) or the candidates list (false)
const showingPatientHistory = ref(false);
// Patient info from selected candidate (when not coming from SearchTab handoff)
const currentCid = ref("");
const currentPttype = ref("");

const treeData = ref<TreeYear[]>([]);
const selectedDate = ref<string | null>(null);
const selectedDrugs = ref<DrugDispenseItem[]>([]);

// ── Expose for parent App to call ─────────────────────────────────────────
defineExpose({ setPatientFromLink });

async function setPatientFromLink(
    hn: string,
    name: string,
    record?: PatientRecord,
) {
    patientCandidates.value = [];
    showingPatientHistory.value = true;
    linkedName.value = name.trim();
    currentRecord.value = record ?? null;
    currentCid.value = record?.cid ?? "";
    currentPttype.value = record?.pttype_today ?? "";
    searchText.value = hn.trim();
    if (hn.trim()) await doSearch(hn.trim());
}

// ── Computed ──────────────────────────────────────────────────────────────
const selectedDateLabel = computed(() =>
    selectedDate.value ? formatDateThaiShort(selectedDate.value) : "",
);

const selectedDrugsTotal = computed(() => selectedDrugs.value.length);

// ── Methods ───────────────────────────────────────────────────────────────
function getYearsBack(): number | null {
    return RANGE_OPTIONS[selectedRangeIdx.value].years;
}

async function onSearchKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") await doSearch();
}

async function doSearch(forcedHn?: string) {
    const text = (forcedHn ?? searchText.value).trim();
    if (!text) return;

    loading.value = true;
    hint.value = "กำลังค้นหา…";
    hintOk.value = false;
    hintError.value = false;
    treeData.value = [];
    selectedDate.value = null;
    selectedDrugs.value = [];

    // Clear candidates when starting a fresh search (not when called with forcedHn)
    if (!forcedHn) {
        patientCandidates.value = [];
        showingPatientHistory.value = false;
        currentHn.value = "";
        linkedName.value = "";
        currentRecord.value = null;
        currentCid.value = "";
        currentPttype.value = "";
    }

    try {
        let items: DrugDispenseItem[];

        const isHn = /^\d{5,9}$/.test(text);
        const isCid = /^\d{13}$/.test(text);

        if (isHn || isCid) {
            // Direct HN or CID lookup — resolve patient info first
            if (isCid) {
                // Must resolve CID → HN before querying history (history query uses hn)
                const patientInfo = await api.findPatientById(text);
                if (patientInfo) {
                    currentHn.value = patientInfo.hn;
                    linkedName.value = patientInfo.pt_name;
                    currentCid.value = patientInfo.cid;
                    currentPttype.value = patientInfo.pttype_name;
                } else {
                    hint.value = `ไม่พบผู้ป่วย CID: ${text}`;
                    hintError.value = true;
                    loading.value = false;
                    return;
                }
                showingPatientHistory.value = true;
                items = await api.getPatientHistory(
                    currentHn.value,
                    getYearsBack(),
                );
            } else {
                // HN lookup — fetch patient info and history in parallel
                const [patientInfo, historyItems] = await Promise.all([
                    api.findPatientById(text),
                    api.getPatientHistory(text, getYearsBack()),
                ]);
                items = historyItems;
                if (patientInfo) {
                    currentHn.value = patientInfo.hn;
                    linkedName.value = patientInfo.pt_name;
                    currentCid.value = patientInfo.cid;
                    currentPttype.value = patientInfo.pttype_name;
                } else {
                    currentHn.value = text;
                }
                showingPatientHistory.value = true;
            }
        } else {
            // Name search — first find candidate patients
            const candidates = await api.findPatientsByName(text);

            if (candidates.length === 0) {
                hint.value = `ไม่พบผู้ป่วยชื่อ "${text}"`;
                hintError.value = true;
                loading.value = false;
                return;
            }

            if (candidates.length === 1) {
                // Auto-select the single match
                const cand = candidates[0];
                currentHn.value = cand.hn;
                if (!linkedName.value) linkedName.value = cand.pt_name;
                currentCid.value = cand.cid;
                currentPttype.value = cand.pttype_name;
                showingPatientHistory.value = true;
                items = await api.getPatientHistory(cand.hn, getYearsBack());
            } else {
                // Multiple matches — show selection list, stop loading
                patientCandidates.value = candidates;
                hint.value = `พบ ${candidates.length} รายการ — กรุณาเลือกผู้ป่วย`;
                hintOk.value = true;
                loading.value = false;
                return;
            }
        }

        if (items.length === 0) {
            hint.value = "ไม่พบประวัติการรับยาสมุนไพรในช่วงเวลาที่เลือก";
            hintOk.value = false;
        } else {
            buildTree(items);
            const visitCount = treeData.value.reduce(
                (s, y) =>
                    s + y.months.reduce((sm, m) => sm + m.dates.length, 0),
                0,
            );
            const rangeLabel = RANGE_OPTIONS[selectedRangeIdx.value].label;
            hint.value = `พบ ${visitCount} ครั้งที่รับยา  (${items.length} รายการยา)  |  ${rangeLabel}`;
            hintOk.value = true;
        }
    } catch (err: unknown) {
        hint.value = `❌ ${err instanceof Error ? err.message : String(err)}`;
        hintError.value = true;
    } finally {
        loading.value = false;
    }
}

async function selectCandidate(cand: {
    hn: string;
    cid: string;
    pt_name: string;
    pttype_name: string;
}) {
    // Keep patientCandidates intact for back button — don't clear them
    showingPatientHistory.value = true;
    currentHn.value = cand.hn;
    linkedName.value = cand.pt_name;
    currentCid.value = cand.cid;
    currentPttype.value = cand.pttype_name;
    currentRecord.value = null;
    // Preserve the user's typed query in the search box — do not overwrite searchText.
    await doSearch(cand.hn);
}

function backToSearch() {
    treeData.value = [];
    selectedDate.value = null;
    selectedDrugs.value = [];

    if (patientCandidates.value.length > 0) {
        // Instantly show candidates list again — no re-fetch needed
        showingPatientHistory.value = false;
        currentHn.value = "";
        linkedName.value = "";
        currentCid.value = "";
        currentPttype.value = "";
        currentRecord.value = null;
        // Restore the hint to show candidate count
        hint.value = `พบ ${patientCandidates.value.length} รายการ — กรุณาเลือกผู้ป่วย`;
        hintOk.value = true;
        hintError.value = false;
    } else {
        // No candidates — full clear
        currentHn.value = "";
        linkedName.value = "";
        currentCid.value = "";
        currentPttype.value = "";
        currentRecord.value = null;
        hint.value = "พิมพ์ HN หรือชื่อผู้ป่วย แล้วกด Enter";
        hintOk.value = false;
        hintError.value = false;
    }
}

function buildTree(items: DrugDispenseItem[]) {
    // Group by date
    const dateMap = new Map<string, DrugDispenseItem[]>();
    for (const item of items) {
        if (!item.vstdate) continue;
        const key = item.vstdate.split("T")[0];
        if (!dateMap.has(key)) dateMap.set(key, []);
        dateMap.get(key)!.push(item);
    }

    // Group by BE year → month
    const yearMap = new Map<number, Map<string, string[]>>(); // beYear → monthKey → dates

    for (const dateISO of dateMap.keys()) {
        const [y, m] = dateISO.split("-").map(Number);
        const beYear = y + 543;
        const monthKey = `${y}-${String(m).padStart(2, "0")}`;
        if (!yearMap.has(beYear)) yearMap.set(beYear, new Map());
        const months = yearMap.get(beYear)!;
        if (!months.has(monthKey)) months.set(monthKey, []);
        months.get(monthKey)!.push(dateISO);
    }

    const years: TreeYear[] = [];
    const sortedYears = [...yearMap.keys()].sort((a, b) => b - a);

    for (const beYear of sortedYears) {
        const monthsMap = yearMap.get(beYear)!;
        const months: TreeMonth[] = [];
        const sortedMonthKeys = [...monthsMap.keys()].sort().reverse();

        for (const mk of sortedMonthKeys) {
            const [, mNum] = mk.split("-").map(Number);
            const dates: TreeDate[] = monthsMap
                .get(mk)!
                .sort()
                .reverse()
                .map((dateISO) => ({
                    dateISO,
                    dateLabel: formatDateThaiShort(dateISO),
                    drugs: dateMap.get(dateISO)!,
                }));

            months.push({
                monthLabel: `${THAI_MONTHS_SHORT[mNum - 1]} ${beYear}`,
                monthKey: mk,
                dates,
                expanded: false,
            });
        }

        const totalVisits = months.reduce((s, m) => s + m.dates.length, 0);
        years.push({ beYear, months, expanded: false, totalVisits });
    }

    // Auto-expand first year and first month
    if (years.length > 0) {
        years[0].expanded = true;
        if (years[0].months.length > 0) {
            years[0].months[0].expanded = true;
        }
    }

    treeData.value = years;
}

function toggleYear(year: TreeYear) {
    year.expanded = !year.expanded;
}

function toggleMonth(month: TreeMonth) {
    month.expanded = !month.expanded;
}

function selectDate(dateNode: TreeDate) {
    selectedDate.value = dateNode.dateISO;
    selectedDrugs.value = dateNode.drugs;
}

function clearSearch() {
    searchText.value = "";
    treeData.value = [];
    selectedDate.value = null;
    selectedDrugs.value = [];
    currentHn.value = "";
    linkedName.value = "";
    currentRecord.value = null;
    patientCandidates.value = [];
    showingPatientHistory.value = false;
    currentCid.value = "";
    currentPttype.value = "";
    hint.value = "พิมพ์ HN หรือชื่อผู้ป่วย แล้วกด Enter";
    hintOk.value = false;
    hintError.value = false;
}

async function onRangeChange() {
    if (currentHn.value) await doSearch(currentHn.value);
}
</script>

<template>
    <div class="history-tab">
        <!-- ── Toolbar ──────────────────────────────────────────────────────── -->
        <div class="history-tab__toolbar">
            <div class="history-tab__field-wrap">
                <span class="history-tab__field-icon"
                    ><Search :size="15"
                /></span>
                <input
                    v-model="searchText"
                    class="history-tab__input"
                    type="text"
                    placeholder="HN  /  เลขบัตรประชาชน 13 หลัก  /  ชื่อ-นามสกุล"
                    @keydown="onSearchKeydown"
                    :disabled="loading"
                    autocomplete="off"
                    spellcheck="false"
                />
                <span v-if="linkedName" class="history-tab__linked-name"
                    >← {{ linkedName }}</span
                >
                <button
                    v-if="searchText"
                    class="history-tab__clear-btn"
                    @click="clearSearch"
                    type="button"
                >
                    <X :size="13" />
                </button>
            </div>

            <!-- Range selector -->
            <select
                v-model="selectedRangeIdx"
                class="history-tab__range-select"
                @change="onRangeChange"
                title="ช่วงเวลาประวัติ"
            >
                <option
                    v-for="(opt, idx) in RANGE_OPTIONS"
                    :key="idx"
                    :value="idx"
                >
                    {{ opt.label }}
                </option>
            </select>

            <button
                class="btn btn--primary"
                :disabled="loading || !searchText.trim()"
                @click="doSearch()"
                type="button"
            >
                <Loader v-if="loading" :size="14" class="spin" />
                <Search v-else :size="14" />
                {{ loading ? "กำลังค้นหา…" : "ค้นหา" }}
            </button>
        </div>

        <!-- ── Hint bar ────────────────────────────────────────────────────── -->
        <div
            class="history-tab__hint"
            :class="{ 'hint--ok': hintOk, 'hint--error': hintError }"
        >
            {{ hint }}
        </div>

        <!-- ── Candidate list ── -->
        <div
            v-if="patientCandidates.length > 0 && !showingPatientHistory"
            class="history-candidates"
        >
            <div
                v-for="cand in patientCandidates"
                :key="cand.hn"
                class="history-candidate-row"
                @click="selectCandidate(cand)"
            >
                <User :size="15" class="history-candidate-row__icon" />
                <span class="history-candidate-row__hn">{{
                    hnPadded(cand.hn)
                }}</span>
                <span class="history-candidate-row__name">{{
                    cand.pt_name
                }}</span>
                <ChevronRight :size="13" class="history-candidate-row__arrow" />
            </div>
        </div>

        <!-- ── Patient header (matches SearchTab patient-card__header style) ── -->
        <div
            v-if="(currentHn || linkedName) && showingPatientHistory"
            class="history-patient-header"
        >
            <div class="history-patient-header__avatar">
                <User :size="22" />
            </div>
            <div class="history-patient-header__main">
                <div class="history-patient-header__name">
                    {{ linkedName || hnPadded(currentHn) }}
                </div>
                <div class="history-patient-header__meta">
                    <span class="chip chip--hn"
                        >HN: {{ hnPadded(currentHn) }}</span
                    >
                    <span
                        v-if="
                            (currentRecord?.cid || currentCid) &&
                            (currentRecord?.cid || currentCid) !== '0'
                        "
                        class="chip"
                        >CID: {{ currentRecord?.cid || currentCid }}</span
                    >
                    <span v-if="currentRecord?.current_dept_name" class="chip">
                        {{ currentRecord.current_dept_name }}
                    </span>
                    <span
                        v-if="currentRecord?.pttype_today || currentPttype"
                        class="chip"
                    >
                        สิทธิ์:
                        {{ currentRecord?.pttype_today || currentPttype }}
                    </span>
                </div>
            </div>
            <div
                class="history-patient-header__actions"
                v-if="patientCandidates.length > 0"
            >
                <button
                    class="btn btn--ghost btn--sm"
                    @click="backToSearch"
                    type="button"
                    title="กลับไปยังรายการค้นหา"
                >
                    ← กลับ
                </button>
            </div>
        </div>

        <!-- ── Main content: tree + detail ── -->
        <div
            v-if="showingPatientHistory || patientCandidates.length === 0"
            class="history-tab__body"
        >
            <!-- Tree panel -->
            <div class="tree-panel" v-if="treeData.length > 0 || loading">
                <div v-if="loading" class="tree-panel__loading">
                    <Loader :size="20" class="spin" />
                    <span>กำลังโหลดประวัติ…</span>
                </div>

                <template v-else>
                    <div
                        v-for="year in treeData"
                        :key="year.beYear"
                        class="tree-year"
                    >
                        <!-- Year node -->
                        <button
                            class="tree-node tree-node--year"
                            @click="toggleYear(year)"
                            type="button"
                        >
                            <component
                                :is="year.expanded ? ChevronDown : ChevronRight"
                                :size="14"
                                class="tree-node__arrow"
                            />
                            <Calendar :size="14" class="tree-node__icon" />
                            <span class="tree-node__text">{{
                                year.beYear
                            }}</span>
                            <span class="tree-node__badge"
                                >{{ year.totalVisits }} ครั้ง</span
                            >
                        </button>

                        <!-- Months -->
                        <template v-if="year.expanded">
                            <div
                                v-for="month in year.months"
                                :key="month.monthKey"
                                class="tree-month"
                            >
                                <button
                                    class="tree-node tree-node--month"
                                    @click="toggleMonth(month)"
                                    type="button"
                                >
                                    <component
                                        :is="
                                            month.expanded
                                                ? ChevronDown
                                                : ChevronRight
                                        "
                                        :size="13"
                                        class="tree-node__arrow"
                                    />
                                    <span class="tree-node__text">{{
                                        month.monthLabel
                                    }}</span>
                                    <span
                                        class="tree-node__badge tree-node__badge--sm"
                                        >{{ month.dates.length }} ครั้ง</span
                                    >
                                </button>

                                <!-- Dates -->
                                <template v-if="month.expanded">
                                    <button
                                        v-for="dateNode in month.dates"
                                        :key="dateNode.dateISO"
                                        class="tree-node tree-node--date"
                                        :class="{
                                            'tree-node--selected':
                                                selectedDate ===
                                                dateNode.dateISO,
                                        }"
                                        @click="selectDate(dateNode)"
                                        type="button"
                                    >
                                        <Clock
                                            :size="12"
                                            class="tree-node__icon"
                                        />
                                        <span class="tree-node__text">{{
                                            dateNode.dateLabel
                                        }}</span>
                                        <span
                                            class="tree-node__badge tree-node__badge--xs"
                                            >{{
                                                dateNode.drugs.length
                                            }}
                                            รายการ</span
                                        >
                                    </button>
                                </template>
                            </div>
                        </template>
                    </div>
                </template>
            </div>

            <!-- Empty tree state -->
            <div v-else class="tree-panel tree-panel--empty">
                <Calendar :size="36" />
                <p>ยังไม่มีข้อมูลประวัติ</p>
            </div>

            <!-- Detail panel -->
            <div class="detail-panel">
                <template v-if="selectedDate">
                    <div class="detail-panel__header">
                        <Clock :size="15" />
                        <span
                            >วันที่รับยา:
                            <strong>{{ selectedDateLabel }}</strong></span
                        >
                    </div>
                    <div class="detail-panel__sep"></div>

                    <div class="detail-panel__scroll">
                        <table class="detail-table">
                            <thead>
                                <tr>
                                    <th
                                        class="detail-table__th detail-table__th--name"
                                    >
                                        ชื่อยา
                                    </th>
                                    <th
                                        class="detail-table__th detail-table__th--qty"
                                    >
                                        จำนวน
                                    </th>
                                    <th class="detail-table__th">หน่วย</th>
                                </tr>
                            </thead>
                            <tbody>
                                <tr
                                    v-for="(drug, idx) in selectedDrugs"
                                    :key="idx"
                                    :class="
                                        idx % 2 === 1
                                            ? 'detail-table__row--alt'
                                            : ''
                                    "
                                >
                                    <td
                                        class="detail-table__td detail-table__td--name"
                                    >
                                        {{ drug.drug_name }}
                                    </td>
                                    <td
                                        class="detail-table__td detail-table__td--qty"
                                    >
                                        {{ drug.qty }}
                                    </td>
                                    <td class="detail-table__td">
                                        {{ drug.units }}
                                    </td>
                                </tr>
                            </tbody>
                        </table>
                    </div>

                    <div class="detail-panel__summary">
                        รวม {{ selectedDrugsTotal }} รายการยาสมุนไพร
                    </div>
                </template>

                <div v-else class="detail-panel__empty">
                    <Calendar :size="28" />
                    <p>เลือกวันที่เพื่อดูรายการยา</p>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.history-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 14px 16px;
    gap: 10px;
    overflow: hidden;
    background: #ffffff;
}

/* Toolbar */
.history-tab__toolbar {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-shrink: 0;
    background: #ffffff;
    border-bottom: 1px solid rgba(14, 15, 12, 0.1);
    padding: 12px 16px;
    margin: -14px -16px 0;
}

.history-tab__field-wrap {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1;
}

.history-tab__field-icon {
    position: absolute;
    left: 10px;
    color: #a39e98;
    pointer-events: none;
    display: flex;
}

.history-tab__input {
    width: 100%;
    height: 36px;
    padding: 0 120px 0 34px;
    border: 1px solid rgba(14, 15, 12, 0.15);
    border-radius: 9999px;
    background: #ffffff;
    color: #0e0f0c;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    font-size: 13px;
    outline: none;
    transition: border-color 0.15s;
}
.history-tab__input::placeholder {
    color: #868685;
}
.history-tab__input:focus {
    border-color: #9fe870;
    outline: none;
    box-shadow: 0 0 0 2px rgba(159, 232, 112, 0.25);
}

.history-tab__linked-name {
    position: absolute;
    right: 36px;
    font-size: 11px;
    color: #163300;
    background: #e2f6d5;
    border: 1px solid rgba(159, 232, 112, 0.3);
    border-radius: 9999px;
    padding: 2px 10px;
    pointer-events: none;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 200px;
}

.history-tab__clear-btn {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    cursor: pointer;
    color: #a39e98;
    display: flex;
    align-items: center;
    padding: 2px;
    border-radius: 4px;
}
.history-tab__clear-btn:hover {
    color: #d03238;
}

/* Range dropdown */
.history-tab__range-select {
    height: 32px;
    padding: 0 28px 0 10px;
    border: 1px solid rgba(14, 15, 12, 0.15);
    border-radius: 6px;
    background: #f5f7f3;
    color: #0e0f0c;
    font-size: 12px;
    font-weight: 500;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    cursor: pointer;
    outline: none;
    flex-shrink: 0;
    appearance: auto;
    -webkit-appearance: auto;
}
.history-tab__range-select:focus {
    border-color: #9fe870;
}

/* Hint */
.history-tab__hint {
    font-size: 12px;
    color: #a39e98;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    min-height: 18px;
    flex-shrink: 0;
}
.hint--ok {
    color: #163300;
}
.hint--error {
    color: #d03238;
}

/* ── Candidate list ─────────────────────────────────────── */
.history-candidates {
    border: 1px solid rgba(14, 15, 12, 0.1);
    border-radius: 16px;
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    background: #ffffff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.04);
    flex-shrink: 0;
    max-height: none;
}

.history-candidate-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 14px;
    cursor: pointer;
    font-size: 13px;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    border-bottom: 1px solid rgba(0, 0, 0, 0.05);
    transition: background 0.1s;
}
.history-candidate-row:last-child {
    border-bottom: none;
}
.history-candidate-row:hover {
    background: #f0f7eb;
}
.history-candidate-row__icon {
    color: #a39e98;
    flex-shrink: 0;
}
.history-candidate-row__hn {
    color: #868685;
    font-weight: 600;
    min-width: 75px;
    font-size: 12px;
    flex-shrink: 0;
}
.history-candidate-row__name {
    flex: 1;
    color: #0e0f0c;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.history-candidate-row__cid {
    color: #a39e98;
    font-size: 12px;
    flex-shrink: 0;
}
.history-candidate-row__arrow {
    color: #9fe870;
    flex-shrink: 0;
}

/* ── Patient header (matches SearchTab patient-card__header) ── */
.history-patient-header {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 14px 16px 12px;
    background: #f5f7f3;
    border: 1px solid rgba(14, 15, 12, 0.12);
    border-radius: 12px;
    flex-shrink: 0;
    box-shadow: rgba(14, 15, 12, 0.12) 0px 0px 0px 1px;
}
.history-patient-header__avatar {
    width: 38px;
    height: 38px;
    border-radius: 50%;
    background: #e2f6d5;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    color: #163300;
    border: 1px solid rgba(159, 232, 112, 0.3);
}
.history-patient-header__main {
    flex: 1;
    min-width: 0;
}
.history-patient-header__name {
    font-size: 16px;
    font-weight: 700;
    color: #0e0f0c;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    margin-bottom: 7px;
    line-height: 1.2;
}
.history-patient-header__meta {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
}
.history-patient-header__actions {
    display: flex;
    flex-direction: row;
    gap: 5px;
    flex-shrink: 0;
    align-items: center;
}

/* Body: tree + detail */
.history-tab__body {
    display: flex;
    gap: 12px;
    flex: 1;
    min-height: 0;
    border: 1px solid rgba(14, 15, 12, 0.1);
    border-radius: 12px;
    overflow: hidden;
}

/* Tree panel */
.tree-panel {
    width: 280px;
    flex-shrink: 0;
    overflow-y: auto;
    border: none;
    border-right: 1px solid rgba(14, 15, 12, 0.1);
    border-radius: 0;
    background: #ffffff;
    padding: 6px 0;
}

.tree-panel--empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #a39e98;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    font-size: 13px;
    padding: 20px;
}

.tree-panel__loading {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 24px;
    color: #615d59;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    font-size: 13px;
}

/* Tree nodes */
.tree-node {
    display: flex;
    align-items: center;
    gap: 5px;
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    text-align: left;
    padding: 4px 10px;
    transition: background 0.1s;
    border-radius: 4px;
    color: rgba(0, 0, 0, 0.95);
}
.tree-node:hover {
    background: #f0f7eb;
}

.tree-node--year {
    font-size: 13px;
    font-weight: 700;
    color: rgba(0, 0, 0, 0.95);
}
.tree-node--month {
    font-size: 12px;
    padding-left: 20px;
    color: #615d59;
}
.tree-node--date {
    font-size: 12px;
    padding-left: 36px;
    color: rgba(0, 0, 0, 0.95);
}
.tree-node--selected {
    background: #e2f6d5;
}
.tree-node--selected .tree-node__text {
    color: #163300;
    font-weight: 600;
}

.tree-node__arrow {
    flex-shrink: 0;
    color: #a39e98;
}
.tree-node__icon {
    flex-shrink: 0;
    color: #a39e98;
}
.tree-node__text {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.tree-node__badge {
    font-size: 11px;
    font-weight: 600;
    background: #e2f6d5;
    border: none;
    border-radius: 9999px;
    padding: 1px 7px;
    color: #163300;
    flex-shrink: 0;
}
.tree-node__badge--sm {
    font-size: 11px;
}
.tree-node__badge--xs {
    font-size: 11px;
}

/* Detail panel */
.detail-panel {
    flex: 1;
    min-width: 0;
    border: none;
    border-radius: 0;
    background: #ffffff;
    overflow: hidden;
    display: flex;
    flex-direction: column;
}

.detail-panel__header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    font-size: 13px;
    color: #0e0f0c;
    background: #ffffff;
    border-bottom: 1px solid rgba(14, 15, 12, 0.1);
    flex-shrink: 0;
}
.detail-panel__header strong {
    color: #163300;
}

.detail-panel__sep {
    height: 0;
}

.detail-panel__scroll {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
}

.detail-table {
    width: 100%;
    border-collapse: collapse;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    font-size: 13px;
}

.detail-table__th {
    text-align: left;
    padding: 6px 12px;
    background: #f5f7f3;
    color: #454745;
    font-weight: 600;
    font-size: 12px;
    border-bottom: 1px solid rgba(14, 15, 12, 0.1);
    position: sticky;
    top: 0;
    z-index: 1;
}
.detail-table__th--name {
    width: 60%;
}
.detail-table__th--qty {
    width: 20%;
    text-align: center;
}

.detail-table__td {
    padding: 6px 12px;
    border-bottom: 1px solid rgba(0, 0, 0, 0.05);
    color: rgba(0, 0, 0, 0.9);
    vertical-align: middle;
}
.detail-table__td--name {
    font-weight: 600;
}
.detail-table__td--qty {
    text-align: center;
    color: #163300;
    font-weight: 700;
}
.detail-table__row--alt {
    background: #f9fbf7;
}

.detail-panel__summary {
    padding: 8px 14px;
    font-size: 13px;
    color: #454745;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    border-top: 1px solid rgba(14, 15, 12, 0.1);
    background: #f5f7f3;
    text-align: right;
    flex-shrink: 0;
}

.detail-panel__empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: #a39e98;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    font-size: 13px;
}

/* Shared chips */
.chip--hn {
    background: #e2f6d5;
    border-color: rgba(159, 232, 112, 0.4);
    color: #163300;
    font-weight: 600;
}
.chip {
    display: inline-flex;
    align-items: center;
    background: #f6f5f4;
    border: 1px solid rgba(0, 0, 0, 0.1);
    border-radius: 12px;
    padding: 2px 9px;
    font-size: 11px;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    color: #615d59;
}
.chip--amber {
    background: #fff3cd;
    border-color: #f59e0b;
    color: #92400e;
}

/* Search / action button */
.btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 7px 16px;
    border-radius: 9999px;
    font-size: 13px;
    font-family:
        Inter,
        -apple-system,
        system-ui,
        "Segoe UI",
        Helvetica,
        Arial,
        "Tahoma",
        "TH Sarabun New",
        sans-serif;
    font-weight: 500;
    cursor: pointer;
    transition:
        background 0.15s,
        color 0.15s,
        transform 0.1s;
    border: none;
    outline: none;
}
.btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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

.spin {
    animation: spin 1s linear infinite;
}
@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
</style>
