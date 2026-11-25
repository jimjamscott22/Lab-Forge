import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { attemptView } from "../test/fixtures";
import { PacketLedger } from "./PacketLedger";

describe("PacketLedger", () => {
  const evidence = attemptView().challenge.evidence[0]!;

  it("lists every packet with its time in seconds", () => {
    render(<PacketLedger evidence={evidence} />);
    const table = screen.getByRole("table");
    expect(within(table).getAllByRole("row")).toHaveLength(1 + 4 + 2); // header, packets, two gaps
    expect(within(table).getByText("6.312")).toBeInTheDocument();
    expect(within(table).getByRole("columnheader", { name: "Source" })).toBeInTheDocument();
  });

  it("calls out long pauses as their own rows and ignores short ones", () => {
    render(<PacketLedger evidence={evidence} />);
    const gaps = screen.getAllByTestId("gap-row").map((r) => r.textContent);
    expect(gaps).toEqual([
      "+4.298 s passed between packet 2 and packet 3",
      "+2.002 s passed between packet 3 and packet 4",
    ]);
  });

  it("makes the scrollable table reachable by keyboard", () => {
    render(<PacketLedger evidence={evidence} />);
    expect(screen.getByRole("region", { name: /packet table/i })).toHaveAttribute("tabindex", "0");
  });
});
