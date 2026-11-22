import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { RefreshCw, FolderOpen, Sun, Moon, Download, HardDrive, Undo2, ShieldCheck } from "lucide-react";
import LiquidFab from "./components/LiquidFab";
import ScanModal from "./components/ScanModal";
import SsdWarning from "./components/SsdWarning";
import SourceList from "./components/SourceList";
import ResultsTable from "./components/ResultsTable";
import HistoryPanel from "./components/HistoryPanel";
import { cancelScan, checkpointStatus, listSources, onScanProgress, recoverFiles, scanDrive } from "./api";
import type { CheckpointStatus, RecoverableFile, RecoveryReport, ScanProgress, SourceInfo } from "./types";

type BackendStatus = "checking" | "ready" | "unavailable";

export default function App() {
  const [light, setLight] = useState(false);
  const [status, setStatus] = useState<BackendStatus>("checking");
  const [sources, setSources] = useState<SourceInfo[]>([]);
  const [selectedSource, setSelectedSource] = useState<SourceInfo | null>(null);
  const [scanning, setScanning] = useState(false);
  const [scanDeep, setScanDeep] = useState(false);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [files, setFiles] = useState<RecoverableFile[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [report, setReport] = useState<RecoveryReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showSsdWarning, setShowSsdWarning] = useState(false);
  const [checkpoint, setCheckpoint] = useState<CheckpointStatus | null>(null);
  const [historyKey, setHistoryKey] = useState(0);
  const pendingScan = useRef<{ path: string; deep: boolean } | null>(null);
  const resultsRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    document.documentElement.classList.toggle("light", light);
  }, [light]);

  const refreshSources = useCallback(async () => {
    try {
      const list = await listSources();
      setSources(list);
      setStatus("ready");
    } catch {
      setStatus("unavailable");
      setError(null);
    }
  }, []);

  useEffect(() => {
    refreshSources();
  }, [refreshSources]);

  useEffect(() => {
    let unbind: (() => void) | undefined;
    let cancelled = false;
    onScanProgress(setProgress)
      .then((u) => {
        if (cancelled) u();
        else unbind = u;
      })
      .catch(() => undefined); // engine unavailable (browser preview)
    return () => {
      cancelled = true;
      unbind?.();
    };
  }, []);

  const runScan = useCallback(async (path: string, deep: boolean, resume = false) => {
    setScanning(true);
    setScanDeep(deep);
    setProgress(null);
    setError(null);
    setReport(null);
    try {
      const found = await scanDrive(path, deep, resume);
      setFiles(found);
      setSelectedIds(new Set());
      setHistoryKey((k) => k + 1);
    } catch (e) {
      setError(String(e));
      setFiles([]);
    } finally {
      setScanning(false);
    }
  }, []);

  // Surface any resumable deep-scan checkpoint for the selected source.
  useEffect(() => {
    if (!selectedSource || status !== "ready") {
      setCheckpoint(null);
      return;
    }
    let cancelled = false;
    checkpointStatus(selectedSource.path)
      .then((c) => {
        if (!cancelled) setCheckpoint(c);
      })
      .catch(() => undefined); // engine unavailable (browser preview)
    return () => {
      cancelled = true;
    };
  }, [selectedSource, status]);

  const selectSource = useCallback((s: SourceInfo) => {
    setSelectedSource(s);
    setFiles([]);
    setSelectedIds(new Set());
    setReport(null);
    setError(null);
    setShowSsdWarning(s.device_type === "ssd");
    setCheckpoint(null);
  }, []);

  // Flow from the FAB: pick a folder if nothing is selected yet, then scan it.
  const scanViaFab = useCallback(
    async (deep: boolean) => {
      if (selectedSource) {
        await runScan(selectedSource.path, deep);
        resultsRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
        return;
      }
      pendingScan.current = { path: "", deep };
      try {
        const dir = await import("@tauri-apps/plugin-dialog").then((m) => m.open({ directory: true }));
        if (typeof dir === "string") {
          const s: SourceInfo = {
            name: dir.split(/[\\/]/).pop() || dir,
            path: dir,
            kind: "folder",
            size_bytes: 0,
            device_type: "unknown",
            read_only: true,
          };
          setSelectedSource(s);
          await runScan(dir, deep);
        }
      } catch (e) {
        // In the browser preview the dialog/engine is unavailable; say so
        // inline instead of surfacing a raw TypeError.
        const msg = String(e);
        setError(
          msg.includes("invoke") || msg.includes("dialog")
            ? "The native dialog is not reachable from this preview. Run the desktop app to pick a folder and scan."
            : msg,
        );
      }
    },
    [selectedSource, runScan],
  );

  const toggle = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const toggleAll = useCallback((checked: boolean) => {
    setSelectedIds(checked ? new Set(files.map((f) => f.id)) : new Set());
  }, [files]);

  const recover = useCallback(async () => {
    if (!selectedSource || selectedIds.size === 0) return;
    try {
      const dir = await import("@tauri-apps/plugin-dialog").then((m) => m.open({ directory: true, title: "Choose recovery output folder" }));
      if (typeof dir !== "string") return;
      const r = await recoverFiles([...selectedIds], dir);
      setReport(r);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [selectedSource, selectedIds]);

  const selectedBytes = useMemo(
    () => files.filter((f) => selectedIds.has(f.id)).reduce((a, f) => a + f.size, 0),
    [files, selectedIds],
  );

  const fmtBytes = (n: number) =>
    n >= 1024 ** 2 ? `${(n / 1024 ** 2).toFixed(1)} MB` : n >= 1024 ? `${(n / 1024).toFixed(0)} KB` : `${n} B`;

  return (
    <div className="grain flex h-full flex-col" style={{ background: "var(--bg)" }}>
      {/* ── Header ─────────────────────────────────────────────── */}
      <header
        className="relative z-10 flex items-center gap-4 border-b px-6 py-4"
        style={{ borderColor: "var(--border)", background: "var(--surface)" }}
      >
        <div className="flex h-9 w-9 items-center justify-center rounded-full border" style={{ borderColor: "var(--muted)" }} aria-hidden>
          <ShieldCheck size={18} style={{ color: "var(--accent)" }} />
        </div>
        <div className="min-w-0">
          <h1 className="truncate text-base font-semibold tracking-tight">PhoenixRecover</h1>
          <p className="eyebrow">READ-ONLY SCAN ENGINE</p>
        </div>
        <div className="mono ml-auto flex items-center gap-4 text-xs" style={{ color: "var(--muted)" }}>
          <span className="hidden md:inline" data-testid="backend-status">
            {status === "ready" ? "ENGINE: ONLINE" : status === "checking" ? "ENGINE: …" : "ENGINE: OFFLINE (RUN IN TAURI)"}
          </span>
          <button
            className="round-btn h-9 w-9"
            onClick={() => setLight(!light)}
            aria-label={light ? "Switch to dark theme" : "Switch to light theme"}
          >
            {light ? <Moon size={15} aria-hidden /> : <Sun size={15} aria-hidden />}
          </button>
        </div>
      </header>

      <main className="relative z-10 flex-1 overflow-y-auto">
        <div className="mx-auto flex h-full max-w-4xl flex-col gap-6 px-6 py-8">
          {/* ── Source selection ───────────────────────────────── */}
          <section aria-labelledby="sources-h">
            <div className="mb-4 flex items-end justify-between gap-4">
              <div>
                <h2 id="sources-h" className="text-lg font-semibold tracking-tight">
                  Recover what the filesystem forgot.
                </h2>
                <p className="mt-1 max-w-prose text-sm" style={{ color: "var(--muted)" }}>
                  Undelete via NTFS, FAT, exFAT, ext4 and HFS+ metadata, or carve byte-level
                  signatures across 150+ formats with unearth and forsmost. Every source
                  operation is strictly read-only.
                </p>
              </div>
              <button className="btn shrink-0" onClick={refreshSources} aria-label="Rescan connected sources">
                <RefreshCw size={15} aria-hidden /> Refresh
              </button>
            </div>

            {status === "unavailable" ? (
              <div
                className="rounded border p-4 text-sm"
                style={{ borderColor: "var(--border)", background: "var(--surface)" }}
              >
                <p style={{ color: "var(--muted)" }}>
                  The native engine is not reachable from this preview. Run the desktop app to
                  scan real drives; browser preview is UI-only.
                </p>
              </div>
            ) : (
              <SourceList sources={sources} selected={selectedSource?.path ?? null} onSelect={selectSource} />
            )}

            <div className="mt-4 flex flex-wrap items-center gap-3">
              <button
                className="btn"
                onClick={async () => {
                  try {
                    const dir = await import("@tauri-apps/plugin-dialog").then((m) => m.open({ directory: true }));
                    if (typeof dir === "string") {
                      selectSource({
                        name: dir.split(/[\\/]/).pop() || dir,
                        path: dir,
                        kind: "folder",
                        size_bytes: 0,
                        device_type: "unknown",
                        read_only: true,
                      });
                    }
                  } catch (e) {
                    setError(String(e));
                  }
                }}
              >
                <FolderOpen size={16} aria-hidden /> Select Drive / Folder
              </button>
              {selectedSource ? (
                <>
                  <button
                    className="btn"
                    onClick={() => runScan(selectedSource.path, false)}
                    disabled={scanning}
                  >
                    <HardDrive size={16} aria-hidden /> Quick Scan
                  </button>
                  <button
                    className="btn"
                    onClick={() => runScan(selectedSource.path, true)}
                    disabled={scanning}
                  >
                    Deep Scan
                  </button>
                </>
              ) : null}
            </div>

            {checkpoint?.exists ? (
              <div
                role="status"
                className="mt-3 flex flex-wrap items-center gap-3 rounded border px-4 py-3 text-sm"
                style={{ borderColor: "var(--muted)", background: "var(--surface-2)" }}
              >
                <Undo2 size={15} aria-hidden style={{ color: "var(--accent)" }} />
                <span className="min-w-0 flex-1" style={{ color: "var(--muted)" }}>
                  An interrupted deep scan of this source left a checkpoint. Resume instead of
                  starting over.
                </span>
                <button
                  className="btn h-9 min-h-9 px-3 text-xs"
                  onClick={() => selectedSource && runScan(selectedSource.path, true, true)}
                  disabled={scanning}
                >
                  <Undo2 size={14} aria-hidden /> Resume deep scan
                </button>
              </div>
            ) : null}
          </section>

          {showSsdWarning && selectedSource?.device_type === "ssd" ? (
            <SsdWarning onDismiss={() => setShowSsdWarning(false)} />
          ) : null}

          {error ? (
            <div
              role="alert"
              className="flex items-center gap-3 rounded border p-4 text-sm"
              style={{ borderColor: "var(--muted)", background: "var(--surface-2)" }}
            >
              <span className="min-w-0 flex-1 break-words" style={{ color: "var(--text)" }}>{error}</span>
              <button className="btn h-9 min-h-9 px-3 text-xs" onClick={() => setError(null)}>Dismiss</button>
            </div>
          ) : null}

          {/* ── Results ────────────────────────────────────────── */}
          <section ref={resultsRef} aria-labelledby="results-h" className="flex min-h-[320px] flex-1 flex-col">
            <div className="mb-3 flex items-center justify-between">
              <h2 id="results-h" className="eyebrow">
                {selectedSource ? `RESULTS // ${selectedSource.name}` : "RESULTS"}
              </h2>
              {files.length > 0 ? (
                <button className="btn h-10 min-h-10" onClick={recover} disabled={selectedIds.size === 0}>
                  <Download size={15} aria-hidden />
                  Recover Selected ({selectedIds.size} · {fmtBytes(selectedBytes)})
                </button>
              ) : null}
            </div>

            <div
              className="min-h-[280px] flex-1 overflow-hidden rounded-lg border"
              style={{ borderColor: "var(--border)", background: "var(--surface)" }}
            >
              <ResultsTable files={files} selectedIds={selectedIds} onToggle={toggle} onToggleAll={toggleAll} />
            </div>
          </section>

          {report ? (
            <section
              aria-label="Recovery report"
              className="rounded-lg border p-5"
              style={{ borderColor: "var(--border)", background: "var(--surface-2)" }}
            >
              <p className="eyebrow mb-3">RECOVERY REPORT</p>
              <div className="mono grid grid-cols-2 gap-x-8 gap-y-1 text-sm sm:grid-cols-4">
                <span style={{ color: "var(--muted)" }}>recovered</span>
                <span style={{ color: "var(--accent)" }}>{report.recovered}</span>
                <span style={{ color: "var(--muted)" }}>failed</span>
                <span>{report.failed}</span>
                <span style={{ color: "var(--muted)" }}>bytes</span>
                <span>{fmtBytes(report.bytes_recovered)}</span>
                <span style={{ color: "var(--muted)" }}>output</span>
                <span className="truncate">{report.output_dir}</span>
              </div>
              <p className="mono mt-3 truncate text-xs" style={{ color: "var(--faint)" }}>
                {Object.entries(report.by_type)
                  .map(([k, v]) => `${k}×${v}`)
                  .join("  ")}
              </p>
              {report.notes.map((n) => (
                <p key={n} className="mt-1 text-xs" style={{ color: "var(--muted)" }}>{n}</p>
              ))}
            </section>
          ) : null}

          <HistoryPanel refreshKey={historyKey} />
        </div>
      </main>

      <LiquidFab
        onQuickScan={() => scanViaFab(false)}
        onDeepScan={() => scanViaFab(true)}
        onSettings={() => setLight(!light)}
        light={light}
        disabled={scanning}
      />

      <ScanModal
        open={scanning}
        deep={scanDeep}
        progress={progress}
        onCancel={() => {
          cancelScan().catch(() => undefined);
        }}
      />
    </div>
  );
}
