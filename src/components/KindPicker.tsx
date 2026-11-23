import { useState } from "react";
import { Image, Video, FileText, Music, Archive, Check } from "lucide-react";

export const ALL_KINDS = ["image", "video", "document", "audio", "archive"] as const;
export type RecoverKind = (typeof ALL_KINDS)[number];

const OPTIONS: { kind: RecoverKind; label: string; hint: string; icon: typeof Image }[] = [
  { kind: "image", label: "Photos", hint: "JPG, PNG, HEIC, GIF, RAW…", icon: Image },
  { kind: "video", label: "Videos", hint: "MP4, MOV, AVI, MKV…", icon: Video },
  { kind: "document", label: "Documents & Office", hint: "DOCX, XLSX, PPTX, PDF…", icon: FileText },
  { kind: "audio", label: "Audio", hint: "MP3, WAV, AAC…", icon: Music },
  { kind: "archive", label: "Archives", hint: "ZIP, RAR, 7Z…", icon: Archive },
];

interface Props {
  open: boolean;
  deep: boolean;
  initial: RecoverKind[];
  onConfirm: (kinds: RecoverKind[]) => void;
  onCancel: () => void;
}

export default function KindPicker({ open, deep, initial, onConfirm, onCancel }: Props) {
  const [picked, setPicked] = useState<Set<RecoverKind>>(new Set(initial));

  if (!open) return null;

  const toggle = (k: RecoverKind) =>
    setPicked((prev) => {
      const next = new Set(prev);
      if (next.has(k)) next.delete(k);
      else next.add(k);
      return next;
    });

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center p-4"
      style={{ background: "rgba(0,0,0,0.6)" }}
      role="dialog"
      aria-modal="true"
      aria-labelledby="kindpicker-h"
    >
      <div
        className="w-full max-w-sm rounded-lg border p-5"
        style={{ borderColor: "var(--border)", background: "var(--surface)" }}
      >
        <p className="eyebrow mb-1">{deep ? "DEEP SCAN" : "QUICK SCAN"}</p>
        <h2 id="kindpicker-h" className="text-base font-semibold tracking-tight">
          What should we recover?
        </h2>
        <p className="mt-1 text-xs" style={{ color: "var(--muted)" }}>
          Pick the file types to look for. Everything is searched read-only.
        </p>

        <div className="mt-4 flex flex-col gap-2">
          {OPTIONS.map(({ kind, label, hint, icon: Icon }) => {
            const on = picked.has(kind);
            return (
              <button
                key={kind}
                type="button"
                onClick={() => toggle(kind)}
                aria-pressed={on}
                className="flex items-center gap-3 rounded border px-3 py-2 text-left text-sm transition-colors"
                style={{
                  borderColor: on ? "var(--accent)" : "var(--border)",
                  background: on ? "var(--surface-2)" : "transparent",
                }}
              >
                <Icon size={16} aria-hidden style={{ color: on ? "var(--accent)" : "var(--muted)" }} />
                <span className="min-w-0 flex-1">
                  <span className="block font-medium" style={{ color: "var(--text)" }}>{label}</span>
                  <span className="mono block text-[10px]" style={{ color: "var(--muted)" }}>{hint}</span>
                </span>
                {on ? <Check size={15} aria-hidden style={{ color: "var(--accent)" }} /> : null}
              </button>
            );
          })}
        </div>

        <div className="mt-4 flex items-center justify-between gap-2">
          <button
            className="btn h-9 min-h-9 px-3 text-xs"
            onClick={() => setPicked(new Set(ALL_KINDS))}
          >
            Everything
          </button>
          <div className="flex gap-2">
            <button className="btn h-9 min-h-9 px-3 text-xs" onClick={onCancel}>Cancel</button>
            <button
              className="btn h-9 min-h-9 px-3 text-xs"
              disabled={picked.size === 0}
              onClick={() => onConfirm([...picked])}
            >
              Start scan
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
