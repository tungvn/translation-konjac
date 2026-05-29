import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window"; // used for hide()

interface Props {
  stale: boolean;
  loading: boolean;
  showHistory: boolean;
  updateVersion: string | null;
  updating: boolean;
  onTranslate: () => void;
  onOpenSettings: () => void;
  onToggleHistory: () => void;
  onUpdate: () => void;
}

function HistoryIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="8" cy="8" r="6" />
      <polyline points="8,4.5 8,8 10.5,9.5" />
    </svg>
  );
}

export default function ToolbarControls({
  stale,
  loading,
  showHistory,
  updateVersion,
  updating,
  onTranslate,
  onOpenSettings,
  onToggleHistory,
  onUpdate,
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
      {updateVersion && (
        <button
          className={`icon-btn update-btn${updating ? " updating" : ""}`}
          aria-label={`Update to ${updateVersion}`}
          title={updating ? "Installing update…" : `Update available: ${updateVersion}`}
          onClick={onUpdate}
          disabled={updating}
        >
          {updating ? <UpdateSpinner /> : <UpdateIcon />}
        </button>
      )}
      <button className="icon-btn settings-btn" aria-label="Settings" onClick={onOpenSettings}>
        ⚙
      </button>
      <button
        className="icon-btn close-btn"
        aria-label="Hide to tray"
        title="Hide to tray"
        onClick={() => { getCurrentWindow().hide(); invoke("show_tray"); }}
      >
        <svg width="12" height="12" viewBox="0 0 12 2" fill="currentColor">
          <rect x="0" y="0" width="12" height="2" rx="1" />
        </svg>
      </button>
    </div>
  );
}

function UpdateIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <line x1="8" y1="12" x2="8" y2="3" />
      <polyline points="4,7 8,3 12,7" />
      <line x1="3" y1="13" x2="13" y2="13" />
    </svg>
  );
}

function UpdateSpinner() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" className="update-spin">
      <circle cx="8" cy="8" r="6" strokeDasharray="28" strokeDashoffset="8" />
    </svg>
  );
}
