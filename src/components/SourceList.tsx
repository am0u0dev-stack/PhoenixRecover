import { HardDrive, FolderOpen, Disc, FileArchive, Cpu, Lock } from "lucide-react";
import type { SourceInfo } from "../types";

const KIND_LABEL: Record<SourceInfo["kind"], string> = {
  drive: "BLOCK DEVICE",
  folder: "FOLDER",
  image: "DISK IMAGE",
};

const DEV_LABEL: Record<SourceInfo["device_type"], string> = {
  ssd: "SSD",
  hdd: "HDD",
  usb: "USB FLASH",
  sd: "SD CARD",
  unknown: "",
};

function icon(src: SourceInfo) {
  if (src.kind === "folder") return <FolderOpen size={18} aria-hidden />;
  if (src.kind === "image") return <FileArchive size={18} aria-hidden />;
  if (src.device_type === "ssd") return <Cpu size={18} aria-hidden />;
  return <HardDrive size={18} aria-hidden />;
}

function fmtSize(n: number): string {
  if (n <= 0) return "—";
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(1)} GB`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(0)} MB`;
  return `${(n / 1024).toFixed(0)} KB`;
}

export default function SourceList({
  sources,
  selected,
  onSelect,
}: {
  sources: SourceInfo[];
  selected: string | null;
  onSelect: (s: SourceInfo) => void;
}) {
  if (sources.length === 0) {
    return (
      <p className="mono py-8 text-center text-sm" style={{ color: "var(--muted)" }}>
        No sources found. Connect a drive, or pick a folder below.
      </p>
    );
  }
  return (
    <ul className="grid gap-2" role="listbox" aria-label="Recovery sources">
      {sources.map((s) => {
        const active = s.path === selected;
        return (
          <li key={s.path}>
            <button
              role="option"
              aria-selected={active}
              onClick={() => onSelect(s)}
              className="flex w-full items-center gap-4 rounded border p-4 text-left transition-colors"
              style={{
                borderColor: active ? "var(--accent)" : "var(--border)",
                background: active ? "var(--surface-2)" : "var(--surface)",
              }}
            >
              <span style={{ color: active ? "var(--accent)" : "var(--muted)" }}>{icon(s)}</span>
              <span className="min-w-0 flex-1">
                <span className="block truncate text-sm font-medium">{s.name}</span>
                <span className="mono block truncate text-xs" style={{ color: "var(--muted)" }}>
                  {s.path}
                </span>
              </span>
              <span className="mono flex shrink-0 items-center gap-3 text-xs" style={{ color: "var(--muted)" }}>
                {s.device_type === "ssd" ? (
                  <span
                    className="rounded border px-1.5 py-0.5"
                    style={{ borderColor: "var(--muted)", color: "var(--accent)" }}
                  >
                    SSD
                  </span>
                ) : s.device_type !== "unknown" ? (
                  <span
                    className="rounded border px-1.5 py-0.5"
                    style={{ borderColor: "var(--border)" }}
                  >
                    {DEV_LABEL[s.device_type]}
                  </span>
                ) : null}
                <span>{KIND_LABEL[s.kind]}</span>
                <span>{fmtSize(s.size_bytes)}</span>
                {s.read_only ? (
                  <span className="flex items-center gap-1" title="Opened read-only">
                    <Lock size={12} aria-hidden />
                  </span>
                ) : null}
              </span>
            </button>
          </li>
        );
      })}
    </ul>
  );
}

export { Disc };
