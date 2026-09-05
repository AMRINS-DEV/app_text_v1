import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { StepUpPrompt } from "./step-up-prompt";

describe("StepUpPrompt", () => {
  it("calls onConfirm with the entered code on submit", async () => {
    const onConfirm = vi.fn().mockResolvedValue(undefined);
    render(<StepUpPrompt onConfirm={onConfirm} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "123456" } });
    fireEvent.click(screen.getByText("Confirm"));

    await waitFor(() => expect(onConfirm).toHaveBeenCalledWith("123456"));
  });

  it("calls onCancel and never onConfirm when Cancel is clicked", () => {
    const onConfirm = vi.fn();
    const onCancel = vi.fn();
    render(<StepUpPrompt onConfirm={onConfirm} onCancel={onCancel} />);

    fireEvent.click(screen.getByText("Cancel"));

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("shows an error message when onConfirm rejects", async () => {
    const onConfirm = vi.fn().mockRejectedValue(new Error("invalid TOTP code"));
    render(<StepUpPrompt onConfirm={onConfirm} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "000000" } });
    fireEvent.click(screen.getByText("Confirm"));

    expect(await screen.findByText("invalid TOTP code")).toBeInTheDocument();
  });

  it("disables the confirm button while the request is in flight", async () => {
    let resolveConfirm: () => void = () => {};
    const onConfirm = vi.fn(() => new Promise<void>((resolve) => (resolveConfirm = resolve)));
    render(<StepUpPrompt onConfirm={onConfirm} onCancel={vi.fn()} />);

    fireEvent.change(screen.getByRole("textbox"), { target: { value: "123456" } });
    fireEvent.click(screen.getByText("Confirm"));

    expect(screen.getByText("Confirm")).toBeDisabled();
    resolveConfirm();
    await waitFor(() => expect(screen.getByText("Confirm")).not.toBeDisabled());
  });
});
