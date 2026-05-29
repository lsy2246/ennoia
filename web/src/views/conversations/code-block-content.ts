export function trimTrailingBlankLines(value: string) {
  return value.replace(/[ \t]*(?:\r?\n[ \t]*)+$/, "");
}

export function normalizeFencedCodeBody(body: string) {
  return trimTrailingBlankLines(body
    .replace(/^```[^\r\n]*\r?\n?/, "")
    .replace(/\r?\n?```$/, ""));
}

export function normalizeCodeBlockText(body: string) {
  return normalizeFencedCodeBody(body);
}
