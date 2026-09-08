import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ScreenInfo } from "../types";
import { ScreenTopology } from "./ScreenTopology";

const sampleScreen: ScreenInfo = {
  id: "screen-1",
  nativeId: "DISPLAY1",
  ownerDeviceId: "local",
  ownerName: "测试电脑",
  name: "主显示器",
  width: 1920,
  height: 1080,
  scaleFactor: 1,
  x: 0,
  y: 0,
  primary: true,
  enabled: true,
  online: true,
};

describe("ScreenTopology", () => {
  it("支持键盘微调屏幕位置", () => {
    const onMove = vi.fn();
    render(
      <ScreenTopology
        screens={[sampleScreen]}
        localDeviceId="local"
        activeScreenId="screen-1"
        onMove={onMove}
        onEnabledChange={vi.fn()}
      />,
    );

    fireEvent.keyDown(screen.getByRole("group"), { key: "ArrowRight", shiftKey: true });
    expect(onMove).toHaveBeenCalledWith("screen-1", 100, 0);
  });

  it("可以单独停用一块屏幕", () => {
    const onEnabledChange = vi.fn();
    const secondScreen = { ...sampleScreen, id: "screen-2", name: "副显示器", x: 1920 };
    render(
      <ScreenTopology
        screens={[sampleScreen, secondScreen]}
        localDeviceId="local"
        activeScreenId="screen-1"
        onMove={vi.fn()}
        onEnabledChange={onEnabledChange}
      />,
    );

    fireEvent.click(screen.getByRole("switch", { name: "主显示器 输入共享" }));
    expect(onEnabledChange).toHaveBeenCalledWith("screen-1", false);
  });
});
