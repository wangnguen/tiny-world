// Ngôn ngữ của các cửa sổ React (Cài đặt, khung chat). Câu chữ nằm ở packages/core/src/i18n/vi.json, en.json;
// overlay không dùng file này (câu pet nói lấy qua packages/sim, `setLang`).
import { createContext, useContext } from "react";
import { fill, messages, type Lang, type Messages } from "@tinyworld/core";

/** Trước khi đọc được cài đặt: đoán theo ngôn ngữ của WebView (theo ngôn ngữ hiển thị của Windows). */
export function guessLang(): Lang {
  return navigator.language.toLowerCase().startsWith("vi") ? "vi" : "en";
}

/** Ngôn ngữ của cửa sổ, theo `Settings.lang`. */
export const LangContext = createContext<Lang>(guessLang());

export function useLang(): Lang {
  return useContext(LangContext);
}

/** Chữ của ngôn ngữ đang dùng trong cửa sổ. */
export function useMessages(): Messages {
  return messages(useLang());
}

export { fill };
