<script setup lang="ts">
/**
 * SettingsDialog — Database connection settings.
 */
import { ref, onMounted } from "vue";
import {
    Database,
    Eye,
    EyeOff,
    X,
    CheckCircle,
    AlertCircle,
    Loader,
} from "lucide-vue-next";
import { api } from "../api/tauri";
import { connectToDatabase, dbConfig } from "../stores/connection";

const emit = defineEmits<{
    (e: "close"): void;
    (e: "saved"): void;
}>();

// Form fields
const host = ref("localhost");
const port = ref(5432);
const dbName = ref("");
const user = ref("postgres");
const password = ref("");
const showPw = ref(false);

// Test state
const testStatus = ref<"idle" | "testing" | "ok" | "error">("idle");
const testMessage = ref("");

// Save state
const saving = ref(false);

onMounted(async () => {
    try {
        const cfg = await api.getDbConfig();
        host.value = cfg.host;
        port.value = cfg.port;
        dbName.value = cfg.name;
        user.value = cfg.user;
        password.value = cfg.password;
    } catch {
        // use defaults
    }
});

async function testConnection() {
    testStatus.value = "testing";
    testMessage.value = "กำลังทดสอบ…";
    try {
        const msg = await api.testConnection(
            host.value,
            port.value,
            dbName.value,
            user.value,
            password.value,
        );
        testStatus.value = "ok";
        testMessage.value = msg || "เชื่อมต่อสำเร็จ";
    } catch (err: unknown) {
        testStatus.value = "error";
        testMessage.value = err instanceof Error ? err.message : String(err);
    }
}

async function save() {
    saving.value = true;
    try {
        await api.saveDbConfig(
            host.value,
            port.value,
            dbName.value,
            user.value,
            password.value,
        );
        dbConfig.value = {
            host: host.value,
            port: port.value,
            name: dbName.value,
            user: user.value,
            password: password.value,
        };
        await connectToDatabase();
        emit("saved");
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
        <div class="dialog">
            <!-- Header -->
            <div class="dialog__header">
                <Database :size="18" />
                <span class="dialog__title">ตั้งค่าการเชื่อมต่อฐานข้อมูล</span>
                <button
                    class="dialog__close"
                    @click="emit('close')"
                    type="button"
                >
                    <X :size="16" />
                </button>
            </div>

            <!-- Body -->
            <div class="dialog__body">
                <div class="form-row">
                    <label class="form-label">Host</label>
                    <input
                        v-model="host"
                        class="form-input"
                        type="text"
                        placeholder="localhost"
                    />
                </div>
                <div class="form-row">
                    <label class="form-label">Port</label>
                    <input
                        v-model.number="port"
                        class="form-input form-input--short"
                        type="number"
                        placeholder="5432"
                    />
                </div>
                <div class="form-row">
                    <label class="form-label">Database</label>
                    <input
                        v-model="dbName"
                        class="form-input"
                        type="text"
                        placeholder="ชื่อฐานข้อมูล"
                    />
                </div>
                <div class="form-row">
                    <label class="form-label">User</label>
                    <input
                        v-model="user"
                        class="form-input"
                        type="text"
                        placeholder="postgres"
                    />
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
                        <button
                            class="pw-toggle"
                            @click="showPw = !showPw"
                            type="button"
                        >
                            <EyeOff v-if="showPw" :size="14" />
                            <Eye v-else :size="14" />
                        </button>
                    </div>
                </div>

                <!-- Test result -->
                <div
                    v-if="testStatus !== 'idle'"
                    class="test-result"
                    :class="`test-result--${testStatus}`"
                >
                    <Loader
                        v-if="testStatus === 'testing'"
                        :size="14"
                        class="spin"
                    />
                    <CheckCircle v-else-if="testStatus === 'ok'" :size="14" />
                    <AlertCircle
                        v-else-if="testStatus === 'error'"
                        :size="14"
                    />
                    <span>{{ testMessage }}</span>
                </div>
            </div>

            <!-- Footer -->
            <div class="dialog__footer">
                <button
                    class="btn btn--ghost"
                    @click="testConnection"
                    :disabled="testStatus === 'testing'"
                    type="button"
                >
                    <Loader
                        v-if="testStatus === 'testing'"
                        :size="13"
                        class="spin"
                    />
                    ทดสอบการเชื่อมต่อ
                </button>
                <span class="dialog__footer-spacer" />
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
                    บันทึก &amp; เชื่อมต่อ
                </button>
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
    background: #ffffff;
    border: 1px solid rgba(14, 15, 12, 0.12);
    border-radius: 16px;
    width: 440px;
    max-width: 95vw;
    box-shadow: rgba(14, 15, 12, 0.12) 0px 8px 32px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
}

.dialog__header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 16px;
    background: #f5f7f3;
    border-bottom: 1px solid rgba(14, 15, 12, 0.1);
    color: #163300;
}

.dialog__title {
    flex: 1;
    font-size: 15px;
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
}

.dialog__close {
    background: none;
    border: none;
    cursor: pointer;
    color: #a39e98;
    display: flex;
    align-items: center;
    padding: 3px;
    border-radius: 5px;
}
.dialog__close:hover {
    background: rgba(14, 15, 12, 0.07);
    color: rgba(0, 0, 0, 0.95);
}

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
    font-weight: 500;
}

.form-input {
    flex: 1;
    height: 34px;
    padding: 0 10px;
    border: 1px solid rgba(14, 15, 12, 0.15);
    border-radius: 8px;
    background: #ffffff;
    color: #0e0f0c;
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
    outline: none;
    transition: border-color 0.15s;
}
.form-input::placeholder {
    color: #868685;
}
.form-input:focus {
    border-color: #9fe870;
}
.form-input--short {
    max-width: 100px;
}

.pw-wrap {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
}
.pw-wrap .form-input {
    flex: 1;
    padding-right: 36px;
}

.pw-toggle {
    position: absolute;
    right: 8px;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
    align-items: center;
}

.test-result {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 12px;
    border-radius: 8px;
    font-size: 13px;
    font-family: var(--font-thai);
}
.test-result--testing {
    background: #f5f7f3;
    color: #454745;
}
.test-result--ok {
    background: #e2f6d5;
    color: #163300;
    border: 1px solid rgba(159, 232, 112, 0.5);
}
.test-result--error {
    background: #fde8e8;
    color: #d03238;
    border: 1px solid rgba(208, 50, 56, 0.2);
}

.dialog__footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid rgba(14, 15, 12, 0.1);
    background: #f5f7f3;
}

.dialog__footer-spacer {
    flex: 1;
}

.btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 8px 16px;
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

.btn--ghost {
    background: transparent;
    color: #454745;
    border: 1px solid rgba(14, 15, 12, 0.15);
}
.btn--ghost:hover:not(:disabled) {
    background: rgba(14, 15, 12, 0.06);
    transform: scale(1.05);
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
