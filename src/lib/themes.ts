export interface Theme {
  id: string;
  name: string;
  vars: Record<string, string>;
}

export const THEMES: Theme[] = [
  {
    id: "graphite",
    name: "Graphite",
    vars: {
      bg: "#121212", panel: "#181818", raise: "#222222", line: "#2c2c2c",
      fg: "#ece9e2", mute: "#8b8780",
      accent: "#f2a93b", "accent-fg": "#1c1304",
      rec: "#e5484d", "rec-fg": "#ffffff",
    },
  },
  {
    id: "mono",
    name: "Mono",
    vars: {
      bg: "#0b0b0c", panel: "#111112", raise: "#1a1a1b", line: "#262627",
      fg: "#f2f2f2", mute: "#8a8a8d",
      accent: "#f2f2f2", "accent-fg": "#0b0b0c",
      rec: "#ff4a4a", "rec-fg": "#ffffff",
    },
  },
];

export const DEFAULT_THEME = THEMES[0].id;

export function applyTheme(id: string) {
  const t = THEMES.find((x) => x.id === id) ?? THEMES[0];
  const root = document.documentElement;
  for (const [k, v] of Object.entries(t.vars)) root.style.setProperty(`--${k}`, v);
  root.dataset.theme = t.id;
}
