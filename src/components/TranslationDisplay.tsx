import { useState } from "react";

interface Props {
  loading: boolean;
  text: string;
  error: string | null;
  onClear: () => void;
  fontSize: number;
}

function CopyIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
      <rect x="5" y="5" width="9" height="9" rx="1.5" />
      <path d="M11 5V3.5A1.5 1.5 0 0 0 9.5 2H3.5A1.5 1.5 0 0 0 2 3.5v6A1.5 1.5 0 0 0 3.5 11H5" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <polyline points="2.5,8.5 6,12 13.5,4" />
    </svg>
  );
}

function ClearIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <line x1="3" y1="3" x2="13" y2="13" />
      <line x1="13" y1="3" x2="3" y2="13" />
    </svg>
  );
}

export default function TranslationDisplay({ loading, text, error, onClear, fontSize }: Props) {
  const [copied, setCopied] = useState(false);

  function handleCopy() {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }

  if (loading) {
    return (
      <div className="translation-body center">
        <div className="spinner" role="status" aria-label="Translating" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="translation-body center">
        <span className="error">{error}</span>
      </div>
    );
  }

  if (!text) {
    return (
      <div className="translation-body center">
        <span className="placeholder">Position the window over text, then click ↻ to translate</span>
      </div>
    );
  }

  return (
    <div className="translation-body translation-body--has-text">
      <div className="text-actions">
        <button
          className={`copy-btn icon-btn${copied ? " copy-btn--done" : ""}`}
          onClick={handleCopy}
          title={copied ? "Copied!" : "Copy"}
          aria-label="Copy translation"
        >
          {copied ? <CheckIcon /> : <CopyIcon />}
        </button>
        <button
          className="clear-btn icon-btn"
          onClick={onClear}
          title="Clear"
          aria-label="Clear translation"
        >
          <ClearIcon />
        </button>
      </div>
      <pre className="translation-text" style={{ fontSize }}>{text}</pre>
    </div>
  );
}
