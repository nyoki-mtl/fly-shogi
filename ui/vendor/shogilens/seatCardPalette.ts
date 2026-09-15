import type { ColorData } from "./types";

export interface SeatCardPalette {
  activeOverlay: string;
  background: string;
  nameText: string;
  sideBar: string;
}

export function getSeatCardPalette(color: ColorData): SeatCardPalette {
  if (color === "black") {
    return {
      activeOverlay: "rgba(47, 109, 181, 0.12)",
      background: "#f5f1e8",
      nameText: "#2b2116",
      sideBar: "#28231d",
    };
  }

  return {
    activeOverlay: "rgba(47, 109, 181, 0.12)",
    background: "#f5f1e8",
    nameText: "#2b2116",
    sideBar: "#a9aca8",
  };
}
