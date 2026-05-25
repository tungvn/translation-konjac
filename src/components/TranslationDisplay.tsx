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
      <div className="translation-body center error">
        <span>{error}</span>
      </div>
    );
  }

  if (!text) {
    return (
      <div className="translation-body center placeholder">
        <span>Move the window over text to translate</span>
      </div>
    );
  }

  return (
    <div className="translation-body">
      <p className="translation-text">{text}</p>
    </div>
  );
}
