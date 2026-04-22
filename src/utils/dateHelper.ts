// ─────────────────────────────────────────────────────────────────────────────
// HerbReady — date formatting helpers (Thai calendar)
// ─────────────────────────────────────────────────────────────────────────────

export const THAI_MONTHS_SHORT = [
  'ม.ค.',
  'ก.พ.',
  'มี.ค.',
  'เม.ย.',
  'พ.ค.',
  'มิ.ย.',
  'ก.ค.',
  'ส.ค.',
  'ก.ย.',
  'ต.ค.',
  'พ.ย.',
  'ธ.ค.',
]

/**
 * Format an ISO date string (YYYY-MM-DD) or null to Thai short date.
 * Example: "2025-06-15" → "15 มิ.ย. 2568"
 */
export function formatDateThaiShort(dateStr: string | null): string {
  if (!dateStr) return '—'
  // Accept "YYYY-MM-DD" or Date-parseable strings
  const parts = dateStr.split('T')[0].split('-')
  if (parts.length < 3) return dateStr
  const [y, m, d] = parts.map(Number)
  if (!y || !m || !d) return dateStr
  const beYear = y + 543
  const monthLabel = THAI_MONTHS_SHORT[m - 1] ?? `เดือน${m}`
  return `${d} ${monthLabel} ${beYear}`
}

/**
 * Returns today's date as ISO string YYYY-MM-DD (local time).
 */
export function getCurrentDateISO(): string {
  const now = new Date()
  const y = now.getFullYear()
  const m = String(now.getMonth() + 1).padStart(2, '0')
  const d = String(now.getDate()).padStart(2, '0')
  return `${y}-${m}-${d}`
}

/** Alias for getCurrentDateISO(). */
export const todayISO = getCurrentDateISO
