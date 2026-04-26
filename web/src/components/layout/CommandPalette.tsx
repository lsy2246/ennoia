import { useEffect, useMemo, useState } from "react";

export type CommandPaletteAction = {
  id: string;
  title: string;
  hint?: string;
  keywords?: string[];
  run: () => void | Promise<void>;
};

type Props = {
  open: boolean;
  actions: CommandPaletteAction[];
  onClose: () => void;
  t: (key: string, fallback: string) => string;
};

export function CommandPalette({ open, actions, onClose, t }: Props) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(() => {
    if (!open) {
      setQuery("");
      setSelectedIndex(0);
    }
  }, [open]);

  const filtered = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    if (!normalized) {
      return actions;
    }
    return actions.filter((action) => {
      const haystacks = [
        action.title.toLowerCase(),
        action.hint?.toLowerCase() ?? "",
        ...(action.keywords ?? []).map((item) => item.toLowerCase()),
      ];
      return haystacks.some((item) => item.includes(normalized));
    });
  }, [actions, query]);

  useEffect(() => {
    if (selectedIndex >= filtered.length) {
      setSelectedIndex(0);
    }
  }, [filtered.length, selectedIndex]);

  useEffect(() => {
    if (!open) {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setSelectedIndex((current) => (current + 1) % Math.max(filtered.length, 1));
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setSelectedIndex((current) => (current - 1 + Math.max(filtered.length, 1)) % Math.max(filtered.length, 1));
        return;
      }
      if (event.key === "Enter") {
        const action = filtered[selectedIndex];
        if (!action) {
          return;
        }
        event.preventDefault();
        void Promise.resolve(action.run()).finally(onClose);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [filtered, onClose, open, selectedIndex]);

  if (!open) {
    return null;
  }

  return (
    <div className="command-palette-backdrop" onMouseDown={onClose}>
      <section
        className="command-palette"
        onMouseDown={(event) => event.stopPropagation()}
        aria-label={t("web.command_palette.title", "命令面板")}
      >
        <input
          autoFocus
          className="command-palette__input"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={t("web.command_palette.placeholder", "输入命令，例如：清空上下文、切换分支、打开会话")}
        />
        <div className="command-palette__list">
          {filtered.length === 0 ? (
            <div className="empty-card">{t("web.command_palette.empty", "没有匹配的命令。")}</div>
          ) : (
            filtered.map((action, index) => (
              <button
                key={action.id}
                type="button"
                className={index === selectedIndex ? "command-palette__item command-palette__item--active" : "command-palette__item"}
                onMouseEnter={() => setSelectedIndex(index)}
                onClick={() => void Promise.resolve(action.run()).finally(onClose)}
              >
                <strong>{action.title}</strong>
                {action.hint ? <span>{action.hint}</span> : null}
              </button>
            ))
          )}
        </div>
        <div className="command-palette__footer">
          <span>{t("web.command_palette.hint", "回车执行，方向键切换，Esc 关闭")}</span>
        </div>
      </section>
    </div>
  );
}
