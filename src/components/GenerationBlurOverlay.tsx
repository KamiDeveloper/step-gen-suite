import React from "react";

interface GenerationBlurOverlayProps {
  /** Variant of the overlay to apply predefined styles, e.g. "gameplay" */
  variant?: "default" | "gameplay";
  /** Whether to show the Gemini-like animated AI color glow at the bottom (default: true) */
  showGlow?: boolean;
}

export const GenerationBlurOverlay: React.FC<GenerationBlurOverlayProps> = ({
  variant = "default",
  showGlow = true,
}) => {
  const containerClass = variant === "gameplay"
    ? "generation-blur-container generation-blur-container--gameplay"
    : "generation-blur-container";

  return (
    <div className={containerClass}>
      <div className="generation-blur-backdrop" />
      {showGlow && <div className="generation-blur-glow" />}
    </div>
  );
};
