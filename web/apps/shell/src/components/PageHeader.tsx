import type { ReactNode } from "react";

type PageHeaderProps = {
  title: string;
  description?: string;
  meta?: string[];
  actions?: ReactNode;
};

export function PageHeader({ title, description, meta = [], actions }: PageHeaderProps) {
  return (
    <div className="page-header">
      <div className="page-header__body">
        <h1>{title}</h1>
        {description ? <p className="page-header__description">{description}</p> : null}
        {meta.length > 0 ? (
          <div className="page-header__meta">
            {meta.map((item) => (
              <span key={item} className="tag">
                {item}
              </span>
            ))}
          </div>
        ) : null}
      </div>
      {actions ? <div className="page-header__actions">{actions}</div> : null}
    </div>
  );
}
