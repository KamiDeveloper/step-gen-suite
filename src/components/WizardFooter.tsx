import React from "react";
import { ArrowLeft } from "lucide-react";

interface WizardFooterProps {
  onBack: () => void;
  backLabel: string;
  onNext: () => void;
  nextLabel: string;
  isNextDisabled?: boolean;
  isSubmitting?: boolean;
  nextIcon?: React.ReactNode;
}

export const WizardFooter: React.FC<WizardFooterProps> = ({
  onBack,
  backLabel,
  onNext,
  nextLabel,
  isNextDisabled = false,
  isSubmitting = false,
  nextIcon,
}) => {
  return (
    <div className="wizard-floating-footer">
      <button
        className="btn-ghost-pill"
        onClick={onBack}
        disabled={isSubmitting}
      >
        <ArrowLeft className="icon-mr" size={16} />
        {backLabel}
      </button>

      <button
        className="btn-primary-pill"
        onClick={onNext}
        disabled={isNextDisabled || isSubmitting}
      >
        {isSubmitting ? "Processing..." : nextLabel}
        {nextIcon}
      </button>
    </div>
  );
};
