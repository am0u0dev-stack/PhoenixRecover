import { Liquid } from "liquid-gooey";
import { ScanSearch, Microscope, Settings, X, Plus } from "lucide-react";
import { useState } from "react";

interface Props {
  onQuickScan: () => void;
  onDeepScan: () => void;
  onSettings: () => void;
  disabled?: boolean;
  light?: boolean;
}

/** Bottom-right gooey FAB built on the real `liquid-gooey` package.
 *  All action items are absolutely stacked on the main button, so closed they
 *  collapse INTO the bubble (scale 0) and open they bubble OUT of it to their
 *  fan positions — never from beside it. */
export default function LiquidFab({ onQuickScan, onDeepScan, onSettings, disabled, light }: Props) {
  const [open, setOpen] = useState(false);
  const ease = "cubic-bezier(0.34, 1.56, 0.64, 1)" as const;
  const atOrigin = { position: "absolute", top: 0, left: 0 } as const;

  const act = (fn: () => void) => {
    setOpen(false);
    fn();
  };

  return (
    <div className="fixed bottom-6 right-6 z-40" aria-label="Quick actions">
      {/* Host is exactly one button in size; absolutely-positioned items stack
          on the main button and are measured there by liquid-gooey. */}
      <div style={{ position: "relative", width: 44, height: 44 }}>
        <Liquid fill={light ? "#ececee" : "#202020"}>
          <Liquid.Item
            x={open ? -54 : 0}
            y={open ? -34 : 0}
            scale={open ? 1 : 0}
            style={atOrigin}
            transition={{ duration: 550, ease }}
            delay={open ? 0 : 80}
          >
            <button
              className="round-btn"
              title="Quick Scan (undelete mode)"
              aria-label="Quick Scan"
              disabled={disabled}
              tabIndex={open ? 0 : -1}
              onClick={() => act(onQuickScan)}
            >
              <ScanSearch size={18} aria-hidden />
            </button>
          </Liquid.Item>
          <Liquid.Item
            x={0}
            y={open ? -64 : 0}
            scale={open ? 1 : 0}
            style={atOrigin}
            transition={{ duration: 550, ease }}
            delay={open ? 40 : 40}
          >
            <button
              className="round-btn"
              title="Deep Scan (carving mode)"
              aria-label="Deep Scan"
              disabled={disabled}
              tabIndex={open ? 0 : -1}
              onClick={() => act(onDeepScan)}
            >
              <Microscope size={18} aria-hidden />
            </button>
          </Liquid.Item>
          <Liquid.Item
            x={open ? 54 : 0}
            y={open ? -34 : 0}
            scale={open ? 1 : 0}
            style={atOrigin}
            transition={{ duration: 550, ease }}
            delay={open ? 80 : 0}
          >
            <button
              className="round-btn"
              title="Settings (toggle light/dark)"
              aria-label="Settings"
              disabled={disabled}
              tabIndex={open ? 0 : -1}
              onClick={() => act(onSettings)}
            >
              <Settings size={18} aria-hidden />
            </button>
          </Liquid.Item>
          <Liquid.Item transition={{ duration: 550, ease }}>
            <button
              className="round-btn"
              title={open ? "Close quick actions" : "Open quick actions"}
              aria-label={open ? "Close quick actions" : "Open quick actions"}
              aria-expanded={open}
              onClick={() => setOpen((o) => !o)}
            >
              {open ? <X size={18} aria-hidden /> : <Plus size={18} aria-hidden />}
            </button>
          </Liquid.Item>
        </Liquid>
      </div>
    </div>
  );
}
