import { X, ShieldCheck, Microscope, Sun, Moon, HardDrive } from "lucide-react";

interface Props {
  open: boolean;
  light: boolean;
  onToggleTheme: () => void;
  onClose: () => void;
}

function Row({ icon, title, body }: { icon: React.ReactNode; title: string; body: string }) {
  return (
    <div className="flex gap-3 py-3">
      <div className="mt-0.5 shrink-0" style={{ color: "var(--accent)" }}>{icon}</div>
      <div className="min-w-0">
        <p className="text-sm font-medium">{title}</p>
        <p className="mono mt-1 text-xs leading-relaxed" style={{ color: "var(--muted)" }}>{body}</p>
      </div>
    </div>
  );
}

/** Settings & diagnostics — what recovery apps show before you trust them
 *  with a drive: which engine runs, what it touches, how output is written. */
export default function SettingsModal({ open, light, onToggleTheme, onClose }: Props) {
  if (!open) return null;
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      style={{ background: "rgb(0 0 0 / 0.6)" }}
      role="dialog"
      aria-modal="true"
      aria-label="Settings"
    >
      <div
        className="grain relative w-[460px] max-w-[calc(100vw-32px)] rounded-lg border p-6"
        style={{ background: "var(--surface)", borderColor: "var(--border)", boxShadow: "var(--shadow-2)" }}
      >
        <div className="mb-4 flex items-center justify-between">
          <p className="eyebrow">SETTINGS // ENGINE &amp; SAFETY</p>
          <button className="round-btn h-9 w-9" onClick={onClose} aria-label="Close settings">
            <X size={15} aria-hidden />
          </button>
        </div>

        <div className="divide-y" style={{ borderColor: "var(--border)" }}>
          <Row
            icon={<Microscope size={16} aria-hidden />}
            title="Deep-scan engine: PhotoRec 7.2 (CGSecurity)"
            body="Signature carving over 480+ file formats — recovers files from formatted and repartitioned media, not just deleted ones. Quick scan still uses metadata undelete."
          />
          <Row
            icon={<ShieldCheck size={16} aria-hidden />}
            title="Read-only scan"
            body="Sources are opened read-only; nothing is ever written to the drive being scanned. Recovered files go to an output folder you choose."
          />
          <Row
            icon={<HardDrive size={16} aria-hidden />}
            title="Drive roots need administrator rights"
            body="Scanning a whole drive (E:\\) reads its raw volume. If access is denied, right-click PhoenixRecover and choose Run as administrator."
          />
        </div>

        <div className="mt-4 flex items-center justify-between rounded border p-3" style={{ borderColor: "var(--border)", background: "var(--surface-2)" }}>
          <span className="text-sm">Appearance</span>
          <button className="btn h-9 min-h-9 px-3 text-xs" onClick={onToggleTheme}>
            {light ? <Moon size={14} aria-hidden /> : <Sun size={14} aria-hidden />}
            {light ? " Dark theme" : " Light theme"}
          </button>
        </div>

        <button className="btn mt-5 w-full" onClick={onClose}>
          Done
        </button>
      </div>
    </div>
  );
}
