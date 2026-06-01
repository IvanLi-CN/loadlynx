import type { HTMLAttributes } from "react";
import { cn } from "../../lib/utils.ts";

export type BadgeVariant =
  | "default"
  | "neutral"
  | "ghost"
  | "outline"
  | "success"
  | "warning"
  | "error"
  | "info";

export type BadgeProps = HTMLAttributes<HTMLSpanElement> & {
  variant?: BadgeVariant;
};

const variantClass: Record<BadgeVariant, string> = {
  default: "ll-badge",
  neutral: "ll-badge ll-badge-neutral",
  ghost: "ll-badge ll-badge-ghost",
  outline: "ll-badge ll-badge-outline",
  success: "ll-badge ll-badge-success",
  warning: "ll-badge ll-badge-warning",
  error: "ll-badge ll-badge-error",
  info: "ll-badge ll-badge-info",
};

export function Badge({
  variant = "default",
  className,
  ...props
}: BadgeProps) {
  return <span className={cn(variantClass[variant], className)} {...props} />;
}
