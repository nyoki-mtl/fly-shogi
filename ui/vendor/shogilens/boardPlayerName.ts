import type { KifuMetadata } from "../../lib/ipc/kifu";
import type { ColorData } from "./types";

export function resolveBoardPlayerName(
  color: ColorData,
  metadata: KifuMetadata | null,
  blackPlayerName: string,
  whitePlayerName: string,
): string {
  return color === "black"
    ? (metadata?.blackPlayer ?? blackPlayerName)
    : (metadata?.whitePlayer ?? whitePlayerName);
}
