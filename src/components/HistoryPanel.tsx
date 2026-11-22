import { useEffect, useState } from "react";
import { Database } from "lucide-react";
import { scanHistory } from "../api";
import type { HistoryEntry } from "../types";

function fmtBytes(n: number): string {
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(2)} GB`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${n} B`;
}

/**
 * Per-scan history panel backed by the SQLite history store
 * (scan_history command). Hidden entirely when the engine is unavailable.
 */
export default function HistoryPanel({ refreshKey }: { refreshKey: number }) {
  const [entries, setEntries] = useState<HistoryEntry[] | null>(null);

  useEffect(() => {
    let cancelled = false;
    scanHistory()
      .then((h) => {
        if (!cancelled) setEntries(h);
      })
      .catch(() => {
        if (!cancelled) setEntries(null);
      });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  if (entries === null || entries.length === 0) return null;

  return (
    <section
      aria-label="Scan history"
      className="rounded-lg border p-5"
      style={{ borderColor: "var(--border)", background: "var(--surface-2)" }}
    >
      <p className="eyebrow mb-3 flex items-center gap-2">
        <Database size={13} aria-hidden /> SCAN HISTORY
      </p>
      <ul className="flex flex-col gap-1">
        {entries.map((e) => (
          <li
            key={e.id}
            className="mono flex flex-wrap items-baseline gap-x-3 gap-y-0.5 text-xs"
          >
            <span style={{ color: "var(--faint)" }}>
              {new Date(e.ts + "Z").toLocaleString()}
            </span>
            <span className="uppercase" style={{ color: "var(--accent)" }}>{e.kind}</span>
            <span className="min-w-0 flex-1 truncate" style={{ color: "var(--text)" }} title={e.source}>
              {e.source}
            </span>
            <span style={{ color: "var(--muted)" }}>
              {e.files} files · {fmtBytes(e.bytes)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
