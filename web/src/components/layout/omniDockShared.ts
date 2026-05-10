export type DockPosition = "left" | "right" | "top" | "bottom" | "floating";

export interface NavItem {
  id: string;
  href: string;
  icon: string;
  label: string;
  hint: string;
  source: "builtin" | "extension";
}

export const ENNOIA_ROUTE_DRAG_MIME = "application/ennoia-route";

let activeDraggedNavItem: NavItem | null = null;

export function getActiveDraggedNavItem() {
  return activeDraggedNavItem;
}

export function setActiveDraggedNavItem(item: NavItem | null) {
  activeDraggedNavItem = item;
}
