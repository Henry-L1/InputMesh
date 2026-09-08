import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("InputMesh 前端", () => {
  it("在浏览器演示模式中完成权限授权、服务切换和设备配对", async () => {
    render(<App />);

    expect(await screen.findByText("一套键鼠，穿梭所有屏幕")).toBeInTheDocument();
    expect(screen.getByText("可交互演示")).toBeInTheDocument();

    const permissionBanner = screen.getByRole("alert");
    expect(within(permissionBanner).getByText("还差一项系统权限")).toBeInTheDocument();
    fireEvent.click(within(permissionBanner).getByRole("button", { name: "打开系统设置" }));
    await waitFor(() => expect(screen.queryByText("还差一项系统权限")).not.toBeInTheDocument());

    const sharingSwitch = screen.getByRole("switch", { name: "键鼠共享服务" });
    expect(sharingSwitch).not.toBeChecked();
    fireEvent.click(sharingSwitch);
    await waitFor(() => expect(sharingSwitch).toBeChecked());
    expect(screen.getByText("共享运行中")).toBeInTheDocument();
    fireEvent.click(sharingSwitch);
    await waitFor(() => expect(sharingSwitch).not.toBeChecked());
    expect(screen.getByText("共享已暂停")).toBeInTheDocument();

    fireEvent.click(screen.getAllByRole("button", { name: /设备/ })[0]);
    expect(await screen.findByText("当前配对码")).toBeInTheDocument();
    expect(await screen.findByText("确认两端均显示")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "确认一致" }));
    await waitFor(() => {
      expect(screen.getAllByText("已连接").length).toBeGreaterThanOrEqual(2);
    });
  });
});
