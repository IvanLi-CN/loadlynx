import { useTranslation } from "react-i18next";
import { Select } from "../ui/field.tsx";

const LOCALES = [
  { code: "zh-CN", label: "🇨🇳 中文" },
  { code: "en", label: "🇺🇸 EN" },
] as const;

export function LanguageSwitcher() {
  const { i18n, t } = useTranslation();

  return (
    <label
      className="ll-form-control min-w-[120px]"
      htmlFor="loadlynx-language"
    >
      <span className="sr-only">{t("shell.language")}</span>
      <Select
        id="loadlynx-language"
        name="locale"
        className="ll-select-sm"
        value={i18n.language}
        aria-label={t("shell.language")}
        onChange={(event) => {
          const nextLanguage = event.target.value;
          window.localStorage.setItem("loadlynx.locale", nextLanguage);
          void i18n.changeLanguage(nextLanguage);
        }}
      >
        {LOCALES.map((locale) => (
          <option key={locale.code} value={locale.code}>
            {locale.label}
          </option>
        ))}
      </Select>
    </label>
  );
}
