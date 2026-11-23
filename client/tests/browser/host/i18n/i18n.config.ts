import en from "../../../../i18n/en.json";
import ja from "../../../../i18n/ja.json";
import ko from "../../../../i18n/ko.json";
import zhCN from "../../../../i18n/zh-CN.json";
import common from "#theater-host-words";

// The words the screens read: the bundle's own packs, and the layer's `common` keys a console answers, which the test
// writes into the host's cache before the host starts (the console's own words where it is handed them, the keys
// themselves otherwise).

export default defineI18nConfig(() => ({
  legacy: false,
  locale: "ko",
  fallbackLocale: "ko",
  missingWarn: false,
  fallbackWarn: false,
  messages: { ko: { ...ko, ...common }, en: { ...en, ...common }, ja: { ...ja, ...common }, "zh-CN": { ...zhCN, ...common } }
}));
