type NavigatorWithUAData = Navigator & {
  userAgentData?: {
    platform?: string;
  };
};

export function detectOperatorSystem(): string | null {
  if (typeof navigator === "undefined") {
    return null;
  }

  const browserNavigator = navigator as NavigatorWithUAData;
  const platform = browserNavigator.userAgentData?.platform ?? navigator.platform ?? "";
  const userAgent = navigator.userAgent ?? "";
  const combined = `${platform} ${userAgent}`;

  if (/MacIntel/i.test(platform) && navigator.maxTouchPoints > 1) {
    return "iPadOS";
  }
  if (/iPhone|iPad|iPod/i.test(combined)) {
    return "iOS";
  }
  if (/Android/i.test(combined)) {
    return "Android";
  }
  if (/CrOS/i.test(combined)) {
    return "ChromeOS";
  }
  if (/Windows|Win32|Win64|WOW64/i.test(combined)) {
    return "Windows";
  }
  if (/Macintosh|Mac OS X|MacIntel|MacPPC|Mac68K/i.test(combined)) {
    return "macOS";
  }
  if (/Linux|X11/i.test(combined)) {
    return "Linux";
  }

  return null;
}
