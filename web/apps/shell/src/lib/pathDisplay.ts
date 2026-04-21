export function formatRelativePath(path: string, root?: string) {
  if (!path) {
    return "—";
  }
  const normalizedPath = path.replace(/\\/g, "/");
  const normalizedRoot = root?.replace(/\\/g, "/").replace(/\/$/, "");
  if (normalizedRoot && normalizedPath.toLowerCase().startsWith(normalizedRoot.toLowerCase())) {
    const relative = normalizedPath.slice(normalizedRoot.length).replace(/^\/+/, "");
    return relative || ".";
  }
  const parts = normalizedPath.split("/").filter(Boolean);
  if (parts.length <= 3) {
    return normalizedPath;
  }
  return parts.slice(-3).join("/");
}
