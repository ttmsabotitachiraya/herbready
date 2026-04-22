<script setup lang="ts">
/**
 * App.vue — Root component for HerbReady Tauri app.
 * Manages: tab navigation, menu bar, dialogs, cross-tab communication.
 */
import { ref, computed, onMounted } from "vue";
import { Building2, Database, Settings } from "lucide-vue-next";
import HerbReadyLogo from "./components/HerbReadyLogo.vue";
import type { PatientRecord } from "./types";

// Stores
import { loadDbConfig } from "./stores/connection";
import { loadAppConfig, appConfig } from "./stores/appConfig";

// ── Expose dialogs so ConnectionBar buttons can open them ─────────────────

// Components
import SearchTab from "./components/SearchTab.vue";
import HistoryTab from "./components/HistoryTab.vue";
import DailyTab from "./components/DailyTab.vue";
import ConnectionBar from "./components/ConnectionBar.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import AppSettingsDialog from "./components/AppSettingsDialog.vue";

// ── Tabs ──────────────────────────────────────────────────────────────────
const tabs = [
    { label: "ประมวลผลรายวัน", index: 0 },
    { label: "ค้นหารายบุคคล", index: 1 },
    { label: "ประวัติรับยาสมุนไพร", index: 2 },
];
const activeTab = ref(0);

// ── Department display ────────────────────────────────────────────────────
const deptLabel = computed(() => {
    const depts = appConfig.value.departments;
    if (!depts || depts.length === 0) return null;
    if (depts.length <= 2) return depts.map((d) => d.name || d.code).join(", ");
    return `${depts
        .slice(0, 2)
        .map((d) => d.name || d.code)
        .join(", ")} +${depts.length - 2}`;
});

const deptTooltip = computed(() =>
    appConfig.value.departments.map((d) => d.name || d.code).join(", "),
);

// ── Dialog visibility ─────────────────────────────────────────────────────
const showSettings = ref(false);
const showAppSettings = ref(false);

// ── HistoryTab ref (for cross-tab patient handoff) ────────────────────────
const historyTabRef = ref<InstanceType<typeof HistoryTab> | null>(null);

// ── Cross-tab communication ───────────────────────────────────────────────
function onPatientSelected(_record: PatientRecord) {
    // Called when SearchTab selects a patient — no automatic tab switch here,
    // user can explicitly click "ประวัติรับยา" button in the card.
}

function onViewHistory(record: PatientRecord) {
    activeTab.value = 2;
    // Give the HistoryTab a tick to mount/show before calling its method
    setTimeout(() => {
        historyTabRef.value?.setPatientFromLink(
            record.hn,
            record.pt_name,
            record,
        );
    }, 50);
}

// ── Lifecycle ─────────────────────────────────────────────────────────────
onMounted(async () => {
    await Promise.all([loadDbConfig(), loadAppConfig()]);
});

function onSettingsSaved() {
    // Reconnection is handled inside SettingsDialog; nothing extra needed here.
}

// Exposed so ConnectionBar can trigger dialogs via provide/inject
function openDbSettings() {
    showSettings.value = true;
}
function openAppSettings() {
    showAppSettings.value = true;
}

import { provide } from "vue";
provide("openDbSettings", openDbSettings);
provide("openAppSettings", openAppSettings);
</script>

<template>
    <div class="app-container">
        <!-- ══ Menu bar ══════════════════════════════════════════════════════ -->
        <header class="menu-bar">
            <span class="menu-bar__app-name">
                <HerbReadyLogo :size="18" style="flex-shrink: 0" />
                HerbReady
            </span>

            <span class="menu-bar__spacer" />

            <button
                class="menu-bar__btn"
                @click="openDbSettings"
                type="button"
                title="ตั้งค่าฐานข้อมูล"
            >
                <Database :size="12" />
                ฐานข้อมูล
            </button>
            <button
                class="menu-bar__btn"
                @click="openAppSettings"
                type="button"
                title="ตั้งค่าแอป"
            >
                <Settings :size="12" />
                ตั้งค่า
            </button>

            <span
                v-if="deptLabel"
                class="menu-bar__dept-badge"
                :title="deptTooltip"
            >
                <Building2 :size="13" style="flex-shrink: 0" />
                {{ deptLabel }}
            </span>
        </header>

        <!-- ══ Tab bar ═══════════════════════════════════════════════════════ -->
        <nav class="tab-bar" role="tablist">
            <button
                v-for="tab in tabs"
                :key="tab.index"
                class="tab-btn"
                :class="{ 'tab-btn--active': activeTab === tab.index }"
                role="tab"
                :aria-selected="activeTab === tab.index"
                @click="activeTab = tab.index"
                type="button"
            >
                {{ tab.label }}
            </button>
        </nav>

        <!-- ══ Tab content ═══════════════════════════════════════════════════ -->
        <main class="tab-content">
            <DailyTab v-show="activeTab === 0" style="height: 100%" />

            <SearchTab
                v-show="activeTab === 1"
                @patient-selected="onPatientSelected"
                @view-history="onViewHistory"
                style="height: 100%; overflow-y: auto"
            />

            <HistoryTab
                v-show="activeTab === 2"
                ref="historyTabRef"
                style="height: 100%"
            />
        </main>

        <!-- ══ Connection bar ════════════════════════════════════════════════ -->
        <ConnectionBar />

        <!-- ══ Dialogs ═══════════════════════════════════════════════════════ -->
        <SettingsDialog
            v-if="showSettings"
            @close="showSettings = false"
            @saved="onSettingsSaved"
        />

        <AppSettingsDialog
            v-if="showAppSettings"
            @close="showAppSettings = false"
        />
    </div>
</template>

<style>
/* Global styles imported from index.css are injected via main.ts.
   Only minimal overrides here. */

.menu-bar__btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 8px;
    border-radius: 4px;
    border: none;
    background: none;
    color: rgba(0, 0, 0, 0.55);
    font-size: 11px;
    font-weight: 500;
    font-family: var(--font-sans);
    cursor: pointer;
    transition:
        background 0.1s,
        color 0.1s;
    white-space: nowrap;
}
.menu-bar__btn:hover {
    background: rgba(14, 15, 12, 0.07);
    color: rgba(14, 15, 12, 0.85);
}

.menu-bar__dept-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 10px;
    border-radius: 9999px;
    background: #e2f6d5;
    border: 1px solid rgba(159, 232, 112, 0.5);
    color: #163300;
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    max-width: 260px;
    overflow: hidden;
    text-overflow: ellipsis;
    cursor: default;
    font-family: var(--font-sans);
}
</style>
