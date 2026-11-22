import { useEffect, useMemo, useRef, useState } from "react";
import { ArrowDown, ArrowUp, Search, SearchX } from "lucide-react";
import type { RecoverableFile } from "../types";
import HexPreview from "./HexPreview";

const ROW_H = 40;
const OVERSCAN = 8;

function fmtSize(n: number): string {
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(2)} GB`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(1)} MB`;
  if (n >= 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${n} B`;
}

type SortKey = "name" | "size" | "offset";
type SortDir = "asc" | "desc";

interface Props {
  files: RecoverableFile[];
  selectedIds: Set<string>;
  onToggle: (id: string) => void;
  onToggleAll: (checked: boolean) => void;
}

/** Sortable column header button. */
function SortHeader({
  label,
  k,
  sortKey,
  sortDir,
  onSort,
  right,
}: {
  label: string;
  k: SortKey;
  sortKey: SortKey;
  sortDir: SortDir;
  onSort: (k: SortKey) => void;
  right?: boolean;
}) {
  const active = sortKey === k;
  return (
    <button
      className={`flex items-center gap-1 px-2 py-2 font-normal ${right ? "ml-auto" : ""}`}
      style={{ color: active ? "var(--text)" : "var(--muted)" }}
      onClick={() => onSort(k)}
      aria-sort={active ? (sortDir === "asc" ? "ascending" : "descending") : "none"}
    >
      {label}
      {active ? (
        sortDir === "asc" ? <ArrowUp size={11} aria-hidden /> : <ArrowDown size={11} aria-hidden />
      ) : null}
    </button>
  );
}

export default function ResultsTable({ files, selectedIds, onToggle, onToggleAll }: Props) {
  const [query, setQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState("all");
  const [modeFilter, setModeFilter] = useState<"all" | "undelete" | "carve">("all");
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [sortDir, setSortDir] = useState<SortDir>("asc");
  const [preview, setPreview] = useState<RecoverableFile | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewportH, setViewportH] = useState(480);

  const types = useMemo(() => {
    const set = new Set(files.map((f) => f.ext).filter(Boolean));
    return [...set].sort();
  }, [files]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    let list = files.filter((f) => {
      if (q && !f.name.toLowerCase().includes(q)) return false;
      if (typeFilter !== "all" && f.ext !== typeFilter) return false;
      if (modeFilter !== "all" && f.strategy !== modeFilter) return false;
      return true;
    });
    const dir = sortDir === "asc" ? 1 : -1;
    list = [...list].sort((a, b) => {
      switch (sortKey) {
        case "size":
          return (a.size - b.size) * dir;
        case "offset":
          return ((a.offset ?? -1) - (b.offset ?? -1)) * dir;
        default:
          return a.name.localeCompare(b.name) * dir;
      }
    });
    return list;
  }, [files, query, typeFilter, modeFilter, sortKey, sortDir]);

  // Virtualized window over the (possibly 100k+) filtered rows.
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const onScroll = () => setScrollTop(el.scrollTop);
    el.addEventListener("scroll", onScroll, { passive: true });
    const ro = new ResizeObserver(() => setViewportH(el.clientHeight));
    ro.observe(el);
    return () => {
      el.removeEventListener("scroll", onScroll);
      ro.disconnect();
    };
  }, [filtered.length]);

  const start = Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN);
  const end = Math.min(filtered.length, Math.ceil((scrollTop + viewportH) / ROW_H) + OVERSCAN);
  const window_ = filtered.slice(start, end);
  const allChecked = filtered.length > 0 && filtered.every((f) => selectedIds.has(f.id));

  const onSort = (k: SortKey) => {
    if (k === sortKey) setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    else {
      setSortKey(k);
      setSortDir("asc");
    }
  };

  if (files.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 py-16 text-center">
        <SearchX size={28} aria-hidden style={{ color: "var(--faint)" }} />
        <p className="text-sm" style={{ color: "var(--muted)" }}>
          No recoverable files found on this source.
        </p>
        <p className="mono text-xs" style={{ color: "var(--faint)" }}>
          Try a Deep Scan — carving can find files the metadata pass misses.
        </p>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col md:flex-row">
      <div className="flex min-w-0 flex-1 flex-col">
        {/* Filter / sort toolbar */}
        <div
          className="flex flex-wrap items-center gap-3 border-b px-4 py-2"
          style={{ borderColor: "var(--border)", background: "var(--surface)" }}
        >
          <label className="flex items-center gap-2 text-xs" style={{ color: "var(--muted)" }}>
            <input
              type="checkbox"
              checked={allChecked}
              onChange={(e) => onToggleAll(e.target.checked)}
              aria-label="Select all files"
            />
            select all
          </label>
          <label className="relative">
            <span className="sr-only">Filter by name</span>
            <Search
              size={13}
              aria-hidden
              className="absolute left-2 top-1/2 -translate-y-1/2"
              style={{ color: "var(--faint)" }}
            />
            <input
              type="search"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Filter by name…"
              aria-label="Filter files by name"
              className="mono h-8 w-44 rounded border bg-transparent pl-7 pr-2 text-xs"
              style={{ borderColor: "var(--border)", color: "var(--text)" }}
            />
          </label>
          <label className="flex items-center gap-1 text-xs" style={{ color: "var(--muted)" }}>
            type
            <select
              value={typeFilter}
              onChange={(e) => setTypeFilter(e.target.value)}
              aria-label="Filter by file type"
              className="mono h-8 rounded border bg-transparent px-1 text-xs"
              style={{ borderColor: "var(--border)", color: "var(--text)" }}
            >
              <option value="all">all</option>
              {types.map((t) => (
                <option key={t} value={t}>{t}</option>
              ))}
            </select>
          </label>
          <label className="flex items-center gap-1 text-xs" style={{ color: "var(--muted)" }}>
            mode
            <select
              value={modeFilter}
              onChange={(e) => setModeFilter(e.target.value as typeof modeFilter)}
              aria-label="Filter by recovery mode"
              className="mono h-8 rounded border bg-transparent px-1 text-xs"
              style={{ borderColor: "var(--border)", color: "var(--text)" }}
            >
              <option value="all">all</option>
              <option value="undelete">undelete</option>
              <option value="carve">carve</option>
            </select>
          </label>
          <span className="mono ml-auto text-xs" style={{ color: "var(--muted)" }}>
            {filtered.length === files.length
              ? `${files.length} files`
              : `${filtered.length} of ${files.length}`} · {selectedIds.size} selected
          </span>
        </div>

        {filtered.length === 0 ? (
          <div className="flex flex-1 flex-col items-center gap-2 py-12 text-center">
            <SearchX size={22} aria-hidden style={{ color: "var(--faint)" }} />
            <p className="text-sm" style={{ color: "var(--muted)" }}>
              No files match the current filters.
            </p>
            <button
              className="btn h-8 min-h-8 px-3 text-xs"
              onClick={() => {
                setQuery("");
                setTypeFilter("all");
                setModeFilter("all");
              }}
            >
              Clear filters
            </button>
          </div>
        ) : (
          <div className="results-scroll flex-1" ref={scrollRef}>
            <div style={{ height: filtered.length * ROW_H, position: "relative" }}>
              <div style={{ position: "absolute", top: start * ROW_H, left: 0, right: 0 }}>
                {window_.map((f) => (
                  <div
                    key={f.id}
                    className="scan-row flex items-center border-t"
                    style={{
                      height: ROW_H,
                      borderColor: "var(--border)",
                      background: selectedIds.has(f.id) ? "var(--surface-2)" : "transparent",
                    }}
                  >
                    <div className="px-4">
                      <input
                        type="checkbox"
                        checked={selectedIds.has(f.id)}
                        onChange={() => onToggle(f.id)}
                        aria-label={`Select ${f.name}`}
                      />
                    </div>
                    <button
                      className="min-w-0 flex-1 truncate text-left text-sm hover:underline"
                      style={{ color: "var(--text)" }}
                      onClick={() => setPreview(preview?.id === f.id ? null : f)}
                      aria-expanded={preview?.id === f.id}
                      aria-label={`Preview ${f.name}`}
                      title={f.name}
                    >
                      {f.name}
                    </button>
                    <span className="mono w-14 px-2 text-right uppercase" style={{ color: "var(--muted)" }}>
                      {f.ext}
                    </span>
                    <span className="mono w-20 px-2 text-right" style={{ color: "var(--text)" }}>
                      {fmtSize(f.size)}
                    </span>
                    <span className="mono w-20 px-2" style={{ color: "var(--muted)" }}>
                      {f.strategy === "undelete" ? "undelete" : "carve"}
                    </span>
                    <span className="mono w-24 px-4 text-right" style={{ color: "var(--faint)" }}>
                      {f.offset === null ? "—" : `0x${f.offset.toString(16)}`}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Header row rendered after content so it sits directly above the list */}
        <div
          className="eyebrow flex items-center border-t"
          style={{ borderColor: "var(--border)", background: "var(--surface)" }}
        >
          <div className="w-12" />
          <SortHeader label="Name" k="name" sortKey={sortKey} sortDir={sortDir} onSort={onSort} />
          <span className="w-14 px-2 text-right">Type</span>
          <SortHeader label="Size" k="size" sortKey={sortKey} sortDir={sortDir} onSort={onSort} right />
          <span className="w-20 px-2">Mode</span>
          <SortHeader label="Offset" k="offset" sortKey={sortKey} sortDir={sortDir} onSort={onSort} right />
        </div>
      </div>

      {preview ? (
        <HexPreview
          path={preview.staging_path ?? preview.name}
          name={preview.name}
          size={preview.size}
          onClose={() => setPreview(null)}
        />
      ) : null}
    </div>
  );
}
