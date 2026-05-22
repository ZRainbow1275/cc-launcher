import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import {
  DangerousConfirm,
  type DangerousConfirmRuleInfo,
} from "@/components/sandbox/DangerousConfirm";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts ? `${key}|${JSON.stringify(opts)}` : key,
  }),
}));

const ruleInfo: DangerousConfirmRuleInfo = {
  ruleId: "L1.sudo_runas",
  titleKey: "sandbox.l1.sudo_runas.title",
  descriptionKey: "sandbox.l1.sudo_runas.desc",
  riskKeys: [
    "sandbox.l1.sudo_runas.risk1",
    "sandbox.l1.sudo_runas.risk2",
    "sandbox.l1.sudo_runas.risk3",
  ],
};

describe("DangerousConfirm", () => {
  it("walks through step 1 -> step 2 -> step 3 and submits with the keyword", () => {
    const onConfirm = vi.fn();
    const onClose = vi.fn();
    render(
      <DangerousConfirm
        isOpen
        rule={ruleInfo}
        locale="en"
        onConfirm={onConfirm}
        onClose={onClose}
      />,
    );

    expect(screen.getByTestId("dangerous-confirm-step-1")).toBeTruthy();

    fireEvent.click(screen.getByTestId("dangerous-confirm-step1-continue"));
    expect(screen.getByTestId("dangerous-confirm-step-2")).toBeTruthy();

    const nextBtn = screen.getByTestId("dangerous-confirm-step2-next");
    expect(nextBtn.hasAttribute("disabled")).toBe(true);

    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-1"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-2"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-3"));
    expect(nextBtn.hasAttribute("disabled")).toBe(false);

    fireEvent.click(nextBtn);
    expect(screen.getByTestId("dangerous-confirm-step-3")).toBeTruthy();
    expect(
      screen.getByTestId("dangerous-confirm-keyword-display").textContent,
    ).toBe("I UNDERSTAND");

    const submit = screen.getByTestId("dangerous-confirm-submit");
    expect(submit.hasAttribute("disabled")).toBe(true);

    const input = screen.getByTestId("dangerous-confirm-keyword-input");
    fireEvent.change(input, { target: { value: "I UNDERSTAND" } });
    expect(submit.hasAttribute("disabled")).toBe(false);

    fireEvent.click(submit);
    expect(onConfirm).toHaveBeenCalledWith("I UNDERSTAND");
  });

  it("rejects a wrong keyword in step 3", () => {
    const onConfirm = vi.fn();
    render(
      <DangerousConfirm
        isOpen
        rule={ruleInfo}
        locale="zh"
        onConfirm={onConfirm}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("dangerous-confirm-step1-continue"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-1"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-2"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-3"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-step2-next"));

    expect(
      screen.getByTestId("dangerous-confirm-keyword-display").textContent,
    ).toBe("我已知晓");

    const input = screen.getByTestId("dangerous-confirm-keyword-input");
    fireEvent.change(input, { target: { value: "wrong keyword" } });

    const submit = screen.getByTestId("dangerous-confirm-submit");
    expect(submit.hasAttribute("disabled")).toBe(true);

    fireEvent.click(submit);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("enforces case-sensitive keyword match (lowercase rejected)", () => {
    const onConfirm = vi.fn();
    render(
      <DangerousConfirm
        isOpen
        rule={ruleInfo}
        locale="en"
        onConfirm={onConfirm}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByTestId("dangerous-confirm-step1-continue"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-1"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-2"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-3"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-step2-next"));

    const input = screen.getByTestId("dangerous-confirm-keyword-input");
    fireEvent.change(input, { target: { value: "i understand" } });
    expect(
      screen.getByTestId("dangerous-confirm-submit").hasAttribute("disabled"),
    ).toBe(true);
    fireEvent.click(screen.getByTestId("dangerous-confirm-submit"));
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("prevents proceeding to step 3 if only 2 of 3 checkboxes are ticked", () => {
    render(
      <DangerousConfirm
        isOpen
        rule={ruleInfo}
        locale="en"
        onConfirm={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("dangerous-confirm-step1-continue"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-1"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-2"));

    const next = screen.getByTestId("dangerous-confirm-step2-next");
    expect(next.hasAttribute("disabled")).toBe(true);
    fireEvent.click(next);

    expect(screen.queryByTestId("dangerous-confirm-step-3")).toBeNull();
    expect(screen.getByTestId("dangerous-confirm-step-2")).toBeTruthy();
  });

  it("uses the Japanese keyword when locale=ja", () => {
    render(
      <DangerousConfirm
        isOpen
        rule={ruleInfo}
        locale="ja"
        onConfirm={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("dangerous-confirm-step1-continue"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-1"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-2"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-3"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-step2-next"));

    expect(
      screen.getByTestId("dangerous-confirm-keyword-display").textContent,
    ).toBe("理解しました");
  });
});
