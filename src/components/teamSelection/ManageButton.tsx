import { ChevronRight, Loader2 } from "lucide-react";

import { Button } from "../ui";

interface ManageButtonProps {
  /** What it reads: "Manage <club>", "Choose a club", or the confirming label. */
  label: string;
  /** No club is chosen, or the career is already starting. */
  disabled: boolean;
  /** The career is starting: the arrow gives way to a spinner. */
  confirming: boolean;
  onClick: () => void;
  /** The id of the text that says why it is disabled, while it is. */
  describedBy?: string;
  className?: string;
}

/**
 * The button that starts the career with the chosen club. The header and the chosen-club card each
 * draw one; both are this component with the page's one handler and state, so they cannot drift.
 */
export default function ManageButton({
  label,
  disabled,
  confirming,
  onClick,
  describedBy,
  className,
}: ManageButtonProps) {
  return (
    <Button
      className={className}
      disabled={disabled}
      aria-describedby={describedBy}
      onClick={onClick}
      iconRight={
        confirming ? (
          <Loader2 aria-hidden="true" className="motion-safe:animate-spin" />
        ) : (
          <ChevronRight aria-hidden="true" />
        )
      }
    >
      {label}
    </Button>
  );
}
