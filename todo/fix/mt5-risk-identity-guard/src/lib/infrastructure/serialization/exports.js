/**
 * Pure serializers for user-exportable research and audit artifacts.
 * Keep these independent of browser and Tauri APIs so they can be tested with Node.
 * @typedef {import('../../types').AuditEvent} AuditEvent
 * @typedef {import('../../types').BacktestResult} BacktestResult
 */

const AUDIT_HEADERS = [
  'timestamp_utc',
  'event_id',
  'level',
  'category',
  'title',
  'detail',
];

/**
 * Quote every cell and neutralize leading spreadsheet formula markers.
 * Prefixing a literal apostrophe is deliberate: CSV quoting alone does not stop
 * spreadsheet applications from interpreting untrusted text as a formula.
 * @param {unknown} value
 */
function csvCell(value) {
  const raw = value == null ? '' : String(value);
  const safe = /^[\u0000-\u0020]*[=+\-@]/.test(raw) ? `'${raw}` : raw;
  return `"${safe.replace(/"/g, '""')}"`;
}

/** @param {unknown} timestampMs */
function timestampUtc(timestampMs) {
  const numericTimestamp = typeof timestampMs === 'number' ? timestampMs : Number.NaN;
  const date = new Date(numericTimestamp);
  return Number.isFinite(numericTimestamp) && Number.isFinite(date.getTime())
    ? date.toISOString()
    : '';
}

/**
 * Serialize audit rows in their displayed order (newest first).
 * Exports are intentionally limited to the events supplied by the caller.
 * @param {AuditEvent[]} events
 */
export function auditEventsToCsv(events) {
  if (!Array.isArray(events)) throw new TypeError('Audit events must be an array.');
  const rows = events.map((event) => [
    timestampUtc(event.timestampMs),
    event.id,
    event.level,
    event.category,
    event.title,
    event.detail,
  ]);
  // UTF-8 BOM improves direct opening in spreadsheet programs; CRLF is CSV-friendly.
  return `\uFEFF${[AUDIT_HEADERS, ...rows].map((row) => row.map(csvCell).join(',')).join('\r\n')}\r\n`;
}

/**
 * Wrap a completed backtest output in export schema v2. The result includes a
 * SHA-256 fingerprint for canonical input candles, but does not embed raw candles or claim source authenticity.
 * @param {BacktestResult} result
 * @param {string} [exportedAtUtc]
 */
export function backtestReportToJson(result, exportedAtUtc = new Date().toISOString()) {
  if (!result || typeof result !== 'object' || typeof result.id !== 'string') {
    throw new TypeError('A completed backtest result is required.');
  }
  return `${JSON.stringify({
    format: 'agentic-trading-os.backtest-report',
    formatVersion: 2,
    exportedAtUtc,
    result,
  }, null, 2)}\n`;
}
