<script setup lang="ts">
/**
 * PatientPickerDialog — Modal list of matched patients for user to select.
 */
import { X, User } from 'lucide-vue-next'
import type { PatientRecord } from '../../types'

defineProps<{
  records: PatientRecord[]
}>()

const emit = defineEmits<{
  (e: 'selected', record: PatientRecord): void
  (e: 'cancel'): void
}>()
</script>

<template>
  <div class="overlay" @click.self="emit('cancel')">
    <div class="dialog" role="dialog" aria-modal="true">
      <!-- Header -->
      <div class="dialog__header">
        <User :size="16" />
        <span class="dialog__title">พบผู้ป่วย {{ records.length }} ราย — กรุณาเลือก</span>
        <button class="dialog__close" type="button" @click="emit('cancel')">
          <X :size="15" />
        </button>
      </div>

      <!-- Table -->
      <div class="dialog__body">
        <table class="picker-table">
          <thead>
            <tr>
              <th>HN</th>
              <th>ชื่อ-นามสกุล</th>
              <th>เลขบัตรประชาชน</th>
              <th>สิทธิ์</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="record in records"
              :key="record.hn"
              class="picker-row"
              @click="emit('selected', record)"
            >
              <td class="col-hn">{{ record.hn }}</td>
              <td>{{ record.pt_name }}</td>
              <td class="col-cid">{{ record.cid || '—' }}</td>
              <td>
                <span class="pttype-badge">{{ record.pttype_today }}</span>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Footer -->
      <div class="dialog__footer">
        <button class="btn btn--ghost" type="button" @click="emit('cancel')">ยกเลิก</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1100;
  font-family: var(--font-thai, 'Sarabun', 'Tahoma', Arial, sans-serif);
}

.dialog {
  background: var(--bg-input, #fff);
  border: 1.5px solid var(--border-default, #BFEFD3);
  border-radius: 14px;
  width: 580px;
  max-width: 95vw;
  max-height: 70vh;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.dialog__header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 13px 16px;
  background: var(--bg-surface, #E8FFF1);
  border-bottom: 1px solid var(--border-default, #BFEFD3);
  color: var(--accent-primary-dark, #2FAE66);
}

.dialog__title {
  flex: 1;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary, #1B2B23);
}

.dialog__close {
  background: none;
  border: none;
  cursor: pointer;
  color: var(--text-muted, #6FAF95);
  display: flex;
  align-items: center;
  padding: 3px;
  border-radius: 5px;
}
.dialog__close:hover {
  background: var(--bg-elevated, #D6F8E4);
  color: var(--text-primary, #1B2B23);
}

.dialog__body {
  flex: 1;
  overflow-y: auto;
}

.picker-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

.picker-table thead tr {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--bg-elevated, #D6F8E4);
}

.picker-table th {
  padding: 8px 12px;
  text-align: left;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary, #355F4A);
  border-bottom: 1px solid var(--border-default, #BFEFD3);
}

.picker-table td {
  padding: 9px 12px;
  border-bottom: 1px solid var(--bg-elevated, #D6F8E4);
  color: var(--text-primary, #1B2B23);
}

.picker-row {
  cursor: pointer;
  transition: background 0.1s;
}
.picker-row:hover td {
  background: var(--bg-surface, #E8FFF1);
}

.col-hn {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary, #355F4A);
  width: 80px;
}
.col-cid {
  font-size: 12px;
  color: var(--text-muted, #6FAF95);
  width: 140px;
}

.pttype-badge {
  display: inline-block;
  background: var(--bg-elevated, #D6F8E4);
  border: 1px solid var(--border-default, #BFEFD3);
  color: var(--text-secondary, #355F4A);
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 10px;
  font-weight: 600;
}

.dialog__footer {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  padding: 10px 16px;
  border-top: 1px solid var(--border-default, #BFEFD3);
  background: var(--bg-surface, #E8FFF1);
}

.btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 32px;
  padding: 0 16px;
  border-radius: 7px;
  font-size: 13px;
  font-family: inherit;
  font-weight: 500;
  cursor: pointer;
  border: 1.5px solid transparent;
  transition: all 0.15s;
}
.btn--ghost {
  background: transparent;
  border-color: var(--border-default, #BFEFD3);
  color: var(--text-muted, #6FAF95);
}
.btn--ghost:hover {
  background: var(--bg-elevated, #D6F8E4);
  color: var(--text-primary, #1B2B23);
}
</style>
