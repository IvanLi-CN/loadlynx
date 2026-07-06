import type { ReactNode } from "react";

type PageContainerVariant = "default" | "workspace" | "full";

export type PageContainerProps = {
  variant?: PageContainerVariant;
  className?: string;
  children: ReactNode;
};

export function PageContainer({
  variant = "default",
  className,
  children,
}: PageContainerProps) {
  const outerClassName = "w-full";

  const innerClassName = [
    "mx-auto w-full min-w-0",
    variant === "full"
      ? "max-w-none"
      : variant === "workspace"
        ? "max-w-[var(--ll-page-max-workspace)]"
        : "max-w-[var(--ll-page-max-default)]",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={outerClassName}>
      <div data-ll-page-container={variant} className={innerClassName}>
        {children}
      </div>
    </div>
  );
}
