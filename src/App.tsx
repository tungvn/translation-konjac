import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import TranslationDisplay from "./components/TranslationDisplay";
import LanguagePicker from "./components/LanguagePicker";
import ToolbarControls from "./components/ToolbarControls";
import "./App.css";

interface AppConfig {
  gateway_url: string;
  provider: string;
  model: string;
  api_key: string;
  target_language: string;
  delta_threshold: number;
}

interface HistoryEntry {
  id: number;
  text: string;
  timestamp: number;
}

export default function App() {
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stale, setStale] = useState(false);
  const [language, setLanguage] = useState("English");
  const [showSettings, setShowSettings] = useState(false);
  const [showHistory, setShowHistory] = useState(false);
  const [config, setConfig] = useState<AppConfig | null>(null);

  useEffect(() => {
    invoke<AppConfig>("get_config").then((c) => {
      setConfig(c);
      setLanguage(c.target_language);
    });
    invoke<boolean>("get_stale").then(setStale);

    const unlisteners = [
      listen("capture-stale", () => setStale(true)),
      listen("translation-loading", () => {
        setLoading(true);
        setError(null);
      }),
      listen<string>("translation-updated", (e) => {
        setLoading(false);
        setStale(false);
        setText(e.payload);
      }),
      listen<string>("translation-error", (e) => {
        setLoading(false);
        setError(e.payload);
      }),
    ];

    return () => {
      unlisteners.forEach((p) => p.then((fn) => fn()));
    };
  }, []);

  const handleClear = useCallback(() => {
    setText("");
    setStale(false);
    setError(null);
  }, []);

  const handleTranslate = useCallback(() => {
    setStale(false);
    setLoading(true);
    setError(null);
    invoke("translate_now");
  }, []);

  const handleLanguageChange = useCallback((lang: string) => {
    setLanguage(lang);
    invoke("set_target_language", { language: lang });
    setStale(true);
  }, []);

  const handleSaveSettings = useCallback((updated: AppConfig) => {
    invoke("save_config", { config: updated }).then(() => {
      setConfig(updated);
      setLanguage(updated.target_language);
      setShowSettings(false);
    });
  }, []);

  const handleToggleHistory = useCallback(() => {
    setShowHistory((v) => !v);
    setShowSettings(false);
  }, []);

  const handleRestoreHistory = useCallback((entry: HistoryEntry) => {
    setText(entry.text);
    setShowHistory(false);
  }, []);

  return (
    <div className="app">
      <div className="toolbar" data-tauri-drag-region>
        <LanguagePicker value={language} onChange={handleLanguageChange} />
        <ToolbarControls
          stale={stale}
          loading={loading}
          showHistory={showHistory}
          onTranslate={handleTranslate}
          onOpenSettings={() => { setShowSettings((v) => !v); setShowHistory(false); }}
          onToggleHistory={handleToggleHistory}
        />
      </div>

      {showSettings && config ? (
        <SettingsPopover
          config={config}
          onSave={handleSaveSettings}
          onClose={() => setShowSettings(false)}
        />
      ) : showHistory ? (
        <HistoryPanel
          onRestore={handleRestoreHistory}
          onClose={() => setShowHistory(false)}
        />
      ) : (
        <TranslationDisplay loading={loading} text={text} error={error} onClear={handleClear} />
      )}
    </div>
  );
}

function HistoryPanel({
  onRestore,
  onClose,
}: {
  onRestore: (entry: HistoryEntry) => void;
  onClose: () => void;
}) {
  const [entries, setEntries] = useState<HistoryEntry[]>([]);

  useEffect(() => {
    invoke<HistoryEntry[]>("get_history").then(setEntries);
  }, []);

  function handleDelete(id: number) {
    invoke("delete_history_item", { id }).then(() =>
      setEntries((prev) => prev.filter((e) => e.id !== id))
    );
  }

  function handleClearAll() {
    invoke("clear_history").then(() => setEntries([]));
  }

  return (
    <div className="history-panel">
      <div className="history-header">
        <span className="history-title">History</span>
        {entries.length > 0 && (
          <button className="history-clear-all" onClick={handleClearAll}>
            Clear all
          </button>
        )}
      </div>
      {entries.length === 0 ? (
        <div className="history-empty">No history yet</div>
      ) : (
        <ul className="history-list">
          {entries.map((entry) => (
            <li key={entry.id} className="history-item">
              <button
                className="history-item-text"
                onClick={() => onRestore(entry)}
                title="Restore this translation"
              >
                {entry.text.slice(0, 80)}{entry.text.length > 80 ? "…" : ""}
              </button>
              <button
                className="history-item-delete icon-btn"
                onClick={() => handleDelete(entry.id)}
                aria-label="Delete"
                title="Delete"
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function SettingsPopover({
  config,
  onSave,
  onClose,
}: {
  config: AppConfig;
  onSave: (c: AppConfig) => void;
  onClose: () => void;
}) {
  const [draft, setDraft] = useState(config);

  return (
    <div className="settings">
      <label>
        Gateway URL
        <input
          value={draft.gateway_url}
          onChange={(e) => setDraft({ ...draft, gateway_url: e.target.value })}
        />
      </label>
      <label>
        API Key
        <input
          type="password"
          value={draft.api_key}
          onChange={(e) => setDraft({ ...draft, api_key: e.target.value })}
        />
      </label>
      <label>
        Model
        <input
          value={draft.model}
          onChange={(e) => setDraft({ ...draft, model: e.target.value })}
        />
      </label>
      <label>
        Delta threshold ({draft.delta_threshold.toFixed(2)})
        <input
          type="range"
          min="0.01"
          max="0.5"
          step="0.01"
          value={draft.delta_threshold}
          onChange={(e) =>
            setDraft({ ...draft, delta_threshold: parseFloat(e.target.value) })
          }
        />
      </label>
      <div className="settings-actions">
        <button onClick={() => onSave(draft)}>Save</button>
        <button onClick={onClose}>Cancel</button>
      </div>
    </div>
  );
}
