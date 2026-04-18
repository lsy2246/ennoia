import type { ReactNode } from "react";

import type { Thread } from "./api";
import { styles } from "./app.styles";

export function StatCard(props: { label: string; value: number }) {
  return (
    <article className={styles.statCard}>
      <span className={styles.statLabel}>{props.label}</span>
      <strong className={styles.statValue}>{props.value}</strong>
    </article>
  );
}

export function HeroMetric(props: { label: string; value: number }) {
  return (
    <article className={styles.heroMetricCard}>
      <span className={styles.statLabel}>{props.label}</span>
      <strong className={styles.heroMetricValue}>{props.value}</strong>
    </article>
  );
}

export function NavButton(props: {
  active: boolean;
  children: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      className={`${styles.navButton} ${props.active ? styles.navButtonActive : ""}`}
      type="button"
      onClick={props.onClick}
    >
      {props.children}
    </button>
  );
}

export function ThreadButton(props: {
  active: boolean;
  onClick: () => void;
  thread: Thread;
}) {
  return (
    <button
      className={`${styles.threadButton} ${props.active ? styles.threadButtonActive : ""}`}
      type="button"
      onClick={props.onClick}
    >
      <strong className={styles.threadTitle}>{props.thread.title}</strong>
      <span className={styles.threadMeta}>
        {props.thread.owner.id} · {props.thread.participants.length} participants
      </span>
    </button>
  );
}

export function PanelSection(props: {
  eyebrow: string;
  title: string;
  children: ReactNode;
}) {
  return (
    <section className={styles.surfaceCard}>
      <div className={styles.brandBlock}>
        <p className={styles.overline}>{props.eyebrow}</p>
        <h4 className={styles.sectionTitle}>{props.title}</h4>
      </div>
      <div className={styles.list}>{props.children}</div>
    </section>
  );
}

export function ListRow(props: { meta: string; title: string }) {
  return (
    <div className={styles.listRow}>
      <strong className={styles.listTitle}>{props.title}</strong>
      <span className={styles.listMeta}>{props.meta}</span>
    </div>
  );
}

export function EmptyState(props: { text: string }) {
  return <div className={styles.emptyState}>{props.text}</div>;
}
