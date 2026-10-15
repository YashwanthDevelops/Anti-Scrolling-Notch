import type { IslandViewName } from "./layout";

export type IslandTextEntry = "chat" | "github-path" | null;

/** Which focused text controls may temporarily activate the nonactivating island. */
export function isIslandTextEntryFocused(
  view: IslandViewName,
  entry: IslandTextEntry,
  windowFocused: boolean,
): boolean {
  return windowFocused && (
    (view === "prompt" && entry === "chat") ||
    (view === "history" && entry === "github-path")
  );
}

/** Escape in the repository selector ends typing without closing the island. */
export function escapeBlursIslandTextEntry(
  key: string,
  view: IslandViewName,
  entry: IslandTextEntry,
): boolean {
  return key === "Escape" && view === "history" && entry === "github-path";
}
