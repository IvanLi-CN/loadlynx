import type { HTMLAttributes } from "react";
import { cn } from "../../lib/utils.ts";

export type AlertVariant = "default" | "info" | "success" | "warning" | "error";

export type AlertProps = HTMLAttributes<HTMLDivElement> & {
  variant?: AlertVariant;
};

const variantClass: Record<AlertVariant, string> = {
  default: "ll-alert",
  info: "ll-alert ll-alert-info",
  success: "ll-alert ll-alert-success",
  warning: "ll-alert ll-alert-warning",
  error: "ll-alert ll-alert-error",
};

export function Alert({
  variant = "default",
  className,
  ...props
}: AlertProps) {
  return <div className={cn(variantClass[variant], className)} {...props} />;
}
