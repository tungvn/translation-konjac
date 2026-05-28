interface Props {
  stale: boolean;
  loading: boolean;
  onTranslate: () => void;
  onOpenSettings: () => void;
}

export default function ToolbarControls({
  stale,
  loading,
  onTranslate,
  onOpenSettings,
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
      <button className="icon-btn" aria-label="Settings" onClick={onOpenSettings}>
        ⚙
      </button>
    </div>
  );
}
