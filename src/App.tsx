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

export default function App() {
  const [text, setText] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [stale, setStale] = useState(false);
  const [language, setLanguage] = useState("English");
  const [showSettings, setShowSettings] = useState(false);
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

  return (
    <div className="app">
      <div className="toolbar" data-tauri-drag-region>
        <LanguagePicker value={language} onChange={handleLanguageChange} />
        <ToolbarControls
          stale={stale}
          loading={loading}
          onTranslate={handleTranslate}
          onOpenSettings={() => setShowSettings((v) => !v)}
        />
      </div>

      {showSettings && config ? (
        <SettingsPopover
          config={config}
          onSave={handleSaveSettings}
          onClose={() => setShowSettings(false)}
        />
      ) : (
        <TranslationDisplay loading={loading} text={text} error={error} />
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
