// ─────────────────────────────────────────────────────────────────────────────
// HerbReady — drug string parsers and Thai sort helpers
// ─────────────────────────────────────────────────────────────────────────────
import type { DrugItem, PatientRecord } from "../types";

// ---------------------------------------------------------------------------
// Parse helpers
// ---------------------------------------------------------------------------

/**
 * Parse a comma-separated drug name list with no eligibility info.
 * e.g. "ฟ้าทะลายโจร  500 mg., มะขามแขก, ขิง"
 * → [{drug_name: "ฟ้าทะลายโจร  500 mg.", days_remaining: null}, ...]
 */
export function parseSimpleDrugList(raw: string): DrugItem[] {
  if (!raw || !raw.trim()) return [];
  return raw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean)
    .map((drug_name) => ({ drug_name, days_remaining: null }));
}

/**
 * Parse a comma-separated "not yet eligible" drug list.
 * Each entry has the form: "ขมิ้นชัน 500 mg. (in 5 days|last:2024-11-01)"
 * or legacy: "ขมิ้นชัน 500 mg. (in 5 days)"
 * → [{drug_name: "ขมิ้นชัน 500 mg.", days_remaining: 5, last_dispense_date: "2024-11-01"}, ...]
 */
export function parseNotYetDrugList(raw: string): DrugItem[] {
  if (!raw || !raw.trim()) return [];
  return raw
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean)
    .map((entry) => {
      // New format: "drug_name (in N days|last:YYYY-MM-DD)"
      const matchNew = entry.match(
        /^(.+?)\s*\(in\s+(\d+)\s+days\|last:(\d{4}-\d{2}-\d{2})\)\s*$/i,
      );
      if (matchNew) {
        return {
          drug_name: matchNew[1].trim(),
          days_remaining: parseInt(matchNew[2], 10),
          last_dispense_date: matchNew[3],
        };
      }
      // Legacy format: "drug_name (in N days)"
      const matchLegacy = entry.match(/^(.+?)\s*\(in\s+(\d+)\s+days?\)\s*$/i);
      if (matchLegacy) {
        return {
          drug_name: matchLegacy[1].trim(),
          days_remaining: parseInt(matchLegacy[2], 10),
          last_dispense_date: undefined,
        };
      }
      // Fallback: no days info found
      return {
        drug_name: entry,
        days_remaining: null,
        last_dispense_date: undefined,
      };
    });
}

// ---------------------------------------------------------------------------
// Thai alphabetical sort key
// ---------------------------------------------------------------------------

const THAI_ALPHABET = "กขฃคฅฆงจฉชซฌญฎฏฐฑฒณดตถทธนบปผฝพฟภมยรฤลฦวศษสหฬอฮ";
const THAI_ORDER: Record<string, string> = {};
THAI_ALPHABET.split("").forEach((ch, i) => {
  THAI_ORDER[ch] = String(i + 1).padStart(3, "0");
});
const THAI_VOWELS = new Set("ะาิีึืุูเแโใไๅ");

/**
 * Produce a sortable string key for Thai alphabetical ordering (ก→ฮ).
 * Skips leading vowels and diacritics/tone marks.
 * Non-Thai characters sort after Thai.
 */
export function thaiSortKey(text: string): string {
  if (!text) return "\uffff";

  // Decompose combining characters where possible using NFC
  const s = text.trim().normalize("NFC");

  // Find the first Thai consonant (skip vowels)
  let firstConsonantIdx: number | null = null;
  for (let i = 0; i < s.length; i++) {
    const ch = s[i];
    if (THAI_VOWELS.has(ch)) continue;
    if (ch in THAI_ORDER) {
      firstConsonantIdx = i;
      break;
    }
  }

  const seq = firstConsonantIdx !== null ? s.slice(firstConsonantIdx) : s;

  const parts: string[] = [];
  for (const ch of seq) {
    if (THAI_VOWELS.has(ch)) continue;
    if (ch in THAI_ORDER) {
      parts.push(THAI_ORDER[ch]);
    } else {
      // Non-Thai: put after Thai with deterministic ordering
      parts.push(`9${ch.codePointAt(0)!.toString().padStart(6, "0")}`);
    }
  }

  return parts.length > 0 ? parts.join("") : "\uffff";
}

// ---------------------------------------------------------------------------
// HN padding helper
// ---------------------------------------------------------------------------

/**
 * Zero-pad HN to 7 digits if all-numeric and shorter than 7 chars.
 * e.g. "12345" → "0012345", "AB1234" → "AB1234"
 */
export function hnPadded(hn: string): string {
  const stripped = hn.trim();
  if (/^\d+$/.test(stripped) && stripped.length < 7) {
    return stripped.padStart(7, "0");
  }
  return stripped;
}

// ---------------------------------------------------------------------------
// Drug selection helper
// ---------------------------------------------------------------------------

/**
 * Return the list of drug names where drug_selection[name] === true.
 */
export function getSelectedDrugs(record: PatientRecord): string[] {
  return Object.entries(record.drug_selection)
    .filter(([, selected]) => selected)
    .map(([name]) => name);
}
