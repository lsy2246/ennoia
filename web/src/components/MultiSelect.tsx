import { useCallback, useEffect, useMemo, useRef, useState } from "react";

export type MultiSelectOption = {
  value: string;
  label: string;
};

type MultiSelectProps = {
  values: string[];
  options: MultiSelectOption[];
  onChange: (values: string[]) => void;
  placeholder: string;
  className?: string;
};

function toggleValue(values: string[], nextValue: string) {
  return values.includes(nextValue)
    ? values.filter((item) => item !== nextValue)
    : [...values, nextValue];
}

export function MultiSelect({ values, options, onChange, placeholder, className }: MultiSelectProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const close = useCallback(() => {
    setOpen(false);
  }, []);

  useEffect(() => {
    if (!open) {
      return;
    }
    const handler = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        close();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [close, open]);

  const summary = useMemo(() => {
    if (values.length === 0) {
      return placeholder;
    }
    return options
      .filter((item) => values.includes(item.value))
      .map((item) => item.label)
      .join(" / ");
  }, [options, placeholder, values]);

  return (
    <div ref={containerRef} className={`custom-select custom-multi-select ${open ? "custom-select--open" : ""} ${className ?? ""}`}>
      <button
        type="button"
        className="custom-select__trigger"
        onClick={() => setOpen((current) => !current)}
        aria-haspopup="listbox"
        aria-expanded={open}
      >
        <span>{summary}</span>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>
      {open ? (
        <div className="custom-select__menu custom-multi-select__menu" role="listbox" aria-multiselectable="true">
          {options.map((option) => (
            <label key={option.value} className="check-row custom-multi-select__option">
              <input
                type="checkbox"
                checked={values.includes(option.value)}
                onChange={() => onChange(toggleValue(values, option.value))}
              />
              {option.label}
            </label>
          ))}
        </div>
      ) : null}
    </div>
  );
}
