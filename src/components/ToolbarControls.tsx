interface Props {
  capturing: boolean;
  onPause: () => void;
  onResume: () => void;
  onOpenSettings: () => void;
}

export default function ToolbarControls({ capturing, onPause, onResume, onOpenSettings }: Props) {
  return (
    <div className="toolbar-controls">
      <button
        className="icon-btn"
        aria-label={capturing ? "Pause" : "Resume"}
        onClick={capturing ? onPause : onResume}
      >
        {capturing ? "⏸" : "▶"}
      </button>
      <button
        className="icon-btn"
        aria-label="Settings"
        onClick={onOpenSettings}
      >
        ⚙
      </button>
    </div>
  );
}
