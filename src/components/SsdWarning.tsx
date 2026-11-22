import { AlertTriangle } from "lucide-react";

/** Prominent monochrome warning shown when the selected source is an SSD. */
export default function SsdWarning({ onDismiss }: { onDismiss: () => void }) {
  return (
    <div
      role="alert"
      className="grain relative rounded-md border p-4"
      style={{ borderColor: "var(--muted)", background: "var(--surface-2)", boxShadow: "var(--shadow-1)" }}
    >
      <div className="flex items-start gap-3">
        <AlertTriangle size={20} aria-hidden style={{ color: "var(--accent)", flexShrink: 0, marginTop: 2 }} />
        <div className="min-w-0">
          <p className="eyebrow" style={{ color: "var(--accent)" }}>
            SSD SELECTED — TRIM WARNING
          </p>
          <p className="mt-2 text-sm" style={{ color: "var(--text)" }}>
            This is an SSD. <strong>TRIM</strong> erases deleted blocks within seconds of deletion,
            which makes recovery nearly impossible. If the data is critical:
          </p>
          <ol className="mt-2 list-decimal space-y-1 pl-5 text-sm" style={{ color: "var(--muted)" }}>
            <li>Power the device off <em>immediately</em> — every powered-on second lowers the odds.</li>
            <li>Boot a Live USB environment and mount the device <span className="mono">read-only</span>.</li>
            <li>Scan the device or a sector image of it from that environment.</li>
          </ol>
        </div>
        <button
          className="btn ml-auto h-9 min-h-9 px-3 text-xs"
          onClick={onDismiss}
          aria-label="Dismiss SSD warning"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
