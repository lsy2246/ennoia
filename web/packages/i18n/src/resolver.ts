import type { LocalizedText } from "@ennoia/ui-sdk";

import type { BundleSelector, I18nRegistry, TranslationBundle } from "./types";

const EMPTY_BUNDLES: TranslationBundle[] = [];

function resolveBundles(
  locale: string,
  source: I18nRegistry | TranslationBundle[],
  selector?: BundleSelector,
): TranslationBundle[] {
  return Array.isArray(source) ? source : source.getBundles(locale, selector);
}

export function resolveLocalizedText(
  text: LocalizedText,
  locale: string,
  source: I18nRegistry | TranslationBundle[] = EMPTY_BUNDLES,
  selector?: BundleSelector,
): string {
  const bundles = resolveBundles(locale, source, selector);
  for (const bundle of bundles) {
    const value = bundle.messages[text.key];
    if (value) {
      return value;
    }
  }
  return text.fallback;
}

export function translate(
  locale: string,
  key: string,
  fallback: string,
  source: I18nRegistry | TranslationBundle[] = EMPTY_BUNDLES,
  selector?: BundleSelector,
): string {
  return resolveLocalizedText({ key, fallback }, locale, source, selector);
}

export function formatDateTime(value: string | number | Date, locale: string, timeZone?: string) {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone,
  }).format(new Date(value));
}

export function formatDate(value: string | number | Date, locale: string, timeZone?: string) {
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeZone,
  }).format(new Date(value));
}

export function formatTime(value: string | number | Date, locale: string, timeZone?: string) {
  return new Intl.DateTimeFormat(locale, {
    timeStyle: "short",
    timeZone,
  }).format(new Date(value));
}
