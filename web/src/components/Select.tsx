import { useState, useRef, useEffect, useCallback } from "react";

export type SelectOption = {
  value: string;
  label: string;
  group?: string;
};

type SelectProps = {
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
};

export function Select({ value, options, onChange, placeholder, className }: SelectProps) {
  const [open, setOpen] = useState(false);
  const [focusIndex, setFocusIndex] = useState(-1);
  const containerRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  const selected = options.find((o) => o.value === value);

  const close = useCallback(() => {
    setOpen(false);
    setFocusIndex(-1);
  }, []);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) close();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open, close]);

  useEffect(() => {
    if (!open || focusIndex < 0) return;
    const items = listRef.current?.querySelectorAll("[data-index]");
    (items?.[focusIndex] as HTMLElement)?.scrollIntoView({ block: "nearest" });
  }, [open, focusIndex]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!open) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp" || e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        setOpen(true);
        const idx = options.findIndex((o) => o.value === value);
        setFocusIndex(idx >= 0 ? idx : 0);
      }
      return;
    }
    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setFocusIndex((i) => Math.min(i + 1, options.length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setFocusIndex((i) => Math.max(i - 1, 0));
        break;
      case "Enter":
      case " ":
        e.preventDefault();
        if (focusIndex >= 0 && focusIndex < options.length) {
          onChange(options[focusIndex].value);
          close();
        }
        break;
      case "Escape":
        e.preventDefault();
        close();
        break;
    }
  };

  const groups = options.reduce<Map<string, SelectOption[]>>((acc, opt) => {
    const key = opt.group ?? "";
    if (!acc.has(key)) acc.set(key, []);
    acc.get(key)!.push(opt);
    return acc;
  }, new Map());

  let flatIndex = 0;

  return (
    <div ref={containerRef} className={`custom-select ${open ? "custom-select--open" : ""} ${className ?? ""}`}>
      <button
        type="button"
        className="custom-select__trigger"
        onClick={() => {
          setOpen(!open);
          if (!open) {
            const idx = options.findIndex((o) => o.value === value);
            setFocusIndex(idx >= 0 ? idx : 0);
          }
        }}
        onKeyDown={handleKeyDown}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <span>{selected?.label ?? placeholder ?? ""}</span>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>
      {open && (
        <ul ref={listRef} className="custom-select__menu" role="listbox">
          {[...groups.entries()].map(([group, items]) => (
            <li key={group} role="presentation">
              {group && <div className="custom-select__group">{group}</div>}
              {items.map((opt) => {
                const idx = flatIndex++;
                return (
                  <div
                    key={opt.value}
                    role="option"
                    data-index={idx}
                    aria-selected={opt.value === value}
                    className={`custom-select__option ${opt.value === value ? "custom-select__option--selected" : ""} ${idx === focusIndex ? "custom-select__option--focused" : ""}`}
                    onMouseEnter={() => setFocusIndex(idx)}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      onChange(opt.value);
                      close();
                    }}
                  >
                    {opt.label}
                  </div>
                );
              })}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
