import { getCurrentWindow } from "@tauri-apps/api/window";

interface Props {
  stale: boolean;
  loading: boolean;
  showHistory: boolean;
  onTranslate: () => void;
  onOpenSettings: () => void;
  onToggleHistory: () => void;
}

function HistoryIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="8" cy="8" r="6" />
      <polyline points="8,4.5 8,8 10.5,9.5" />
    </svg>
  );
}

export default function ToolbarControls({
  stale,
  loading,
  showHistory,
  onTranslate,
  onOpenSettings,
  onToggleHistory,
}: Props) {
  return (
    <div className="toolbar-controls">
      <button
        className={`icon-btn translate-btn${stale ? " stale" : ""}${loading ? " translating" : ""}`}
        aria-label="Translate"
        onClick={onTranslate}
        disabled={loading}
        title={stale ? "Content changed — click to translate" : "Translate"}
      >
        ↻
      </button>
      <button
        className={`icon-btn${showHistory ? " active" : ""}`}
        aria-label="History"
        onClick={onToggleHistory}
        title="History"
      >
        <HistoryIcon />
      </button>
      <button className="icon-btn settings-btn" aria-label="Settings" onClick={onOpenSettings}>
        ⚙
      </button>
      <button
        className="icon-btn close-btn"
        aria-label="Hide to tray"
        title="Hide to tray"
        onClick={() => getCurrentWindow().close()}
      >
        <svg width="12" height="12" viewBox="0 0 12 2" fill="currentColor">
          <rect x="0" y="0" width="12" height="2" rx="1" />
        </svg>
      </button>
    </div>
  );
}
