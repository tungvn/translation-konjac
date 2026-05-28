interface Props {
  loading: boolean;
  text: string;
  error: string | null;
}

export default function TranslationDisplay({ loading, text, error }: Props) {
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
    <div className="translation-body">
      <pre className="translation-text">{text}</pre>
    </div>
  );
}
