import type { CSSProperties, InputHTMLAttributes, ReactNode } from "react";
import { useEffect, useMemo, useState } from "react";
import { cn } from "../../lib/utils.ts";
import { Input } from "./field.tsx";

export type BlockControlRowProps = {
  label: string;
  htmlFor?: string;
  error?: string | null;
  className?: string;
  controlClassName?: string;
  children: ReactNode;
};

export function BlockControlRow({
  label,
  htmlFor,
  error = null,
  className,
  controlClassName,
  children,
}: BlockControlRowProps) {
  const labelContent = (
    <>
      <span className="ll-block-control-row__label-text">{label}</span>
    </>
  );

  return (
    <div className={cn("ll-block-control-row", className)}>
      {htmlFor ? (
        <label htmlFor={htmlFor} className="ll-block-control-row__label">
          {labelContent}
        </label>
      ) : (
        <div className="ll-block-control-row__label">{labelContent}</div>
      )}

      <div className={cn("ll-block-control-row__control", controlClassName)}>
        {children}
        {error ? (
          <div className="text-[11px] text-red-200/85">{error}</div>
        ) : null}
      </div>
    </div>
  );
}

export type BlockControlInputRowProps = {
  label: string;
  error?: string | null;
  className?: string;
  controlClassName?: string;
  inputClassName?: string;
} & InputHTMLAttributes<HTMLInputElement>;

export function BlockControlInputRow({
  label,
  error = null,
  className,
  controlClassName,
  inputClassName,
  id,
  ...inputProps
}: BlockControlInputRowProps) {
  return (
    <BlockControlRow
      label={label}
      htmlFor={id}
      error={error}
      className={className}
      controlClassName={controlClassName}
    >
      <Input
        id={id}
        className={cn("ll-block-control-row__input", inputClassName)}
        {...inputProps}
      />
    </BlockControlRow>
  );
}

export type BlockControlSliderRowProps = {
  label: string;
  id: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  displayValue?: string;
  inputMode?: InputHTMLAttributes<HTMLInputElement>["inputMode"];
  pattern?: string;
  disabled?: boolean;
  error?: string | null;
  className?: string;
  controlClassName?: string;
  inputClassName?: string;
  sliderClassName?: string;
  onValueChange: (value: number) => void;
  onDisplayValueChange?: (raw: string) => void;
  onDisplayValueCommit?: (raw: string) => void;
};

function clampNumber(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min;
  return Math.min(Math.max(value, min), max);
}

function parseIntegerValue(raw: string, fallback: number) {
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) ? parsed : fallback;
}

export function BlockControlSliderRow({
  label,
  id,
  value,
  min,
  max,
  step = 1,
  displayValue,
  inputMode = "numeric",
  pattern,
  disabled = false,
  error = null,
  className,
  controlClassName,
  inputClassName,
  sliderClassName,
  onValueChange,
  onDisplayValueChange,
  onDisplayValueCommit,
}: BlockControlSliderRowProps) {
  const safeMax = max >= min ? max : min;
  const sliderValue = clampNumber(value, min, safeMax);
  const percent =
    safeMax === min ? 100 : ((sliderValue - min) / (safeMax - min)) * 100;
  const sliderStyle = {
    "--ll-block-control-slider-percent": `${percent}%`,
  } as CSSProperties;
  const fallbackDisplayValue = useMemo(() => String(value), [value]);
  const controlledDisplayValue = displayValue ?? fallbackDisplayValue;
  const [draftDisplayValue, setDraftDisplayValue] = useState(
    controlledDisplayValue,
  );

  useEffect(() => {
    setDraftDisplayValue(controlledDisplayValue);
  }, [controlledDisplayValue]);

  const commitDisplayValue = () => {
    const nextRaw = draftDisplayValue.trim();
    if (onDisplayValueCommit) {
      onDisplayValueCommit(nextRaw);
      return;
    }

    onValueChange(clampNumber(parseIntegerValue(nextRaw, value), min, safeMax));
  };

  return (
    <div className={cn("ll-block-control-slider-row", className)}>
      <label htmlFor={id} className="ll-block-control-slider-row__label">
        <span className="ll-block-control-slider-row__label-text">{label}</span>
      </label>

      <div
        className={cn(
          "ll-block-control-slider-row__controls",
          controlClassName,
        )}
      >
        <input
          type="range"
          aria-label={label}
          className={cn("ll-block-control-slider-row__slider", sliderClassName)}
          min={min}
          max={safeMax}
          step={step}
          value={sliderValue}
          disabled={disabled}
          style={sliderStyle}
          onChange={(event) => onValueChange(Number(event.target.value))}
        />
        <Input
          id={id}
          type="text"
          inputMode={inputMode}
          pattern={pattern}
          aria-label={label}
          className={cn(
            "ll-block-control-row__input ll-block-control-slider-row__input",
            inputClassName,
          )}
          value={draftDisplayValue}
          disabled={disabled}
          onChange={(event) => {
            const nextRaw = event.target.value;
            setDraftDisplayValue(nextRaw);
            onDisplayValueChange?.(nextRaw);
          }}
          onBlur={commitDisplayValue}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              commitDisplayValue();
              event.currentTarget.blur();
            }
          }}
        />
      </div>

      {error ? (
        <div className="text-[11px] text-red-200/85">{error}</div>
      ) : null}
    </div>
  );
}
