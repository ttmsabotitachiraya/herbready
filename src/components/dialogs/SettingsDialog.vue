<script setup lang="ts">
/**
 * SettingsDialog — Database connection settings.
 * Uses api/tauri.ts and stores/connection.ts.
 */
import { ref, onMounted } from 'vue'
import { Database, Eye, EyeOff, X, CheckCircle, AlertCircle, Loader } from 'lucide-vue-next'
import { api } from '../../api/tauri'
import { connectToDatabase, dbConfig } from '../../stores/connection'

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'saved'): void
}>()

const host     = ref('localhost')
const port     = ref(5432)
const dbName   = ref('')
const user     = ref('postgres')
const password = ref('')
const showPw   = ref(false)

const testStatus  = ref<'idle' | 'testing' | 'ok' | 'error'>('idle')
const testMessage = ref('')
const saving = ref(false)

onMounted(async () => {
  try {
    const cfg = await api.getDbConfig()
    host.value     = cfg.host
    port.value     = cfg.port
    dbName.value   = cfg.name
    user.value     = cfg.user
    password.value = cfg.password
  } catch {
    // use defaults
  }
})

async function testConnection() {
  testStatus.value = 'testing'
  testMessage.value = 'กำลังทดสอบ…'
  try {
    const msg = await api.testConnection(host.value, port.value, dbName.value, user.value, password.value)
    testStatus.value = 'ok'
    testMessage.value = typeof msg === 'string' ? msg : 'เชื่อมต่อสำเร็จ'
  } catch (err: unknown) {
    testStatus.value = 'error'
    testMessage.value = err instanceof Error ? err.message : String(err)
  }
}

async function save() {
  saving.value = true
  try {
    await api.saveDbConfig(host.value, port.value, dbName.value, user.value, password.value)
    dbConfig.value = { host: host.value, port: port.value, name: dbName.value, user: user.value, password: password.value }
    await connectToDatabase()
    emit('saved')
    emit('close')
  } catch (err: unknown) {
    testStatus.value = 'error'
    testMessage.value = `บันทึกล้มเหลว: ${err instanceof Error ? err.message : String(err)}`
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog" role="dialog" aria-modal="true">
      <!-- Header -->
      <div class="dialog__header">
        <Database :size="18" />
        <span class="dialog__title">ตั้งค่าการเชื่อมต่อฐานข้อมูล</span>
        <button class="dialog__close" @click="emit('close')" type="button">
          <X :size="16" />
        </button>
      </div>

      <!-- Body -->
      <div class="dialog__body">
        <div class="form-row">
          <label class="form-label">Host</label>
          <input v-model="host" class="form-input" type="text" placeholder="localhost" />
        </div>
        <div class="form-row">
          <label class="form-label">Port</label>
          <input v-model.number="port" class="form-input form-input--short" type="number" placeholder="5432" />
        </div>
        <div class="form-row">
          <label class="form-label">Database</label>
          <input v-model="dbName" class="form-input" type="text" placeholder="ชื่อฐานข้อมูล" />
        </div>
        <div class="form-row">
          <label class="form-label">User</label>
          <input v-model="user" class="form-input" type="text" placeholder="postgres" />
        </div>
        <div class="form-row">
          <label class="form-label">Password</label>
          <div class="pw-wrap">
            <input
              v-model="password"
              class="form-input"
              :type="showPw ? 'text' : 'password'"
              placeholder="รหัสผ่าน"
            />
            <button class="pw-toggle" @click="showPw = !showPw" type="button">
              <EyeOff v-if="showPw" :size="14" />
              <Eye v-else :size="14" />
            </button>
          </div>
        </div>

        <!-- Test result banner -->
        <div v-if="testStatus !== 'idle'" class="test-result" :class="`test-result--${testStatus}`">
          <Loader v-if="testStatus === 'testing'" :size="14" class="spin" />
          <CheckCircle v-else-if="testStatus === 'ok'" :size="14" />
          <AlertCircle v-else-if="testStatus === 'error'" :size="14" />
          <span>{{ testMessage }}</span>
        </div>
      </div>

      <!-- Footer -->
      <div class="dialog__footer">
        <button
          class="btn btn--ghost"
          type="button"
          :disabled="testStatus === 'testing' || saving"
          @click="testConnection"
        >
          <Loader v-if="testStatus === 'testing'" :size="13" class="spin" />
          ทดสอบการเชื่อมต่อ
        </button>
        <span class="spacer" />
        <button class="btn btn--ghost" @click="emit('close')" type="button" :disabled="saving">ยกเลิก</button>
        <button class="btn btn--primary" @click="save" :disabled="saving || testStatus === 'testing'" type="button">
          <Loader v-if="saving" :size="13" class="spin" />
          บันทึก &amp; เชื่อมต่อ
        </button>
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
  z-index: 1000;
  font-family: var(--font-thai, 'Sarabun', 'Tahoma', Arial, sans-serif);
}

.dialog {
  background: var(--bg-input, #fff);
  border: 1.5px solid var(--border-default, #BFEFD3);
  border-radius: 14px;
  width: 440px;
  max-width: 95vw;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.2);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.dialog__header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 14px 16px;
  background: var(--bg-surface, #E8FFF1);
  border-bottom: 1px solid var(--border-default, #BFEFD3);
  color: var(--accent-primary-dark, #2FAE66);
}

.dialog__title {
  flex: 1;
  font-size: 15px;
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
.dialog__close:hover { background: var(--bg-elevated, #D6F8E4); color: var(--text-primary, #1B2B23); }

.dialog__body {
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.form-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.form-label {
  width: 80px;
  flex-shrink: 0;
  font-size: 13px;
  color: var(--text-secondary, #355F4A);
  font-weight: 600;
}

.form-input {
  flex: 1;
  height: 34px;
  padding: 0 10px;
  border: 1.5px solid var(--border-default, #BFEFD3);
  border-radius: 7px;
  background: var(--bg-window, #F7FFF9);
  color: var(--text-primary, #1B2B23);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s;
}
.form-input:focus { border-color: var(--border-focus, #34C38F); }
.form-input--short { max-width: 110px; flex: none; }

.pw-wrap {
  flex: 1;
  position: relative;
  display: flex;
  align-items: center;
}
.pw-wrap .form-input { flex: 1; padding-right: 38px; }

.pw-toggle {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  cursor: pointer;
  color: var(--text-muted, #6FAF95);
  display: flex;
  align-items: center;
}
.pw-toggle:hover { color: var(--text-secondary, #355F4A); }

.test-result {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 13px;
}
.test-result--testing { background: var(--bg-elevated, #D6F8E4); color: var(--text-muted, #6FAF95); }
.test-result--ok      { background: #D9FBE8; color: #14532D; border: 1px solid #4CCB7A; }
.test-result--error   { background: #FEE2E2; color: #B91C1C; border: 1px solid #FCA5A5; }

.dialog__footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-top: 1px solid var(--border-default, #BFEFD3);
  background: var(--bg-surface, #E8FFF1);
}
.spacer { flex: 1; }

.btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 32px;
  padding: 0 14px;
  border-radius: 7px;
  font-size: 13px;
  font-family: inherit;
  font-weight: 500;
  cursor: pointer;
  border: 1.5px solid transparent;
  transition: all 0.15s;
  white-space: nowrap;
}
.btn:disabled { opacity: 0.6; cursor: not-allowed; }

.btn--primary {
  background: var(--accent-primary, #66DE93);
  border-color: var(--accent-primary-dim, #4CCB7A);
  color: #14532D;
}
.btn--primary:hover:not(:disabled) { background: var(--accent-primary-dim, #4CCB7A); }

.btn--ghost {
  background: transparent;
  border-color: var(--border-default, #BFEFD3);
  color: var(--text-muted, #6FAF95);
}
.btn--ghost:hover:not(:disabled) { background: var(--bg-elevated, #D6F8E4); color: var(--text-primary, #1B2B23); }

@keyframes spin { to { transform: rotate(360deg); } }
.spin { animation: spin 1s linear infinite; }
</style>
