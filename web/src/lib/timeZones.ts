export type TimeZoneOption = {
  value: string;
  label: string;
};

export type TimeZoneOptionGroup = {
  label: string;
  options: TimeZoneOption[];
};

const COMMON_TIME_ZONES = [
  "UTC",
  "Asia/Shanghai",
  "Asia/Tokyo",
  "Asia/Singapore",
  "Europe/London",
  "Europe/Berlin",
  "America/New_York",
  "America/Los_Angeles",
] as const;

type Translate = (key: string, fallback: string) => string;

export function getBrowserTimeZone() {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
}

export function buildTimeZoneOptionGroups(t: Translate, includeBrowserDefault: boolean) {
  const browserTimeZone = getBrowserTimeZone();
  const allTimeZones = getSupportedTimeZones();
  const used = new Set<string>();
  const groups: TimeZoneOptionGroup[] = [];

  if (includeBrowserDefault) {
    groups.push({
      label: t("settings.timezone.default_group", "Default"),
      options: [
        {
          value: "",
          label: t("settings.timezone.browser_default", "Browser default"),
        },
      ],
    });
  }

  groups.push({
    label: t("settings.timezone.detected_group", "Detected"),
    options: [browserTimeZone].map((timeZone) => {
      used.add(timeZone);
      return buildTimeZoneOption(timeZone);
    }),
  });

  const commonOptions = COMMON_TIME_ZONES.filter((timeZone) => !used.has(timeZone)).map(
    (timeZone) => {
      used.add(timeZone);
      return buildTimeZoneOption(timeZone);
    },
  );
  if (commonOptions.length > 0) {
    groups.push({
      label: t("settings.timezone.common_group", "Common"),
      options: commonOptions,
    });
  }

  const allOptions = allTimeZones
    .filter((timeZone) => !used.has(timeZone))
    .map((timeZone) => buildTimeZoneOption(timeZone));
  if (allOptions.length > 0) {
    groups.push({
      label: t("settings.timezone.all_group", "All time zones"),
      options: allOptions,
    });
  }

  return groups;
}

function getSupportedTimeZones() {
  const intlWithValues = Intl as typeof Intl & {
    supportedValuesOf?: (key: "timeZone") => string[];
  };
  return intlWithValues.supportedValuesOf?.("timeZone") ?? [...COMMON_TIME_ZONES];
}

function buildTimeZoneOption(timeZone: string): TimeZoneOption {
  const offset = getTimeZoneOffset(timeZone);
  return {
    value: timeZone,
    label: offset ? `${timeZone} · ${offset}` : timeZone,
  };
}

function getTimeZoneOffset(timeZone: string) {
  try {
    return new Intl.DateTimeFormat("en-US", {
      timeZone,
      timeZoneName: "shortOffset",
    })
      .formatToParts(new Date())
      .find((part) => part.type === "timeZoneName")?.value;
  } catch {
    return null;
  }
}
