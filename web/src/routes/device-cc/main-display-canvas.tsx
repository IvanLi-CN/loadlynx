import { useEffect, useRef, useState } from "react";
import type { AnalogState, LoadMode } from "../../api/types.ts";
import {
  isSevenSegPixel,
  sevenSegFontCharCount,
  sevenSegFontFirstChar,
  sevenSegFontHeight,
  sevenSegFontWidth,
} from "../../fonts/sevenSegFont.ts";
import {
  isSmallFontPixel,
  smallFontHeight,
  smallFontWidth,
} from "../../fonts/smallFont.ts";

export interface MainDisplayCanvasProps {
  remoteVoltageV: number;
  localVoltageV: number;
  localCurrentA: number;
  remoteCurrentA: number;
  totalCurrentA: number;
  totalPowerW: number;
  controlMode: LoadMode;
  controlTargetMilli: number;
  controlTargetUnit: "A" | "V" | "W" | "Ω";
  uptimeSeconds: number;
  tempCoreC: number | undefined;
  tempSinkC: number | undefined;
  tempMcuC: number | undefined;
  remoteActive: boolean;
  analogState: AnalogState;
  faultFlags: number;
}

export function MainDisplayCanvas({
  remoteVoltageV,
  localVoltageV,
  localCurrentA,
  remoteCurrentA,
  totalCurrentA,
  totalPowerW,
  controlMode,
  controlTargetMilli,
  controlTargetUnit,
  uptimeSeconds,
  tempCoreC,
  tempSinkC,
  tempMcuC,
  remoteActive,
  analogState,
  faultFlags,
}: MainDisplayCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [canvasSize, setCanvasSize] = useState<{
    width: number;
    height: number;
  }>({
    width: 0,
    height: 0,
  });

  // Track the rendered size of the canvas in CSS pixels so that we can
  // render at the corresponding device-pixel resolution.
  useEffect(() => {
    const handleResize = () => {
      const canvas = canvasRef.current;
      if (!canvas) {
        return;
      }
      const rect = canvas.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        setCanvasSize({
          width: rect.width,
          height: rect.height,
        });
      }
    };

    handleResize();
    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || canvasSize.width === 0 || canvasSize.height === 0) {
      return;
    }
    const dpr = window.devicePixelRatio ?? 1;
    const baseWidth = 320;
    const baseHeight = 240;

    const logicalWidth = Math.round(canvasSize.width * dpr);
    const logicalHeight = Math.round(canvasSize.height * dpr);
    if (canvas.width !== logicalWidth || canvas.height !== logicalHeight) {
      canvas.width = logicalWidth;
      canvas.height = logicalHeight;
    }

    const context = canvas.getContext("2d");
    if (!context) {
      return;
    }

    const ctx = context;
    const scaleX = canvas.width / baseWidth;
    const scaleY = canvas.height / baseHeight;
    ctx.setTransform(scaleX, 0, 0, scaleY, 0, 0);
    ctx.imageSmoothingEnabled = false;

    const width = baseWidth;
    const height = baseHeight;

    type Rect = { left: number; top: number; right: number; bottom: number };

    const COLOR_CANVAS = "#05070D";
    const COLOR_LEFT_BASE = "#101829";
    const COLOR_RIGHT_BASE = "#080F19";
    const CARD_TINTS = ["#171F33", "#141D2F", "#111828"] as const;

    const COLOR_CAPTION = "#9AB0D8";
    const COLOR_VOLTAGE = "#FFB347";
    const COLOR_CURRENT = "#FF5252";
    const COLOR_POWER = "#6EF58C";
    const COLOR_RIGHT_LABEL = "#6D7FA4";
    const COLOR_RIGHT_VALUE = "#DFE7FF";
    const COLOR_BAR_TRACK = "#1C2638";
    const COLOR_BAR_FILL = "#4CC9F0";

    const fillRect = (rect: Rect, color: string) => {
      const w = rect.right - rect.left;
      const h = rect.bottom - rect.top;
      if (w <= 0 || h <= 0) {
        return;
      }
      ctx.fillStyle = color;
      ctx.fillRect(rect.left, rect.top, w, h);
    };

    const drawSmallChar = (
      ch: string,
      x0: number,
      y0: number,
      color: string,
    ) => {
      if (!ch) {
        return;
      }
      const code = ch.charCodeAt(0);
      ctx.fillStyle = color;
      for (let y = 0; y < smallFontHeight; y += 1) {
        for (let x = 0; x < smallFontWidth; x += 1) {
          if (isSmallFontPixel(code, x, y)) {
            ctx.fillRect(x0 + x, y0 + y, 1, 1);
          }
        }
      }
    };

    const drawSmallText = (
      text: string,
      x: number,
      y: number,
      color: string,
      spacing: number = 0,
    ) => {
      let cursorX = x;
      for (const ch of text) {
        if (ch === " ") {
          cursorX += smallFontWidth + spacing;
          continue;
        }
        drawSmallChar(ch, cursorX, y, color);
        cursorX += smallFontWidth + spacing;
      }
    };

    const drawSevenSegDigit = (
      code: number,
      x0: number,
      y0: number,
      color: string,
    ) => {
      if (
        code < sevenSegFontFirstChar ||
        code >= sevenSegFontFirstChar + sevenSegFontCharCount
      ) {
        return;
      }
      ctx.fillStyle = color;
      for (let y = 0; y < sevenSegFontHeight; y += 1) {
        for (let x = 0; x < sevenSegFontWidth; x += 1) {
          if (isSevenSegPixel(code, x, y)) {
            ctx.fillRect(x0 + x, y0 + y, 1, 1);
          }
        }
      }
    };

    const drawSevenSegValue = (text: string, area: Rect, color: string) => {
      const spacing = 4;
      let totalWidth = 0;
      for (const ch of text) {
        totalWidth += (ch === "." ? 8 : sevenSegFontWidth) + spacing;
      }
      if (text.length > 0) {
        totalWidth -= spacing;
      }

      let cursorX = area.right - totalWidth;
      for (const ch of text) {
        if (ch === ".") {
          ctx.fillStyle = color;
          ctx.fillRect(cursorX, area.bottom - 10, 6, 6);
          cursorX += 8 + spacing;
          continue;
        }
        drawSevenSegDigit(ch.charCodeAt(0), cursorX, area.top, color);
        cursorX += sevenSegFontWidth + spacing;
      }
    };

    const fillRoundRect = (rect: Rect, radius: number, color: string) => {
      const w = rect.right - rect.left;
      const h = rect.bottom - rect.top;
      if (w <= 0 || h <= 0) {
        return;
      }
      const r = Math.max(0, Math.min(radius, w / 2, h / 2));
      ctx.fillStyle = color;
      if (r === 0) {
        ctx.fillRect(rect.left, rect.top, w, h);
        return;
      }
      ctx.beginPath();
      ctx.moveTo(rect.left + r, rect.top);
      ctx.lineTo(rect.right - r, rect.top);
      ctx.quadraticCurveTo(rect.right, rect.top, rect.right, rect.top + r);
      ctx.lineTo(rect.right, rect.bottom - r);
      ctx.quadraticCurveTo(
        rect.right,
        rect.bottom,
        rect.right - r,
        rect.bottom,
      );
      ctx.lineTo(rect.left + r, rect.bottom);
      ctx.quadraticCurveTo(rect.left, rect.bottom, rect.left, rect.bottom - r);
      ctx.lineTo(rect.left, rect.top + r);
      ctx.quadraticCurveTo(rect.left, rect.top, rect.left + r, rect.top);
      ctx.closePath();
      ctx.fill();
    };

    const clamp01 = (value: number) => Math.max(0, Math.min(1, value));

    const drawMirrorBar = (
      top: number,
      left: number,
      right: number,
      leftRatio: number,
      rightRatio: number,
    ) => {
      const barHeight = 8;
      const center = Math.floor((left + right) / 2);
      fillRect({ left, top, right, bottom: top + barHeight }, COLOR_BAR_TRACK);
      fillRect(
        {
          left: center,
          top: top - 2,
          right: center + 1,
          bottom: top + barHeight + 2,
        },
        COLOR_RIGHT_LABEL,
      );

      const halfWidth = Math.floor((right - left) / 2);
      const leftFill = Math.round(halfWidth * clamp01(leftRatio));
      const rightFill = Math.round(halfWidth * clamp01(rightRatio));
      if (leftFill > 0) {
        fillRect(
          {
            left: center - leftFill,
            top,
            right: center,
            bottom: top + barHeight,
          },
          COLOR_BAR_FILL,
        );
      }
      if (rightFill > 0) {
        fillRect(
          {
            left: center,
            top,
            right: center + rightFill,
            bottom: top + barHeight,
          },
          COLOR_BAR_FILL,
        );
      }
    };

    const formatFixed2dp = (value: number) => {
      if (!Number.isFinite(value)) {
        return "99.99";
      }
      const v = Math.abs(value);
      const scaled = Math.floor(v * 100 + 0.5);
      if (scaled > 9_999) {
        return "99.99";
      }
      const intPart = Math.floor(scaled / 100);
      const fracPart = scaled % 100;
      return `${String(intPart).padStart(2, "0")}.${String(fracPart).padStart(2, "0")}`;
    };

    const formatFixed1dp3i = (value: number) => {
      if (!Number.isFinite(value)) {
        return "999.9";
      }
      const v = Math.abs(value);
      const scaled = Math.floor(v * 10 + 0.5);
      if (scaled > 9_999) {
        return "999.9";
      }
      const intPart = Math.floor(scaled / 10);
      const fracPart = scaled % 10;
      return `${String(intPart).padStart(3, "0")}.${fracPart}`;
    };

    const formatPairValue = (value: number, unit: "V" | "A") =>
      `${formatFixed2dp(value)}${unit}`;

    const formatSetpointMilli = (
      valueMilli: number,
      unit: "V" | "A" | "W" | "Ω",
    ) => {
      if (!Number.isFinite(valueMilli) || valueMilli < 0) {
        return `--.--${unit}`;
      }
      let v = Math.max(0, Math.trunc(valueMilli));
      v = Math.floor((v + 5) / 10) * 10;
      const centi = Math.floor(v / 10);
      if (centi > 9_999) {
        return `--.--${unit}`;
      }
      const intPart = Math.floor(centi / 100);
      const fracPart = centi % 100;
      return `${String(intPart).padStart(2, "0")}.${String(fracPart).padStart(2, "0")}${unit}`;
    };

    const formatRunTime = (secs: number) => {
      const hours = Math.floor(secs / 3_600);
      const minutes = Math.floor((secs % 3_600) / 60);
      const seconds = Math.floor(secs % 60);
      return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
    };

    const formatTemp1dp = (temp: number | undefined) => {
      if (typeof temp !== "number" || !Number.isFinite(temp)) {
        return "--.-";
      }
      return temp.toFixed(1);
    };

    const formatStatusLine5 = () => {
      if (faultFlags !== 0) {
        const hex = (faultFlags >>> 0)
          .toString(16)
          .toUpperCase()
          .padStart(8, "0");
        return `FLT 0x${hex}`;
      }
      switch (analogState) {
        case "ready":
          return "RDY";
        case "cal_missing":
          return "CAL";
        case "faulted":
          return "FLT";
        default:
          return "OFF";
      }
    };

    fillRect({ left: 0, top: 0, right: width, bottom: height }, COLOR_CANVAS);
    fillRect({ left: 0, top: 0, right: 190, bottom: height }, COLOR_LEFT_BASE);
    fillRect(
      { left: 190, top: 0, right: width, bottom: height },
      COLOR_RIGHT_BASE,
    );

    const cardTops = [0, 80, 160] as const;
    for (let idx = 0; idx < cardTops.length; idx += 1) {
      const top = cardTops[idx];
      fillRect(
        { left: 8, top: top + 6, right: 182, bottom: top + 80 },
        CARD_TINTS[idx],
      );
    }

    drawSmallText("VOLTAGE", 16, 10, COLOR_CAPTION);
    drawSevenSegValue(
      formatFixed2dp(remoteVoltageV),
      { left: 24, top: 28, right: 170, bottom: 72 },
      COLOR_VOLTAGE,
    );
    drawSmallText("V", 170, 56, COLOR_CAPTION, 1);

    drawSmallText("CURRENT", 16, 90, COLOR_CAPTION);
    drawMirrorBar(92, 76, 180, localCurrentA / 5, remoteCurrentA / 5);
    drawSevenSegValue(
      formatFixed2dp(totalCurrentA),
      { left: 24, top: 108, right: 170, bottom: 152 },
      COLOR_CURRENT,
    );
    drawSmallText("A", 170, 136, COLOR_CAPTION, 1);

    drawSmallText("POWER", 16, 170, COLOR_CAPTION);
    drawSevenSegValue(
      formatFixed1dp3i(totalPowerW),
      { left: 24, top: 188, right: 170, bottom: 232 },
      COLOR_POWER,
    );
    drawSmallText("W", 170, 216, COLOR_CAPTION, 1);

    fillRoundRect(
      { left: 198, top: 10, right: 252, bottom: 38 },
      6,
      COLOR_BAR_TRACK,
    );
    fillRoundRect(
      { left: 256, top: 10, right: 314, bottom: 38 },
      6,
      COLOR_BAR_TRACK,
    );
    fillRect({ left: 225, top: 12, right: 226, bottom: 36 }, COLOR_RIGHT_BASE);

    if (controlMode === "cp") {
      drawSmallText("CP", 204, 18, COLOR_POWER);
    } else if (controlMode === "cr") {
      drawSmallText("CR", 204, 18, COLOR_RIGHT_VALUE);
    } else {
      const ccColor = controlMode === "cc" ? COLOR_CURRENT : COLOR_RIGHT_LABEL;
      const cvColor = controlMode === "cv" ? COLOR_VOLTAGE : COLOR_RIGHT_LABEL;
      drawSmallText("CC", 204, 18, ccColor);
      drawSmallText("CV", 230, 18, cvColor);
    }

    const targetText = formatSetpointMilli(
      controlTargetMilli,
      controlTargetUnit,
    );
    const valueX = 314 - 4 - targetText.length * smallFontWidth;
    const valueY = 18;
    const selectedIdx = 3;
    const cellX = valueX + selectedIdx * smallFontWidth;
    fillRect(
      { left: cellX - 1, top: valueY, right: cellX + 6, bottom: valueY + 12 },
      COLOR_BAR_FILL,
    );
    drawSmallText(targetText, valueX, valueY, COLOR_RIGHT_VALUE);
    if (selectedIdx >= 0 && selectedIdx < targetText.length) {
      drawSmallChar(targetText[selectedIdx], cellX, valueY, COLOR_RIGHT_BASE);
    }

    drawSmallText("REMOTE", 198, 50, COLOR_RIGHT_LABEL);
    const remoteText = remoteActive
      ? formatPairValue(remoteVoltageV, "V")
      : "--.--";
    drawSmallText(remoteText, 198, 62, COLOR_RIGHT_VALUE);
    drawSmallText("LOCAL", 258, 50, COLOR_RIGHT_LABEL);
    drawSmallText(
      formatPairValue(localVoltageV, "V"),
      258,
      62,
      COLOR_RIGHT_VALUE,
    );

    const remoteBar = remoteActive ? remoteVoltageV / 40 : 0;
    drawMirrorBar(84, 198, 314, remoteBar, localVoltageV / 40);

    const runText = `RUN ${formatRunTime(uptimeSeconds)}`;
    const coreText = `CORE ${formatTemp1dp(tempCoreC)}C`;
    const sinkText = `SINK ${formatTemp1dp(tempSinkC)}C`;
    const mcuText = `MCU  ${formatTemp1dp(tempMcuC)}C`;
    const statusText = formatStatusLine5();
    const statusLines = [
      runText,
      coreText,
      sinkText,
      mcuText,
      statusText,
    ] as const;
    for (let idx = 0; idx < statusLines.length; idx += 1) {
      drawSmallText(statusLines[idx], 198, 172 + idx * 12, COLOR_RIGHT_VALUE);
    }

    fillRect({ left: 288, top: 0, right: 320, bottom: 10 }, COLOR_RIGHT_BASE);
    drawSmallText("W:--", 290, 1, COLOR_RIGHT_LABEL);
  }, [
    analogState,
    canvasSize.height,
    canvasSize.width,
    controlMode,
    controlTargetMilli,
    controlTargetUnit,
    faultFlags,
    localCurrentA,
    localVoltageV,
    remoteActive,
    remoteCurrentA,
    remoteVoltageV,
    tempCoreC,
    tempMcuC,
    tempSinkC,
    totalCurrentA,
    totalPowerW,
    uptimeSeconds,
  ]);

  return (
    <div className="ll-panel w-full max-w-[640px] aspect-[4/3] rounded-2xl bg-[#05070D] shadow-2xl overflow-hidden border border-[#1f2937]">
      <canvas
        ref={canvasRef}
        width={320}
        height={240}
        className="w-full h-full block"
        style={{
          imageRendering: "pixelated",
        }}
      />
    </div>
  );
}
