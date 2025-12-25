import { Icon, type IconProps } from "@iconify/react";
import type { IconifyIcon } from "@iconify/types";

export type AppIconProps = Omit<IconProps, "icon" | "width" | "height"> & {
  icon: IconifyIcon;
  size?: number | string;
};

export function AppIcon({
  icon,
  size = 20,
  className,
  ...props
}: AppIconProps) {
  if (typeof icon === "string") {
    throw new Error("AppIcon requires a local Iconify icon data object.");
  }

  const mergedClassName = ["inline-block", className].filter(Boolean).join(" ");

  return (
    <Icon
      icon={icon}
      width={size}
      height={size}
      className={mergedClassName}
      {...props}
    />
  );
}

