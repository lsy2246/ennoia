import { apiUrl } from "@ennoia/api-client";

const CANONICAL_ARTIFACT_PATH = /^\/api\/agents\/[^/]+\/artifacts\/(.+)$/;
const PARSE_BASE_URL = "http://ennoia.local";

function parseCanonicalArtifactUrl(rawUrl: string | null | undefined) {
  const value = rawUrl?.trim() ?? "";
  if (!value.startsWith("/api/agents/")) {
    return undefined;
  }

  let parsed: URL;
  try {
    parsed = new URL(value, PARSE_BASE_URL);
  } catch {
    return undefined;
  }

  const match = parsed.pathname.match(CANONICAL_ARTIFACT_PATH);
  const artifactPath = match?.[1];
  if (!artifactPath) {
    return undefined;
  }

  return {
    artifactPath,
    path: `${parsed.pathname}${parsed.search}${parsed.hash}`,
  };
}

export function getMessageAttachmentDownloadName(rawUrl: string | null | undefined) {
  const artifact = parseCanonicalArtifactUrl(rawUrl);
  if (!artifact) {
    return undefined;
  }

  const filename = artifact.artifactPath.split("/").filter(Boolean).at(-1);
  if (!filename) {
    return "artifact";
  }

  try {
    return decodeURIComponent(filename);
  } catch {
    return filename;
  }
}

export function resolveMessageAttachmentUrl(rawUrl: string | null | undefined) {
  const artifact = parseCanonicalArtifactUrl(rawUrl);
  if (!artifact) {
    return undefined;
  }
  return apiUrl(artifact.path);
}

export function resolveMessageAttachmentDownloadUrl(rawUrl: string | null | undefined) {
  const artifact = parseCanonicalArtifactUrl(rawUrl);
  if (!artifact) {
    return undefined;
  }

  const url = new URL(artifact.path, PARSE_BASE_URL);
  url.searchParams.set("download", "1");
  return apiUrl(`${url.pathname}${url.search}${url.hash}`);
}

export function resolveMessageAttachmentLinkProps(rawUrl: string | null | undefined) {
  const downloadName = getMessageAttachmentDownloadName(rawUrl);
  const downloadHref = resolveMessageAttachmentDownloadUrl(rawUrl);
  if (downloadHref) {
    return {
      download: downloadName,
      href: downloadHref,
      onClick: createMessageAttachmentDownloadClickHandler(downloadHref, downloadName),
    };
  }

  return {
    href: rawUrl ?? undefined,
    rel: "noreferrer",
    target: "_blank",
  };
}

type DownloadClickEvent = {
  preventDefault: () => void;
};

type DownloadArtifact = (url: string, filename: string | undefined) => Promise<void> | void;

export function createMessageAttachmentDownloadClickHandler(
  url: string,
  filename?: string,
  downloadArtifact: DownloadArtifact = downloadMessageAttachment,
) {
  return async (event: DownloadClickEvent) => {
    event.preventDefault();
    await downloadArtifact(url, filename);
  };
}

async function downloadMessageAttachment(url: string, filename?: string) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`download failed: ${response.status}`);
  }
  const blob = await response.blob();
  const objectUrl = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = objectUrl;
  link.download = filename || "artifact";
  link.style.display = "none";
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(objectUrl);
}
