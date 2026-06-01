import i18next from "i18next";
import { initReactI18next } from "react-i18next";

export const resources = {
  "zh-CN": {
    translation: {
      app: {
        title: "LoadLynx Web Console",
        subtitle: "网络设备管理与 CC 控制",
      },
      nav: {
        navigation: "导航",
        devices: "设备",
        cc: "CC 控制",
        status: "状态",
        pd: "USB-PD",
        settings: "设置",
        firmware: "Firmware",
        calibration: "校准",
      },
      shell: {
        addDevice: "添加设备",
        currentDevice: "当前设备",
        noDeviceSelected: "未选择设备",
        openNavigation: "打开导航抽屉",
        closeNavigation: "关闭导航抽屉",
        collapseSidebar: "收起侧边栏",
        expandSidebar: "展开侧边栏",
        deviceSwitcher: "设备切换",
        selectDevice: "选择设备...",
        noDevicesAvailable: "无可用设备",
        language: "语言",
      },
      demo: {
        shell: {
          subtitle: "纯前端 mock 数据，无硬件访问，无 devd 连接",
        },
        devicePrefix: "设备",
        devices: {
          subtitle: "用于本地视觉验收的纯前端设备台账。",
          register: "注册设备",
          registerHint: "支持 Hostname 或 IP，demo 不会发起真实网络探测。",
          name: "设备名称",
          baseUrl: "Base URL",
          status: "状态",
          registry: "设备台账",
          devdHint:
            "展示 USB/probe 候选、缓存 selector 与 lease 状态，不会自动连接硬件。",
          addSimulation: "添加模拟设备",
        },
        pd: {
          safe5v:
            "Safe5V 模式：Apply 只保存配置，启用扩展电压前，当前合同保持 5V。",
        },
        settings: {
          subtitle: "设备身份、网络、备份与维护动作的 mock 汇总。",
          identity: "设备身份",
          capabilities: "能力",
          backup: "备份与恢复",
          backupText: "导出 presets、校准、WiFi 与 USB-PD 配置。",
          network: "网络",
          wifi: "WiFi",
          actions: "操作",
        },
        firmware: {
          subtitle: "mock dry-run 固件操作，不访问硬件。",
          warning: "Demo 模式不会打开串口、烧录固件或联系 devd。",
          start: "开始 dry-run",
        },
        calibration: {
          subtitle: "当前设备：source=factory-default, fmt=3, hw=1",
          empty: "暂无草稿点。",
        },
      },
    },
  },
  en: {
    translation: {
      app: {
        title: "LoadLynx Web Console",
        subtitle: "Network device manager & CC control",
      },
      nav: {
        navigation: "Navigation",
        devices: "Devices",
        cc: "CC Control",
        status: "Status",
        pd: "USB-PD",
        settings: "Settings",
        firmware: "Firmware",
        calibration: "Calibration",
      },
      shell: {
        addDevice: "Add device",
        currentDevice: "Current device",
        noDeviceSelected: "No device selected",
        openNavigation: "Open navigation drawer",
        closeNavigation: "Close navigation drawer",
        collapseSidebar: "Collapse sidebar",
        expandSidebar: "Expand sidebar",
        deviceSwitcher: "Device switcher",
        selectDevice: "Select a device...",
        noDevicesAvailable: "No devices available",
        language: "Language",
      },
      demo: {
        shell: {
          subtitle: "Pure frontend mock data, no hardware, no devd",
        },
        devicePrefix: "Device",
        devices: {
          subtitle: "Pure frontend mock registry for local visual review.",
          register: "Register device",
          registerHint:
            "Accepts hostname or IP. The demo never performs a real network probe.",
          name: "Device name",
          baseUrl: "Base URL",
          status: "Status",
          registry: "Device registry",
          devdHint:
            "Shows USB/probe candidates, cached selectors and lease state without auto-connecting to hardware.",
          addSimulation: "Add simulation device",
        },
        pd: {
          safe5v:
            "Safe5V only: Apply saves the profile; active contract stays at 5V until extended voltage is enabled.",
        },
        settings: {
          subtitle:
            "Mock summary of identity, network, backups and maintenance actions.",
          identity: "Device identity",
          capabilities: "Capabilities",
          backup: "Backup & restore",
          backupText: "Export presets, calibration, WiFi and USB-PD settings.",
          network: "Network",
          wifi: "WiFi",
          actions: "Actions",
        },
        firmware: {
          subtitle: "Mock dry-run firmware operation, no hardware access.",
          warning:
            "Demo mode never opens serial ports, flashes firmware or contacts devd.",
          start: "Start dry-run",
        },
        calibration: {
          subtitle: "Device active: source=factory-default, fmt=3, hw=1",
          empty: "No draft points.",
        },
      },
    },
  },
} as const;

void i18next.use(initReactI18next).init({
  resources,
  lng: window.localStorage.getItem("loadlynx.locale") ?? "zh-CN",
  fallbackLng: "zh-CN",
  interpolation: {
    escapeValue: false,
  },
});

export { i18next };
