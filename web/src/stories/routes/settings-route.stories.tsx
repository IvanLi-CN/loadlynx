import type { Meta, StoryObj } from "@storybook/react";
import { waitFor, within } from "@testing-library/dom";
import userEvent from "@testing-library/user-event";
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

const meta = {
  title: "Routes/Settings",
  component: SettingsRouteStory,
} satisfies Meta<typeof SettingsRouteStory>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const SoftResetDialog: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(() => {
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
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(() => {
      canvas.getByRole("heading", { name: "WiFi" });
      canvas.getByRole("button", { name: "Export Diagnostics" });
    });

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

export const BackupRestorePreview: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(() => {
      canvas.getByText("Backup & Restore");
      canvas.getByRole("button", { name: "Export Backup" });
    });

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
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(() => {
      canvas.getByText("Backup & Restore");
    });

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
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);

    await waitFor(() => {
      canvas.getByText("Backup & Restore");
    });

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
