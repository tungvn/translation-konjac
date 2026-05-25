const LANGUAGES = [
  "English", "Vietnamese", "Japanese", "Chinese (Simplified)",
  "Chinese (Traditional)", "Korean", "French", "German", "Spanish",
  "Portuguese", "Italian", "Russian", "Arabic", "Hindi", "Thai",
  "Indonesian", "Dutch", "Polish", "Turkish", "Swedish",
];

interface Props {
  value: string;
  onChange: (lang: string) => void;
}

export default function LanguagePicker({ value, onChange }: Props) {
  return (
    <select
      className="language-picker"
      value={value}
      onChange={(e) => onChange(e.target.value)}
    >
      {LANGUAGES.map((lang) => (
        <option key={lang} value={lang}>
          {lang}
        </option>
      ))}
    </select>
  );
}
