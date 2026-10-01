/** Presentation-only motion preference shared by the island and Mochi renderer. */
let reduced = false;

export const Motion = {
  get reducedMotion() {
    return reduced;
  },

  setReducedMotion(enabled: boolean) {
    reduced = enabled;
    if (typeof document !== "undefined") {
      document.documentElement.classList.toggle("reduce-motion", enabled);
    }
  },
};
