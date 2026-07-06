import { cn } from "../../lib/utils.ts";

export type LoadOutputSwitchProps = {
  checked: boolean;
  disabled?: boolean;
  onCheckedChange: (checked: boolean) => void;
  offLabel: string;
  onLabel: string;
  offHint?: string;
  onHint?: string;
  ariaLabel?: string;
  size?: "sm" | "md" | "lg";
  className?: string;
};

export function LoadOutputSwitch({
  checked,
  disabled = false,
  onCheckedChange,
  offLabel,
  onLabel,
  offHint,
  onHint,
  ariaLabel = "Load output switch",
  size = "md",
  className,
}: LoadOutputSwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      data-state={checked ? "on" : "off"}
      data-size={size}
      className={cn(
        "ll-load-output-switch",
        size === "sm" && "ll-load-output-switch--sm",
        size === "md" && "ll-load-output-switch--md",
        size === "lg" && "ll-load-output-switch--lg",
        className,
      )}
      onClick={() => {
        if (!disabled) {
          onCheckedChange(!checked);
        }
      }}
    >
      <span aria-hidden="true" className="ll-load-output-switch__thumb" />
      <span
        className={cn(
          "ll-load-output-switch__option",
          !checked && "ll-load-output-switch__option--active",
        )}
      >
        <span className="ll-load-output-switch__text">
          <span className="ll-load-output-switch__label">{offLabel}</span>
          {offHint ? (
            <span className="ll-load-output-switch__hint">{offHint}</span>
          ) : null}
        </span>
      </span>
      <span
        className={cn(
          "ll-load-output-switch__option",
          checked && "ll-load-output-switch__option--active",
        )}
      >
        <span className="ll-load-output-switch__text">
          <span className="ll-load-output-switch__label">{onLabel}</span>
          {onHint ? (
            <span className="ll-load-output-switch__hint">{onHint}</span>
          ) : null}
        </span>
      </span>
    </button>
  );
}
