import type { IconifyIcon } from "@iconify/types";

import chartLine from "@iconify-icons/mdi-light/chart-line";
import chevronDoubleLeft from "@iconify-icons/mdi-light/chevron-double-left";
import chevronDoubleRight from "@iconify-icons/mdi-light/chevron-double-right";
import cog from "@iconify-icons/mdi-light/cog";
import flash from "@iconify-icons/mdi-light/flash";
import menu from "@iconify-icons/mdi-light/menu";
import monitor from "@iconify-icons/mdi-light/monitor";
import pencil from "@iconify-icons/mdi-light/pencil";

export const NAV_ICON_DEVICES: IconifyIcon = monitor;
export const NAV_ICON_CC: IconifyIcon = flash;
export const NAV_ICON_STATUS: IconifyIcon = chartLine;
export const NAV_ICON_SETTINGS: IconifyIcon = cog;
export const NAV_ICON_CALIBRATION: IconifyIcon = pencil;
export const NAV_ICON_MENU: IconifyIcon = menu;
export const NAV_ICON_EXPAND: IconifyIcon = chevronDoubleRight;
export const NAV_ICON_COLLAPSE: IconifyIcon = chevronDoubleLeft;

export const NAV_ICONS = {
  devices: NAV_ICON_DEVICES,
  cc: NAV_ICON_CC,
  status: NAV_ICON_STATUS,
  settings: NAV_ICON_SETTINGS,
  calibration: NAV_ICON_CALIBRATION,
  menu: NAV_ICON_MENU,
  expand: NAV_ICON_EXPAND,
  collapse: NAV_ICON_COLLAPSE,
} as const satisfies Record<string, IconifyIcon>;
