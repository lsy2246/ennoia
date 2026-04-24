import React, { useState, useEffect, useRef } from "react";
import { useUiStore, useUiHelpers } from "@/stores/ui";

export type DockPosition = "left" | "right" | "top" | "bottom" | "floating";
export const ENNOIA_ROUTE_DRAG_MIME = "application/ennoia-route";
let activeDraggedNavItem: NavItem | null = null;

export interface NavItem {
  id: string;
  href: string;
  icon: string;
  label: string;
  hint: string;
  source: "builtin" | "extension";
}

export function getActiveDraggedNavItem() {
  return activeDraggedNavItem;
}

interface OmniDockProps {
  navItems: NavItem[];
  activeId?: string;
  position: DockPosition;
  expanded: boolean;
  onPositionChange: (pos: DockPosition) => void;
  onExpandedChange: (expanded: boolean) => void;
  onOpenItem: (item: NavItem) => void;
}

interface DragState {
  isDragging: boolean;
  startX: number;
  startY: number;
  currentX: number;
  currentY: number;
  previewPos: DockPosition | null;
}

type PreferenceOption = {
  value: string;
  label: string;
  previewColor?: string;
};

interface PreferenceSelectProps {
  label: string;
  value: string;
  options: PreferenceOption[];
  open: boolean;
  onToggle: () => void;
  onSelect: (value: string) => void;
}

function PreferenceSelect({
  label,
  value,
  options,
  open,
  onToggle,
  onSelect,
}: PreferenceSelectProps) {
  const selected = options.find((item) => item.value === value) ?? options[0];

  return (
    <div className={`preference-select ${open ? "preference-select--open" : ""}`}>
      <button
        type="button"
        className="preference-select__trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={onToggle}
      >
        <span className="preference-select__trigger-copy">
          <span className="preference-select__caption">{label}</span>
          <span className="preference-select__value">
            {selected?.previewColor ? (
              <span
                className="preference-select__swatch"
                style={{ backgroundColor: selected.previewColor }}
                aria-hidden="true"
              />
            ) : null}
            <span>{selected?.label ?? value}</span>
          </span>
        </span>
        <span className="preference-select__chevron" aria-hidden="true">⌄</span>
      </button>

      {open ? (
        <div className="preference-select__menu" role="listbox" aria-label={label}>
          {options.map((option) => {
            const active = option.value === value;
            return (
              <button
                key={option.value}
                type="button"
                role="option"
                aria-selected={active}
                className={`preference-select__option ${active ? "preference-select__option--active" : ""}`}
                onClick={() => onSelect(option.value)}
              >
                <span className="preference-select__value">
                  {option.previewColor ? (
                    <span
                      className="preference-select__swatch"
                      style={{ backgroundColor: option.previewColor }}
                      aria-hidden="true"
                    />
                  ) : null}
                  <span>{option.label}</span>
                </span>
                {active ? <span className="preference-select__check" aria-hidden="true">✓</span> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

export function OmniDock({
  navItems,
  activeId,
  position,
  expanded,
  onPositionChange,
  onExpandedChange,
  onOpenItem,
}: OmniDockProps) {
  const { availableLocales, availableThemes, t } = useUiHelpers();
  const uiState = useUiStore();

  const [showPreferences, setShowPreferences] = useState(false);
  const [openPreferenceMenu, setOpenPreferenceMenu] = useState<"theme" | "locale" | null>(null);
  const [drag, setDrag] = useState<DragState>({
    isDragging: false,
    startX: 0, startY: 0, currentX: 0, currentY: 0, previewPos: null
  });

  const dockRef = useRef<HTMLDivElement>(null);
  const preferencesRef = useRef<HTMLDivElement>(null);
  const settingsWrapperRef = useRef<HTMLDivElement>(null);
  const [popoverEdgeStyle] = useState<React.CSSProperties>({});

  const popoverRefCallback = (node: HTMLDivElement | null) => {
    preferencesRef.current = node;
    if (!node) return;
    const wrapper = settingsWrapperRef.current;
    const nav = dockRef.current;
    if (!wrapper || !nav) return;
    const pos = drag.isDragging ? "floating" : position;
    if (pos === "bottom" || pos === "top") {
      const wrapperRect = wrapper.getBoundingClientRect();
      const navRect = nav.getBoundingClientRect();
      const centerX = wrapperRect.left + wrapperRect.width / 2 - navRect.left;
      node.style.left = `${centerX}px`;
      node.style.transform = "translateX(-50%)";
      if (pos === "bottom") {
        node.style.bottom = `calc(100% + 8px)`;
        node.style.top = "auto";
      } else {
        node.style.top = `calc(100% + 8px)`;
        node.style.bottom = "auto";
      }
    }
  };

  const themeOptions: PreferenceOption[] = availableThemes.map((theme) => ({
    value: theme.id,
    label: theme.label,
    previewColor: theme.previewColor,
  }));

  const localeOptions: PreferenceOption[] = availableLocales.map((locale) => ({
    value: locale,
    label: locale,
  }));

  // When dragging, use the preview pos to determine if it should render vertically or horizontally
  // so the user sees it transform dynamically before dropping.
  const visualPosition = drag.isDragging && drag.previewPos && drag.previewPos !== "floating"
    ? drag.previewPos
    : position;

  const isVertical =
    visualPosition === "left" ||
    visualPosition === "right" ||
    (visualPosition === "floating" && (position === "left" || position === "right"));

  // --- 真正的拖拽与磁吸引擎 ---
  useEffect(() => {
    if (!drag.isDragging) return;

    const SNAP_ZONE = 150;

    const handleMouseMove = (e: MouseEvent) => {
      const { innerWidth: width, innerHeight: height } = window;
      const { clientX: x, clientY: y } = e;

      let preview: DockPosition | null = "floating";

      if (x < SNAP_ZONE) preview = "left";
      else if (x > width - SNAP_ZONE) preview = "right";
      else if (y < SNAP_ZONE) preview = "top";
      else if (y > height - SNAP_ZONE) preview = "bottom";

      setDrag(prev => ({
        ...prev,
        currentX: x,
        currentY: y,
        previewPos: preview
      }));
    };

    const handleMouseUp = () => {
      if (drag.previewPos && drag.previewPos !== "floating") {
        onPositionChange(drag.previewPos);
      }
      setDrag(prev => ({ ...prev, isDragging: false, previewPos: null }));
      document.body.style.cursor = "default";
      document.body.style.userSelect = "auto";
    };

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    document.body.style.cursor = "grabbing";
    document.body.style.userSelect = "none";

    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "default";
      document.body.style.userSelect = "auto";
    };
  }, [drag.isDragging, drag.previewPos, onPositionChange]);

  const handleDragStart = (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    setShowPreferences(false);

    setDrag({
      isDragging: true,
      startX: e.clientX,
      startY: e.clientY,
      currentX: e.clientX,
      currentY: e.clientY,
      previewPos: position
    });
  };

  const togglePreferences = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowPreferences((current) => {
      const next = !current;
      if (!next) {
        setOpenPreferenceMenu(null);
      }
      return next;
    });
  };

  const toggleExpanded = () => {
    onExpandedChange(!expanded);
  };

  async function changeTheme(themeId: string) {
    await uiState.savePreferences({ ...uiState, theme_id: themeId });
    setOpenPreferenceMenu(null);
  }

  async function changeLocale(locale: string) {
    await uiState.savePreferences({ ...uiState, locale });
    setOpenPreferenceMenu(null);
  }

  useEffect(() => {
    if (!showPreferences) {
      setOpenPreferenceMenu(null);
    }
  }, [showPreferences]);

  useEffect(() => {
    if (!showPreferences) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (!preferencesRef.current?.contains(event.target as Node)) {
        setOpenPreferenceMenu(null);
      }
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setOpenPreferenceMenu(null);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [showPreferences]);

  const floatingStyle: React.CSSProperties = drag.isDragging ? {
    position: 'fixed',
    left: `${drag.currentX}px`,
    top: `${drag.currentY}px`,
    transform: 'translate(-50%, -20px)',
    transition: 'none',
    zIndex: 9999,
  } : {};

  // Resolve the proper arrow icon based on position and state
  const getToggleIcon = () => {
    if (position === "left") return expanded ? "⟨" : "⟩";
    if (position === "right") return expanded ? "⟩" : "⟨";
    if (position === "top") return expanded ? "︿" : "﹀";
    return expanded ? "﹀" : "︿";
  };

  const setNavDragImage = (event: React.DragEvent, item: NavItem) => {
    const dragImage = document.createElement("div");
    dragImage.className = "dock-drag-image";
    dragImage.innerHTML = `<span>${item.icon}</span><strong>${item.label}</strong>`;
    document.body.appendChild(dragImage);
    event.dataTransfer.setDragImage(dragImage, 18, 18);
    window.setTimeout(() => dragImage.remove(), 0);
  };

  return (
    <>
      {drag.isDragging && drag.previewPos && drag.previewPos !== "floating" && (
        <div className={`dock-preview-zone dock-preview-zone--${drag.previewPos}`} />
      )}

      <nav
        ref={dockRef}
        className={`omni-dock ${drag.isDragging ? "omni-dock--dragging" : ""} ${expanded ? "omni-dock--expanded" : ""}`}
        data-position={drag.isDragging ? drag.previewPos || "floating" : position}
        data-expanded={expanded}
        style={floatingStyle}
      >
        {(position === "left" || position === "right" || position === "top" || position === "bottom") && (
          <button
            type="button"
            className="dock-edge-toggle"
            onClick={toggleExpanded}
            title={expanded ? "收起导航" : "展开导航"}
          >
            <span>{getToggleIcon()}</span>
          </button>
        )}

        <div className="dock-container" data-vertical={isVertical}>
          <div
            className="dock-handle"
            data-vertical={isVertical}
            onMouseDown={handleDragStart}
            title="按住拖拽到任意边缘"
          >
            <div className="dock-handle-grip" />
          </div>

          <div className="dock-divider" data-vertical={isVertical} />

          {navItems.map((item) => (
            <button
              type="button"
              key={`${item.source}:${item.id}`}
              className={activeId === item.id ? "dock-item dock-item--active" : "dock-item"}
              title={expanded ? undefined : item.label}
              onClick={() => onOpenItem(item)}
              draggable="true"
              onDragStart={(e) => {
                activeDraggedNavItem = item;
                e.dataTransfer.setData(ENNOIA_ROUTE_DRAG_MIME, JSON.stringify(item));
                e.dataTransfer.setData("text/plain", item.label);
                e.dataTransfer.effectAllowed = "copy";
                setNavDragImage(e, item);
              }}
              onDragEnd={() => {
                activeDraggedNavItem = null;
              }}
            >
              <span className="dock-icon">{item.icon}</span>
              {expanded && (
                <span className="dock-copy">
                  <span className="dock-label">{item.label}</span>
                  <span className="dock-hint">{item.hint}</span>
                </span>
              )}
            </button>
          ))}

          <div className="dock-divider" data-vertical={isVertical} />

          <div style={{ position: "relative" }} ref={settingsWrapperRef}>
            <button
              type="button"
              className={`dock-item ${showPreferences ? "dock-item--active" : ""}`}
              onClick={togglePreferences}
              title={expanded ? undefined : "偏好设置"}
            >
              <span className="dock-icon">☷</span>
              {expanded && (
                <span className="dock-copy">
                  <span className="dock-label">偏好设置</span>
                  <span className="dock-hint">主题、语言与界面偏好</span>
                </span>
              )}
            </button>
          </div>
        </div>

        {showPreferences && (
          <>
            <div className="popover-backdrop" onClick={() => setShowPreferences(false)} />
            <div
              ref={popoverRefCallback}
              className="dock-settings-popover"
              data-position={drag.isDragging ? "floating" : position}
              style={popoverEdgeStyle}
            >
              <div className="settings-group">
                <PreferenceSelect
                  label={t("web.settings.theme", "主题")}
                  value={uiState.themeId}
                  options={themeOptions}
                  open={openPreferenceMenu === "theme"}
                  onToggle={() => setOpenPreferenceMenu((current) => current === "theme" ? null : "theme")}
                  onSelect={(nextValue) => void changeTheme(nextValue)}
                />
              </div>
              <div className="settings-group">
                <PreferenceSelect
                  label={t("web.settings.language", "语言")}
                  value={uiState.locale}
                  options={localeOptions}
                  open={openPreferenceMenu === "locale"}
                  onToggle={() => setOpenPreferenceMenu((current) => current === "locale" ? null : "locale")}
                  onSelect={(nextValue) => void changeLocale(nextValue)}
                />
              </div>
            </div>
          </>
        )}
      </nav>
    </>
  );
}
