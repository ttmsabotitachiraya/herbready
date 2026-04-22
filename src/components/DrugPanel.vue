<script setup lang="ts">
/**
 * DrugPanel — interactive drug selection grid for the Daily tab.
 * Shows all eligible + never-dispensed drugs as toggleable pills.
 * Always shows ALL drugs from appConfig (missing ones go into "never" group).
 */
import { computed, watch } from "vue";
import type { PatientRecord } from "../types";
import {
    parseSimpleDrugList,
    parseNotYetDrugList,
    thaiSortKey,
} from "../utils/drugParser";
import { appConfig, getAbbrByName, isDisabledDrug } from "../stores/appConfig";
import DrugPill from "./DrugPill.vue";

const props = defineProps<{
    record: PatientRecord;
    processDate?: string; // YYYY-MM-DD from parent, used to compute ready date
}>();

const emit = defineEmits<{
    (e: "update:record", record: PatientRecord): void;
    (e: "countChanged", count: number): void;
}>();

const eligibleDrugs = computed(() =>
    parseSimpleDrugList(props.record.eligible_drugs_raw).filter(
        (item) => !isDisabledDrug(item.drug_name),
    ),
);
const neverDrugs = computed(() =>
    parseSimpleDrugList(props.record.never_dispensed_drugs_raw).filter(
        (item) => !isDisabledDrug(item.drug_name),
    ),
);
const notYetDrugs = computed(() =>
    parseNotYetDrugList(props.record.not_yet_eligible_drugs_raw).filter(
        (item) => !isDisabledDrug(item.drug_name),
    ),
);

function sortedDrugs<T extends { drug_name: string }>(list: T[]): T[] {
    return [...list].sort((a, b) => {
        const ka = thaiSortKey(getAbbrByName(a.drug_name) || a.drug_name);
        const kb = thaiSortKey(getAbbrByName(b.drug_name) || b.drug_name);
        return ka < kb ? -1 : ka > kb ? 1 : 0;
    });
}

/** Normalize drug name for fuzzy comparison */
function normKey(s: string): string {
    return s.trim().replace(/\s+/g, " ").toLowerCase();
}

/** Check if a config drug name is already represented in the patient's drug lists */
function isDrugRepresented(
    configName: string,
    patientDrugNames: string[],
): boolean {
    const cn = normKey(configName);
    return patientDrugNames.some((pn) => {
        const pk = normKey(pn);
        return (
            cn.startsWith(pk) ||
            pk.startsWith(cn) ||
            cn.includes(pk) ||
            pk.includes(cn)
        );
    });
}

/** All patient drug names across all three categories */
const allPatientDrugNames = computed(() => [
    ...eligibleDrugs.value.map((d) => d.drug_name),
    ...neverDrugs.value.map((d) => d.drug_name),
    ...notYetDrugs.value.map((d) => d.drug_name),
]);

/** Config drugs that are not represented in any patient category → treat as "never" */
const missingConfigDrugs = computed(() =>
    appConfig.value.drugs
        .filter(
            (cfg) =>
                cfg.enabled !== false &&
                !isDrugRepresented(cfg.drug_name, allPatientDrugNames.value),
        )
        .map((cfg) => ({
            drug_name: cfg.drug_name,
            days_remaining: null as null,
        })),
);

/** Full "never" list = patient never + missing config drugs */
const allNeverDrugs = computed(() => [
    ...neverDrugs.value,
    ...missingConfigDrugs.value,
]);

const allClickableDrugs = computed(() => [
    ...sortedDrugs(eligibleDrugs.value),
    ...sortedDrugs(allNeverDrugs.value),
]);

/** Compute the date a not-yet drug becomes eligible */
function computeReadyDate(daysRemaining: number | null): string | undefined {
    if (daysRemaining == null) return undefined;
    const base = props.processDate
        ? new Date(props.processDate + "T00:00:00")
        : new Date();
    base.setDate(base.getDate() + daysRemaining);
    return base.toISOString().split("T")[0];
}

const notYetSorted = computed(() => sortedDrugs(notYetDrugs.value));

const selectedCount = computed(
    () => Object.values(props.record.drug_selection).filter(Boolean).length,
);

watch(selectedCount, (n) => emit("countChanged", n));

function isSelected(drugName: string): boolean {
    return props.record.drug_selection[drugName] ?? false;
}

function toggleDrug(drugName: string) {
    const newSelection = { ...props.record.drug_selection };
    newSelection[drugName] = !newSelection[drugName];
    emit("update:record", { ...props.record, drug_selection: newSelection });
}
</script>

<template>
    <div class="drug-panel">
        <div class="drug-panel__pills">
            <!-- Eligible (green) -->
            <DrugPill
                v-for="item in sortedDrugs(eligibleDrugs)"
                :key="'e-' + item.drug_name"
                :drug-name="item.drug_name"
                :abbr="getAbbrByName(item.drug_name)"
                status="eligible"
                :selected="isSelected(item.drug_name)"
                :clickable="true"
                @toggle="toggleDrug"
            />
            <!-- Never dispensed + missing config drugs (white) -->
            <DrugPill
                v-for="item in sortedDrugs(allNeverDrugs)"
                :key="'n-' + item.drug_name"
                :drug-name="item.drug_name"
                :abbr="getAbbrByName(item.drug_name)"
                status="never"
                :selected="isSelected(item.drug_name)"
                :clickable="true"
                @toggle="toggleDrug"
            />
            <!-- Not yet (red, not clickable) -->
            <DrugPill
                v-for="item in notYetSorted"
                :key="'ny-' + item.drug_name"
                :drug-name="item.drug_name"
                :abbr="getAbbrByName(item.drug_name)"
                status="not_yet"
                :days-remaining="item.days_remaining"
                :last-dispense-date="item.last_dispense_date"
                :ready-date="computeReadyDate(item.days_remaining)"
                :clickable="false"
            />

            <span
                v-if="
                    allClickableDrugs.length === 0 && notYetSorted.length === 0
                "
                class="drug-panel__empty"
            >
                —
            </span>
        </div>
    </div>
</template>

<style scoped>
.drug-panel {
    display: flex;
    align-items: center;
    min-height: 30px;
    overflow: hidden;
}

.drug-panel__pills {
    display: flex;
    /* Force a single horizontal row; allow horizontal scrolling when needed */
    flex-wrap: nowrap;
    gap: 3px;
    align-items: center;
    padding: 2px 0;
    min-width: 0;
    width: 100%;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: thin;
}

/* Allow the child pill components to shrink and truncate their text when space is tight.
   Use the deep selector so these rules apply to the scoped child component DOM. */
:deep(.drug-pill) {
    /* allow pills to shrink, but keep a small minimum for tappable target */
    flex: 0 1 auto;
    min-width: 28px;
    max-width: 120px;
}

/* Truncate the displayed label inside each pill with ellipsis when it overflows */
:deep(.drug-pill__abbr) {
    display: inline-block;
    max-width: 68px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.drug-panel__empty {
    color: var(--text-muted);
    font-size: 13px;
}

.drug-panel__count {
    background: #0075de;
    color: white;
    font-size: 11px;
    font-weight: 700;
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
    border-radius: 12px;
    padding: 1px 7px;
    min-width: 22px;
    text-align: center;
    flex-shrink: 0;
}
</style>
