import type { LucideIcon, LucideProps } from "lucide-react";

export type AppIconProps = Omit<LucideProps, "ref"> & {
  icon: LucideIcon;
  size?: number | string;
};

export function AppIcon({
  icon,
  size = 20,
  className,
  ...props
}: AppIconProps) {
  const Icon = icon;
  const mergedClassName = ["inline-block", className].filter(Boolean).join(" ");

  return <Icon size={size} className={mergedClassName} {...props} />;
}
