import {
  Activity,
  ChevronsLeft,
  ChevronsRight,
  Cpu,
  Gauge,
  type LucideIcon,
  Menu,
  MonitorCog,
  PencilRuler,
  RadioTower,
  Settings,
  Zap,
} from "lucide-react";

export const NAV_ICON_DEVICES: LucideIcon = MonitorCog;
export const NAV_ICON_CC: LucideIcon = Zap;
export const NAV_ICON_STATUS: LucideIcon = Activity;
export const NAV_ICON_PD: LucideIcon = RadioTower;
export const NAV_ICON_SETTINGS: LucideIcon = Settings;
export const NAV_ICON_FIRMWARE: LucideIcon = Cpu;
export const NAV_ICON_CALIBRATION: LucideIcon = PencilRuler;
export const NAV_ICON_MENU: LucideIcon = Menu;
export const NAV_ICON_EXPAND: LucideIcon = ChevronsRight;
export const NAV_ICON_COLLAPSE: LucideIcon = ChevronsLeft;

export const NAV_ICONS = {
  devices: NAV_ICON_DEVICES,
  cc: NAV_ICON_CC,
  status: NAV_ICON_STATUS,
  pd: NAV_ICON_PD,
  settings: NAV_ICON_SETTINGS,
  firmware: NAV_ICON_FIRMWARE,
  calibration: NAV_ICON_CALIBRATION,
  gauge: Gauge,
  menu: NAV_ICON_MENU,
  expand: NAV_ICON_EXPAND,
  collapse: NAV_ICON_COLLAPSE,
} as const satisfies Record<string, LucideIcon>;
