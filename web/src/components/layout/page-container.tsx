import type { ReactNode } from "react";

type PageContainerVariant = "default" | "full";

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
  const baseClassName =
    variant === "full" ? "max-w-none w-full" : "max-w-5xl mx-auto w-full";
  const mergedClassName = [baseClassName, className].filter(Boolean).join(" ");

  return <div className={mergedClassName}>{children}</div>;
}
