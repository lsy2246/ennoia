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
  const date = normalizeDateValue(value);
  if (!date) {
    return "—";
  }
  return formatWithFallback(
    locale,
    {
      dateStyle: "medium",
      timeStyle: "short",
      timeZone,
    },
    date,
  );
}

export function formatDate(value: string | number | Date, locale: string, timeZone?: string) {
  const date = normalizeDateValue(value);
  if (!date) {
    return "—";
  }
  return formatWithFallback(
    locale,
    {
      dateStyle: "medium",
      timeZone,
    },
    date,
  );
}

export function formatTime(value: string | number | Date, locale: string, timeZone?: string) {
  const date = normalizeDateValue(value);
  if (!date) {
    return "—";
  }
  return formatWithFallback(
    locale,
    {
      timeStyle: "short",
      timeZone,
    },
    date,
  );
}

function formatWithFallback(
  locale: string,
  options: Intl.DateTimeFormatOptions,
  date: Date,
) {
  try {
    return new Intl.DateTimeFormat(locale, options).format(date);
  } catch {
    const { timeZone: _ignoredTimeZone, ...fallbackOptions } = options;
    return new Intl.DateTimeFormat(locale, fallbackOptions).format(date);
  }
}

function normalizeDateValue(value: string | number | Date) {
  if (value instanceof Date) {
    return Number.isNaN(value.getTime()) ? null : value;
  }

  if (typeof value === "number") {
    const normalized = value < 10_000_000_000 ? value * 1000 : value;
    const date = new Date(normalized);
    return Number.isNaN(date.getTime()) ? null : date;
  }

  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  if (/^\d+$/.test(trimmed)) {
    const numeric = Number(trimmed);
    if (!Number.isFinite(numeric)) {
      return null;
    }
    return normalizeDateValue(numeric);
  }

  const date = new Date(trimmed);
  return Number.isNaN(date.getTime()) ? null : date;
}
