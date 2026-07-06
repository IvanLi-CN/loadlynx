import { cn } from "../../lib/utils.ts";
import {
  SliderRadioGroup,
  type SliderRadioGroupWidthMode,
} from "../ui/slider-radio-group.tsx";

type EditableControlMode = "CC" | "CV" | "CP";
type VisibleControlMode = EditableControlMode | "CR";

export type ModeSliderSelectorProps = {
  availableModes: EditableControlMode[];
  activeMode: VisibleControlMode;
  onModeChange: (mode: EditableControlMode) => void;
  size?: "sm" | "md" | "lg";
  widthMode?: SliderRadioGroupWidthMode;
  ariaLabel?: string;
  className?: string;
};

function isEditableControlMode(
  mode: VisibleControlMode,
): mode is EditableControlMode {
  return mode === "CC" || mode === "CV" || mode === "CP";
}

export function ModeSliderSelector({
  availableModes,
  activeMode,
  onModeChange,
  size = "md",
  widthMode = "fit",
  ariaLabel = "Mode selector",
  className,
}: ModeSliderSelectorProps) {
  const allModes: VisibleControlMode[] = ["CC", "CV", "CP", "CR"];

  return (
    <SliderRadioGroup
      value={activeMode}
      onValueChange={(mode) => {
        if (isEditableControlMode(mode)) {
          onModeChange(mode);
        }
      }}
      options={allModes.map((mode) => ({
        value: mode,
        label: mode,
        disabled:
          !isEditableControlMode(mode) || !availableModes.includes(mode),
      }))}
      size={size}
      widthMode={widthMode}
      ariaLabel={ariaLabel}
      className={cn(className)}
    />
  );
}
