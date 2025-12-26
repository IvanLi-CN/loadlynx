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
  const outerClassName = [
    "w-full",
    // NOTE: ConsoleLayout currently pads <main> with p-3 sm:p-4 md:p-6.
    // PageContainer becomes the single source of truth for horizontal padding by
    // neutralizing the parent's horizontal padding and re-applying it here.
    "-mx-3 sm:-mx-4 md:-mx-6",
    "px-3 sm:px-4 md:px-6",
  ].join(" ");

  const innerClassName = [
    "w-full min-w-0",
    variant === "full" ? "max-w-none" : "max-w-7xl",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div className={outerClassName}>
      <div className={innerClassName}>{children}</div>
    </div>
  );
}
