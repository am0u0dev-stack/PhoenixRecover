import { ThinkingOrb } from "thinking-orbs";
import { X } from "lucide-react";
import type { ScanProgress } from "../types";

interface Props {
  open: boolean;
  deep: boolean;
  progress: ScanProgress | null;
  onCancel: () => void;
}

function fmtBytes(n: number): string {
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(2)} GB`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${n} B`;
}

const PHASE_LABEL: Record<ScanProgress["phase"], string> = {
  metadata: "UNDELETE // METADATA PASS",
  carve: "DEEP SCAN // SIGNATURE CARVE",
  office: "DEEP SCAN // OFFICE FORMATS (FORSMOST)",
  done: "SCAN COMPLETE",
};

/** Monochrome scan progress modal built around the real `thinking-orbs` package. */
export default function ScanModal({ open, deep, progress, onCancel }: Props) {
  if (!open) return null;
  const pct = progress ? Math.min(100, Math.round(progress.percent)) : 0;
  const indeterminate = !progress || progress.percent < 0;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: "rgb(0 0 0 / 0.6)" }}
      role="dialog"
      aria-modal="true"
      aria-label="Scan in progress"
    >
      <div
        className="grain relative w-[420px] max-w-[calc(100vw-32px)] rounded-lg border p-8"
        style={{ background: "var(--surface)", borderColor: "var(--border)", boxShadow: "var(--shadow-2)" }}
      >
        <p className="eyebrow mb-6 text-center">
          {progress ? PHASE_LABEL[progress.phase] : deep ? "DEEP SCAN // PREPARING" : "QUICK SCAN // PREPARING"}
        </p>

        <div className="flex justify-center">
          <ThinkingOrb state="working" size={64} color="#e8e8ea" />
        </div>

        <p className="mono mt-6 text-center text-3xl" style={{ color: "var(--accent)" }}>
          {indeterminate ? "···" : `${pct}%`}
        </p>

        <div
          className="mt-4 h-1 w-full overflow-hidden rounded-full"
          style={{ background: "var(--border)" }}
          role="progressbar"
          aria-valuenow={indeterminate ? undefined : pct}
          aria-valuemin={0}
          aria-valuemax={100}
        >
          <div
            className="h-full rounded-full transition-[width] duration-300 ease-out"
            style={{
              width: indeterminate ? "40%" : `${pct}%`,
              background: "var(--accent)",
            }}
          />
        </div>

        <p className="mono mt-4 text-center text-xs" style={{ color: "var(--muted)" }}>
          {progress
            ? `${fmtBytes(progress.bytes_scanned)} scanned${progress.total_bytes > 0 ? ` of ${fmtBytes(progress.total_bytes)}` : ""}`
            : "opening source read-only…"}
        </p>
        {progress?.note ? (
          <p className="mono mt-2 truncate text-center text-xs" style={{ color: "var(--faint)" }}>
            {progress.note}
          </p>
        ) : null}

        <button className="btn mt-8 w-full" onClick={onCancel}>
          <X size={16} aria-hidden /> Cancel scan
        </button>
      </div>
    </div>
  );
}
