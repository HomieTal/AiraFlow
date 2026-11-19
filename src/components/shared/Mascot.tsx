import React from "react";
import AiraFlowMascot from "../icons/AiraFlowMascot";

interface MascotProps {
  /** Height in px. */
  size: number;
  /** Wave once on arrival, or high-five (re-key it to play again). */
  animation?: "wave" | "slap" | null;
}

// The brand mark is square; the wave's pivot sits at its base.
const BOX_ASPECT = 1;

/** Aira Flow's brand mark, optionally animated. */
export const Mascot: React.FC<MascotProps> = ({ size, animation = null }) => (
  <span
    style={{ width: size * BOX_ASPECT, height: size }}
    className={`relative block origin-[50%_90%] rotate-[-8deg] select-none ${animation === "wave" ? "animate-wave" : animation === "slap" ? "animate-slap" : ""}`}
  >
    <AiraFlowMascot className="absolute inset-0 h-full w-full" />
  </span>
);
