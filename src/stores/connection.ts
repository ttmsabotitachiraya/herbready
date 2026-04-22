// ─────────────────────────────────────────────────────────────────────────────
// HerbReady — database connection composable / global state
// ─────────────────────────────────────────────────────────────────────────────
import { ref } from "vue";
import type { ConnectionStatus, DatabaseConfig } from "../types";
import { api } from "../api/tauri";

// Reactive singletons — shared across all components
export const connectionStatus = ref<ConnectionStatus>("disconnected");
export const connectionMessage = ref<string>("");
export const dbConfig = ref<DatabaseConfig | null>(null);

/**
 * Connect (or reconnect) to the PostgreSQL database.
 * If cfg is omitted, uses the last loaded dbConfig value.
 */
export async function connectToDatabase(cfg?: DatabaseConfig): Promise<void> {
  const config = cfg ?? dbConfig.value;
  if (!config) {
    connectionStatus.value = "error";
    connectionMessage.value = "ไม่พบการตั้งค่าฐานข้อมูล";
    return;
  }

  connectionStatus.value = "connecting";
  connectionMessage.value = "กำลังเชื่อมต่อ…";

  try {
    const msg = await api.connectDb(
      config.host,
      config.port,
      config.name,
      config.user,
      config.password,
    );
    connectionStatus.value = "connected";
    connectionMessage.value = msg ?? "เชื่อมต่อสำเร็จ";
  } catch (err: unknown) {
    connectionStatus.value = "error";
    connectionMessage.value = err instanceof Error ? err.message : String(err);
  }
}

/**
 * Load db config from Rust backend and attempt connection.
 */
export async function loadDbConfig(): Promise<void> {
  try {
    const cfg = await api.getDbConfig();
    dbConfig.value = cfg;
    // Auto-connect after loading config
    await connectToDatabase(cfg);
  } catch (err: unknown) {
    connectionStatus.value = 'error'
    connectionMessage.value =
      err instanceof Error ? err.message : 'โหลดการตั้งค่าฐานข้อมูลล้มเหลว'
  }
}
