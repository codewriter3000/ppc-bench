/**
 * Hex / number formatting helpers shared across panels.
 */
export const hex32 = (n: number): string =>
  "0x" + (n >>> 0).toString(16).padStart(8, "0").toUpperCase();

export const hex16 = (n: number): string =>
  "0x" + (n & 0xffff).toString(16).padStart(4, "0").toUpperCase();

export const hex8 = (n: number): string =>
  (n & 0xff).toString(16).padStart(2, "0").toUpperCase();

export const formatGPR = (n: number): string => "r" + n.toString().padStart(2, "0");
export const formatFPR = (n: number): string => "f" + n.toString().padStart(2, "0");

export const asciiOf = (b: number): string => {
  const c = b & 0xff;
  return c >= 0x20 && c < 0x7f ? String.fromCharCode(c) : ".";
};
