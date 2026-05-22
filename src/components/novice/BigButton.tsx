import type { ReactNode } from "react";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

export interface BigButtonProps {
  icon: ReactNode;
  title: string;
  subtitle: string;
  onClick: () => void;
  disabled?: boolean;
  disabledReason?: string;
  testId?: string;
}

export function BigButton({
  icon,
  title,
  subtitle,
  onClick,
  disabled = false,
  disabledReason,
  testId,
}: BigButtonProps) {
  const button = (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      data-testid={testId}
      data-disabled={disabled ? "true" : "false"}
      className={cn(
        "flex flex-col items-center justify-center gap-3 rounded-2xl border border-border-default bg-card text-card-foreground",
        "h-44 w-full px-6 py-6 text-center shadow-sm transition-colors",
        "hover:bg-muted/50 hover:border-border-hover focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        "disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:bg-card disabled:hover:border-border-default",
      )}
    >
      <span className="text-4xl leading-none" aria-hidden="true">
        {icon}
      </span>
      <span className="text-lg font-semibold leading-tight">{title}</span>
      <span className="text-xs text-muted-foreground leading-snug">
        {subtitle}
      </span>
    </button>
  );

  if (!disabled || !disabledReason) {
    return button;
  }

  return (
    <TooltipProvider delayDuration={150}>
      <Tooltip>
        <TooltipTrigger asChild>
          <span
            tabIndex={0}
            aria-disabled
            className="inline-flex w-full"
            data-testid={testId ? `${testId}-wrapper` : undefined}
          >
            {button}
          </span>
        </TooltipTrigger>
        <TooltipContent data-testid={testId ? `${testId}-tooltip` : undefined}>
          {disabledReason}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
