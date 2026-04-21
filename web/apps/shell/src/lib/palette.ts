export type WorkbenchPalette = {
  id: string;
  label: string;
  description: string;
};

export const WORKBENCH_PALETTES: WorkbenchPalette[] = [
  {
    id: "graphite",
    label: "Graphite Studio",
    description: "深石墨底色、冷蓝边线、适合长时间工作台。",
  },
  {
    id: "ember",
    label: "Ember Terminal",
    description: "黑铜工业风、暖橙焦点、适合高密度日志和任务。",
  },
  {
    id: "paper",
    label: "Paper Lab",
    description: "浅色纸感、墨绿强调、适合文档和配置编辑。",
  },
  {
    id: "aurora",
    label: "Aurora Bay",
    description: "蓝绿极光、深海面板、适合多窗口协作与长期观察。",
  },
];

const PALETTE_STORAGE_KEY = "ennoia.workbench.palette";
const THEME_BRIDGE_VARIABLES = [
  "--bg",
  "--bg-elevated",
  "--bg-soft",
  "--bg-panel",
  "--line",
  "--line-strong",
  "--text",
  "--text-muted",
  "--accent",
  "--accent-soft",
];

export function readWorkbenchPalette() {
  if (typeof localStorage === "undefined") {
    return "graphite";
  }
  const saved = localStorage.getItem(PALETTE_STORAGE_KEY);
  return WORKBENCH_PALETTES.some((palette) => palette.id === saved) ? saved! : "graphite";
}

export function applyWorkbenchPalette(
  paletteId: string,
  options: { clearRuntimeTheme?: boolean } = {},
) {
  const next = WORKBENCH_PALETTES.some((palette) => palette.id === paletteId)
    ? paletteId
    : "graphite";
  document.documentElement.dataset.palette = next;
  if (options.clearRuntimeTheme ?? true) {
    for (const variable of THEME_BRIDGE_VARIABLES) {
      document.documentElement.style.removeProperty(variable);
    }
  }
  localStorage.setItem(PALETTE_STORAGE_KEY, next);
  return next;
}
