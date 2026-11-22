import { useEffect, useRef, useState } from "react";
import { Binary, FileText, X } from "lucide-react";
import { fileHexPreview } from "../api";
import type { FilePreview } from "../types";

function fmtSize(n: number): string {
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(2)} GB`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${n} B`;
}

/**
 * Read-only hex/preview pane for a selected file. Calls the backend's
 * file_hex_preview (first 512 bytes as 16-byte rows). When the native engine
 * is unavailable (browser preview) it says so instead of faking bytes.
 */
export default function HexPreview({
  path,
  name,
  size,
  onClose,
}: {
  path: string | null;
  name: string;
  size: number;
  onClose: () => void;
}) {
  const [preview, setPreview] = useState<FilePreview | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const requested = useRef<string | null>(null);

  useEffect(() => {
    if (!path) return;
    requested.current = path;
    setLoading(true);
    setError(null);
    setPreview(null);
    fileHexPreview(path)
      .then((p) => {
        if (requested.current === path) setPreview(p);
      })
      .catch((e) => {
        if (requested.current === path) setError(String(e));
      })
      .finally(() => {
        if (requested.current === path) setLoading(false);
      });
  }, [path]);

  if (!path) return null;

  return (
    <aside
      aria-label={`Hex preview of ${name}`}
      className="flex w-full flex-col border-t md:w-[380px] md:border-l md:border-t-0"
      style={{ borderColor: "var(--border)", background: "var(--surface-2)" }}
    >
      <div
        className="flex items-center gap-2 border-b px-4 py-2"
        style={{ borderColor: "var(--border)" }}
      >
        <Binary size={14} aria-hidden style={{ color: "var(--muted)" }} />
        <span className="eyebrow">HEX // PREVIEW</span>
        <button
          className="round-btn ml-auto h-8 w-8"
          style={{ width: 32, height: 32, minWidth: 32 }}
          onClick={onClose}
          aria-label="Close preview"
        >
          <X size={14} aria-hidden />
        </button>
      </div>

      <div className="flex items-center gap-3 px-4 py-2 text-xs" style={{ color: "var(--muted)" }}>
        <FileText size={13} aria-hidden />
        <span className="mono truncate" title={name}>{name}</span>
        <span className="mono ml-auto shrink-0">{fmtSize(size)}</span>
      </div>

      <div className="results-scroll flex-1 overflow-x-auto px-4 pb-4">
        {loading ? (
          <p className="mono py-6 text-xs" style={{ color: "var(--muted)" }}>reading bytes…</p>
        ) : error ? (
          <p className="py-6 text-xs" style={{ color: "var(--muted)" }} role="status">
            Preview needs the native engine (run the desktop app).
          </p>
        ) : preview ? (
          <>
            <table className="mono text-[11px] leading-5" style={{ borderSpacing: 0 }}>
              <tbody>
                {preview.rows.map((r) => (
                  <tr key={r.offset}>
                    <td className="pr-3 select-all" style={{ color: "var(--faint)" }}>
                      {r.offset.toString(16).padStart(8, "0")}
                    </td>
                    <td className="pr-3 whitespace-nowrap select-all" style={{ color: "var(--text)" }}>
                      {r.hex.map((b) => b.toString(16).padStart(2, "0")).join(" ")}
                    </td>
                    <td className="whitespace-nowrap" style={{ color: "var(--muted)" }}>
                      {r.ascii}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
            {preview.truncated ? (
              <p className="mono mt-2 text-[11px]" style={{ color: "var(--faint)" }}>
                first 512 bytes of {fmtSize(preview.size)}
              </p>
            ) : null}
          </>
        ) : null}
      </div>
    </aside>
  );
}
