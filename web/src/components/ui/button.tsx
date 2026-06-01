import { Slot } from "@radix-ui/react-slot";
import type { ButtonHTMLAttributes } from "react";
import { cn } from "../../lib/utils.ts";

export type ButtonVariant =
  | "default"
  | "primary"
  | "secondary"
  | "outline"
  | "ghost"
  | "neutral"
  | "danger";

export type ButtonSize = "default" | "sm" | "xs" | "square";

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  asChild?: boolean;
  variant?: ButtonVariant;
  size?: ButtonSize;
};

const variantClass: Record<ButtonVariant, string> = {
  default: "ll-button",
  primary: "ll-button ll-button-primary",
  secondary: "ll-button ll-button-secondary",
  outline: "ll-button ll-button-outline",
  ghost: "ll-button ll-button-ghost",
  neutral: "ll-button ll-button-neutral",
  danger: "ll-button ll-button-danger",
};

const sizeClass: Record<ButtonSize, string> = {
  default: "",
  sm: "ll-button-sm",
  xs: "ll-button-xs",
  square: "ll-button-square",
};

export function Button({
  asChild = false,
  variant = "default",
  size = "default",
  className,
  ...props
}: ButtonProps) {
  const Comp = asChild ? Slot : "button";
  return (
    <Comp
      className={cn(variantClass[variant], sizeClass[size], className)}
      {...props}
    />
  );
}
