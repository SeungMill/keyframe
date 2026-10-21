import type { Macro } from "./types";

export interface Card {
  key: string | null;
  title: string;
  macros: Macro[];
}

export function buildCards(macros: Macro[], folders: string[]): Card[] {
  const extra = [...new Set(macros.map((m) => m.folder).filter((f): f is string => !!f))].filter((f) => !folders.includes(f));
  const folderCards = [...folders, ...extra]
    .filter((f) => macros.some((m) => m.folder === f))
    .map((f) => ({ key: f as string | null, title: f, macros: macros.filter((m) => m.folder === f) }));
  if (folderCards.length === 0) return [{ key: null, title: "Macros", macros }];
  const loose = macros.filter((m) => !m.folder).map((m) => ({ key: `macro:${m.id}` as string | null, title: m.name, macros: [m] }));
  return [...folderCards, ...loose];
}
