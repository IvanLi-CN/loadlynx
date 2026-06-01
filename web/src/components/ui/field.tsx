import { ChevronDown } from "lucide-react";
import type {
  HTMLAttributes,
  InputHTMLAttributes,
  SelectHTMLAttributes,
} from "react";
import { cn } from "../../lib/utils.ts";

export function Field({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("ll-form-control", className)} {...props} />;
}

export function FieldLabel({
  className,
  ...props
}: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn("ll-label-text", className)} {...props} />;
}

export function FieldHint({
  className,
  ...props
}: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn("ll-label-text-alt", className)} {...props} />;
}

export function Input({
  className,
  ...props
}: InputHTMLAttributes<HTMLInputElement>) {
  return <input className={cn("ll-input", className)} {...props} />;
}

export function Select({
  className,
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <span
      className={cn(
        "ll-select-shell",
        props.disabled ? "ll-select-shell-disabled" : "",
        className,
      )}
    >
      <select className="ll-select-control" {...props} />
      <ChevronDown className="ll-select-icon" size={16} aria-hidden="true" />
    </span>
  );
}
