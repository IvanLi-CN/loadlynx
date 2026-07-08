import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "storybook/test";
import { PwaUpdatePromptView } from "../../pwa/pwa-update-prompt-view.tsx";
import { RouteStoryHarness } from "../router/route-story-harness.tsx";

function SettingsRouteStory() {
  return <RouteStoryHarness initialPath="/mock-001/settings" />;
}

function SafetyBlockedSettingsRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-001/settings"
      devices={[
        {
          id: "mock-001",
          name: "Safety Blocked",
          baseUrl: "mock://restore-safety-blocked",
        },
      ]}
    />
  );
}

function WifiConnectionFailedSettingsRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/mock-wifi-failed/settings"
      devices={[
        {
          id: "mock-wifi-failed",
          name: "WiFi Failed",
          baseUrl: "mock://wifi-connect-failed",
        },
      ]}
    />
  );
}

function WifiClearNoopSettingsRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/wifi-clear-noop/settings"
      devices={[
        {
          id: "wifi-clear-noop",
          name: "WiFi Clear No-op",
          baseUrl: "mock://wifi-clear-noop",
        },
      ]}
    />
  );
}

function WifiClearTimeoutSuccessSettingsRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/wifi-clear-timeout-success/settings"
      devices={[
        {
          id: "wifi-clear-timeout-success",
          name: "WiFi Clear Timeout Success",
          baseUrl: "mock://wifi-clear-timeout-success",
        },
      ]}
    />
  );
}

function WifiSetTimeoutSuccessSettingsRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/wifi-set-timeout-success/settings"
      devices={[
        {
          id: "wifi-set-timeout-success",
          name: "WiFi Set Timeout Success",
          baseUrl: "mock://wifi-set-timeout-success",
        },
      ]}
    />
  );
}

function WifiSetEepromErrorSettingsRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/wifi-set-eeprom-error/settings"
      devices={[
        {
          id: "wifi-set-eeprom-error",
          name: "WiFi Set EEPROM Error",
          baseUrl: "mock://wifi-set-eeprom-error",
        },
      ]}
    />
  );
}

function DirectHttpSettingsRouteStory(props: { verifiedLan?: boolean }) {
  return (
    <RouteStoryHarness
      initialPath="/direct-http/settings"
      devices={[
        {
          id: "direct-http",
          name: props.verifiedLan ? "Verified LAN Device" : "WiFi Device",
          baseUrl: "http://192.0.2.55",
          connectionMarks: props.verifiedLan ? ["lan"] : undefined,
        },
      ]}
    />
  );
}

function UnverifiedDevdSettingsRouteStory() {
  return (
    <RouteStoryHarness
      initialPath="/unverified-devd/settings"
      devices={[
        {
          id: "unverified-devd",
          name: "Unverified USB Device",
          baseUrl:
            "http://127.0.0.1:30180/?device_id=digital-2bdf&lease_id=expired",
          connectionMarks: ["usb"],
          devd: {
            baseUrl: "http://127.0.0.1:30180",
            deviceId: "digital-2bdf",
            leaseId: "expired",
          },
        },
      ]}
    />
  );
}

function PwaVersionRefreshSettingsRouteStory() {
  return (
    <>
      <RouteStoryHarness initialPath="/mock-001/settings" />
      <PwaUpdatePromptView
        state="update-ready"
        onClose={() => {}}
        onUpdate={() => {}}
      />
    </>
  );
}

const meta = {
  title: "Routes/Settings",
  component: SettingsRouteStory,
} satisfies Meta<typeof SettingsRouteStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const SoftResetDialog: Story = {
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(() => {
      canvas.getByText("LoadLynx Web Console");
      canvas.getByRole("button", { name: "Soft Reset" });
    });

    await userEvent.click(canvas.getByRole("button", { name: "Soft Reset" }));

    await waitFor(() => {
      canvas.getByRole("dialog");
    });

    const dialog = within(canvas.getByRole("dialog"));
    await userEvent.click(dialog.getByRole("button", { name: "Cancel" }));

    await waitFor(() => {
      if (canvas.queryByRole("dialog")) {
        throw new Error("Expected dialog to close after clicking Cancel");
      }
    });
  },
};

export const WifiAndDiagnostics: Story = {
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByRole("button", { name: "Export Diagnostics" });
      },
      { timeout: 5000 },
    );

    await userEvent.type(canvas.getByPlaceholderText("SSID"), "BenchNet");
    await userEvent.type(canvas.getByPlaceholderText("PSK"), "not-shown");
    await userEvent.click(canvas.getByRole("button", { name: "Save WiFi" }));
    await userEvent.click(
      canvas.getByRole("button", { name: "Export Diagnostics" }),
    );

    await waitFor(() => {
      const diagnostics = canvas.getByText(/schema_version/, {
        selector: "pre",
      });
      if (!diagnostics.textContent?.includes('"ssid": "BenchNet"')) {
        throw new Error("Expected diagnostics export to include saved SSID");
      }
    });
  },
};

export const WiFiConnectionFailed: Story = {
  render: () => <WifiConnectionFailedSettingsRouteStory />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByText("Last error");
        canvas.getByText("connect failed");
        canvas.getByText("WiFi connection failed: connect failed");
      },
      { timeout: 5000 },
    );
  },
};

export const WiFiClearNoopShowsError: Story = {
  render: () => <WifiClearNoopSettingsRouteStory />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByText("user");
        canvas.getByText("192.168.31.216");
      },
      { timeout: 5000 },
    );

    await userEvent.click(
      canvas.getByRole("button", { name: "Clear User WiFi" }),
    );

    await waitFor(() => {
      canvas.getByText(/WiFi clear did not take effect/);
    });
  },
};

export const WiFiClearTimeoutRecoveredByStatus: Story = {
  render: () => <WifiClearTimeoutSuccessSettingsRouteStory />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByText("user");
        canvas.getByText("192.168.31.216");
      },
      { timeout: 5000 },
    );

    await userEvent.click(
      canvas.getByRole("button", { name: "Clear User WiFi" }),
    );

    await waitFor(
      () => {
        if (canvas.getByTestId("wifi-status-source").textContent !== "none") {
          throw new Error("Expected WiFi source to be none");
        }
        if (canvas.getByTestId("wifi-status-state").textContent !== "idle") {
          throw new Error("Expected WiFi state to be idle");
        }
        if (canvas.queryByText(/WiFi update failed/)) {
          throw new Error(
            "Expected successful post-clear status to hide error",
          );
        }
      },
      { timeout: 5000 },
    );
  },
};

export const WiFiSetTimeoutRecoveredByStatus: Story = {
  render: () => <WifiSetTimeoutSuccessSettingsRouteStory />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByPlaceholderText("SSID");
        canvas.getByPlaceholderText("PSK");
      },
      { timeout: 5000 },
    );

    await userEvent.type(canvas.getByPlaceholderText("SSID"), "BenchNet");
    await userEvent.type(canvas.getByPlaceholderText("PSK"), "bench-pass");
    await userEvent.click(canvas.getByRole("button", { name: "Save WiFi" }));

    await waitFor(
      () => {
        if (canvas.getByTestId("wifi-status-ssid").textContent !== "BenchNet") {
          throw new Error("Expected saved WiFi SSID to be visible");
        }
        if (canvas.getByTestId("wifi-status-source").textContent !== "user") {
          throw new Error("Expected WiFi source to be user");
        }
        if (canvas.queryByText(/WiFi 更新失败/)) {
          throw new Error("Expected recovered WiFi save to hide error");
        }
      },
      { timeout: 5000 },
    );
  },
};

export const WiFiSetEepromErrorFriendlyMessage: Story = {
  render: () => <WifiSetEepromErrorSettingsRouteStory />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByPlaceholderText("SSID");
        canvas.getByPlaceholderText("PSK");
      },
      { timeout: 5000 },
    );

    await userEvent.type(canvas.getByPlaceholderText("SSID"), "BenchNet");
    await userEvent.type(canvas.getByPlaceholderText("PSK"), "bench-pass");
    await userEvent.click(canvas.getByRole("button", { name: "Save WiFi" }));

    await waitFor(
      () => {
        canvas.getByText(/设备存储写入失败/);
        if (canvas.queryByText(/UNAVAILABLE/)) {
          throw new Error("Expected EEPROM code to be hidden from user copy");
        }
      },
      { timeout: 5000 },
    );
  },
};

export const WiFiWriteBlockedWithoutManagementLink: Story = {
  render: () => <DirectHttpSettingsRouteStory />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByText(/需要先切换到已验证的 USB\/devd 管理连接/);
        canvas.getByRole("button", { name: "切换连接方式" });
      },
      { timeout: 5000 },
    );

    await userEvent.click(canvas.getByRole("button", { name: "切换连接方式" }));

    const dialog = within(await canvas.findByRole("dialog"));
    await waitFor(() => {
      dialog.getByRole("heading", {
        name: "Bind Connection for WiFi Settings",
      });
      dialog.getByText(/only known through the current WiFi\/HTTP path/);
      dialog.getByRole("link", { name: "Bind connection" });
      dialog.getByRole("button", { name: "Cancel" });
      if (dialog.queryByRole("button", { name: "Continue" })) {
        throw new Error("Expected no continue action without a known link");
      }
      if (dialog.queryByText("WiFi / unknown HTTP")) {
        throw new Error("Expected unavailable direct HTTP option to be hidden");
      }
      if (dialog.queryByText(/Direct HTTP is not marked/)) {
        throw new Error("Expected direct HTTP detail to be hidden");
      }
      if (dialog.queryByText(/No USB\/devd lease/)) {
        throw new Error("Expected redundant unavailable path warning hidden");
      }
    });
  },
};

export const WiFiWriteBlocksVerifiedLanWithoutUsb: Story = {
  render: () => <DirectHttpSettingsRouteStory verifiedLan />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByText(/需要先切换到已验证的 USB\/devd 管理连接/);
        canvas.getByRole("button", { name: "切换连接方式" });
      },
      { timeout: 5000 },
    );

    await userEvent.click(canvas.getByRole("button", { name: "切换连接方式" }));

    const dialog = within(await canvas.findByRole("dialog"));
    await waitFor(() => {
      dialog.getByRole("heading", {
        name: "Bind Connection for WiFi Settings",
      });
      dialog.getByText(/only known through the current WiFi\/HTTP path/);
      dialog.getByRole("link", { name: "Bind connection" });
      dialog.getByRole("button", { name: "Cancel" });
      if (dialog.queryByRole("button", { name: "Continue" })) {
        throw new Error("Expected no continue action for verified LAN writes");
      }
      if (dialog.queryByText("Verified LAN HTTP")) {
        throw new Error("Expected verified LAN to be unavailable for writes");
      }
    });
  },
};

export const WiFiWriteLockedOverlay: Story = {
  render: () => <UnverifiedDevdSettingsRouteStory />,
};

export const PwaVersionRefreshReady: Story = {
  render: () => <PwaVersionRefreshSettingsRouteStory />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByText("LoadLynx Web Console");
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByRole("status");
        canvas.getByText("新版本已缓存");
        canvas.getByRole("button", { name: "升级" });
      },
      { timeout: 5000 },
    );
  },
};

export const WiFiWriteBlockedUntilUsbVerified: Story = {
  render: () => <UnverifiedDevdSettingsRouteStory />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByRole("heading", { name: "WiFi" });
        canvas.getByText(/需要先切换到已验证的 USB\/devd 管理连接/);
        canvas.getByRole("button", { name: "切换连接方式" });
      },
      { timeout: 5000 },
    );

    await userEvent.click(canvas.getByRole("button", { name: "切换连接方式" }));

    const dialog = within(await canvas.findByRole("dialog"));
    await waitFor(() => {
      dialog.getByRole("heading", {
        name: "Switch Connection for WiFi Settings",
      });
      dialog.getByText(/write will not be sent if the switch fails/);
      dialog.getByText("USB / local devd");
      dialog.getByRole("button", { name: "Continue" });
      dialog.getByRole("button", { name: "Cancel" });
    });
  },
};

export const BackupRestorePreview: Story = {
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByText("Backup & Restore");
        canvas.getByRole("button", { name: "Export Backup" });
      },
      { timeout: 5000 },
    );

    const backup = {
      kind: "loadlynx.backup",
      schema_version: 1,
      created_at: "2026-05-31T00:00:00Z",
      sections: {
        presets: {
          active_preset_id: 2,
          presets: [
            {
              preset_id: 1,
              mode: "cc",
              target_i_ma: 1500,
              target_v_mv: 12000,
              target_p_mw: 10000,
              min_v_mv: 0,
              max_i_ma_total: 10000,
              max_p_mw: 150000,
            },
          ],
        },
        settings: {
          wifi: {
            ssid: "BenchNet",
            psk: "storybook-secret",
            source: "user",
          },
          sound: {
            volume: 2,
          },
        },
      },
    };

    const file = new File([JSON.stringify(backup)], "loadlynx-backup.json", {
      type: "application/json",
    });
    await userEvent.upload(canvas.getByLabelText("Import backup file"), file);

    await waitFor(() => {
      canvas.getByText("loadlynx-backup.json");
      canvas.getByText(/Unknown section ignored: settings.sound/);
      canvas.getByRole("button", { name: "Restore Selected" });
    });
  },
};

export const BackupRestoreCompleted: Story = {
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByText("Backup & Restore");
      },
      { timeout: 5000 },
    );

    const backup = {
      kind: "loadlynx.backup",
      schema_version: 1,
      created_at: "2026-05-31T00:00:00Z",
      sections: {
        settings: {
          wifi: {
            ssid: "BenchNet",
            psk: "storybook-secret",
            source: "user",
          },
        },
      },
    };

    const file = new File([JSON.stringify(backup)], "wifi-only.json", {
      type: "application/json",
    });
    await userEvent.upload(canvas.getByLabelText("Import backup file"), file);
    await userEvent.click(
      await canvas.findByRole("button", { name: "Restore Selected" }),
    );

    await waitFor(() => {
      canvas.getByText("WiFi OK");
    });
  },
};

export const BackupRestoreSafetyBlocked: Story = {
  render: () => <SafetyBlockedSettingsRouteStory />,
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);

    await waitFor(
      () => {
        canvas.getByText("Backup & Restore");
      },
      { timeout: 5000 },
    );

    const backup = {
      kind: "loadlynx.backup",
      schema_version: 1,
      created_at: "2026-05-31T00:00:00Z",
      sections: {
        settings: {
          wifi: {
            ssid: "BenchNet",
            psk: "storybook-secret",
            source: "user",
          },
        },
      },
    };

    const file = new File([JSON.stringify(backup)], "wifi-only.json", {
      type: "application/json",
    });
    await userEvent.upload(canvas.getByLabelText("Import backup file"), file);
    await userEvent.click(
      await canvas.findByRole("button", { name: "Restore Selected" }),
    );

    await waitFor(() => {
      canvas.getByText(/Restore safety-blocked/);
    });
  },
};
