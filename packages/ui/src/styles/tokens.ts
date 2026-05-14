/**
 * Design tokens ported from Monoscape (codewriter3000/monoscape).
 * Single source of truth for colors, typography, spacing, and elevation.
 */

export const COLORS = {
  // Primary palette
  primary: "#005fcc",
  primaryLight: "#dce8ff",
  primaryHover: "#89b3f0",

  // Neutrals
  border: "#c3cad8",
  borderSoft: "#d9dde6",
  text: "#172033",
  textMuted: "#52607a",
  label: "#5a606c",

  // Surfaces
  bg: "#f5f6f8",
  bgHover: "#f0f3f6",
  surface: "#ffffff",
  panel: "#ffffff",

  // Semantics
  warningBg: "#fff5e6",
  warningBorder: "#ffcc80",
  error: "#c41e3a",
  success: "#1f8a3c",

  // PPC-Bench specific
  pcRow: "#dce8ff",
  breakpoint: "#c41e3a",
  changedRegister: "#fff5e6",
  memWrite: "#ffe0b2",
  traceDim: "#52607a",
  gutter: "#eef0f4",
} as const;

export const FONTS = {
  ui: 'Inter, "Segoe UI", system-ui, sans-serif',
  mono: '"Cascadia Mono", "SFMono-Regular", Consolas, "Liberation Mono", monospace',
} as const;

export const SIZE = {
  // Type
  caption: "0.72rem",
  label: "0.75rem",
  body: "0.875rem",
  bodyLg: "1rem",
  heading: "1.25rem",

  // Spacing
  s1: "4px",
  s2: "8px",
  s3: "12px",
  s4: "16px",
  s5: "24px",
  s6: "32px",

  // Radii
  radiusSm: "4px",
  radiusMd: "6px",
  radiusLg: "8px",

  // Chrome
  toolbarH: "44px",
  rowH: "22px",
  controlH: "32px",
} as const;

export const SHADOWS = {
  dropdown: "0 8px 24px rgba(23, 32, 51, 0.12)",
  panel: "0 1px 2px rgba(23, 32, 51, 0.06)",
} as const;

/** Inline-style helpers — Monoscape uses inline CSS strings; we follow the same pattern. */
export const PANEL_STYLE = `
  background: ${COLORS.surface};
  border: 1px solid ${COLORS.border};
  border-radius: ${SIZE.radiusMd};
  box-shadow: ${SHADOWS.panel};
  display: flex;
  flex-direction: column;
  overflow: hidden;
  min-height: 0;
`;

export const PANEL_HEADER_STYLE = `
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: ${SIZE.s2} ${SIZE.s3};
  border-bottom: 1px solid ${COLORS.borderSoft};
  background: ${COLORS.bg};
  font: 600 ${SIZE.label} ${FONTS.ui};
  color: ${COLORS.label};
  text-transform: uppercase;
  letter-spacing: 0.04em;
  min-height: 28px;
  flex-shrink: 0;
`;

export const PANEL_BODY_STYLE = `
  flex: 1 1 auto;
  overflow: auto;
  font: 400 ${SIZE.body} ${FONTS.mono};
  color: ${COLORS.text};
  min-height: 0;
`;

export const BUTTON_STYLE = `
  display: inline-flex;
  align-items: center;
  gap: ${SIZE.s1};
  min-height: ${SIZE.controlH};
  padding: ${SIZE.s1} ${SIZE.s3};
  border: 1px solid ${COLORS.border};
  border-radius: ${SIZE.radiusSm};
  background: ${COLORS.surface};
  color: ${COLORS.text};
  font: 500 ${SIZE.body} ${FONTS.ui};
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
`;

export const BUTTON_PRIMARY_STYLE = `
  ${BUTTON_STYLE}
  background: ${COLORS.primary};
  border-color: ${COLORS.primary};
  color: ${COLORS.surface};
`;
