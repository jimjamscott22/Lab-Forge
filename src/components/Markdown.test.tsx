import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const openExternalLink = vi.fn().mockResolvedValue(undefined);
vi.mock("../api/client", () => ({ api: { openExternalLink: (url: string) => openExternalLink(url) } }));

import { Markdown } from "./Markdown";

beforeEach(() => openExternalLink.mockClear());

describe("Markdown", () => {
  it("renders formatting", () => {
    render(<Markdown source={"# Title\n\nSome **bold** text"} />);
    expect(screen.getByRole("heading", { name: "Title" })).toBeInTheDocument();
    expect(screen.getByText("bold").tagName).toBe("STRONG");
  });

  it("shows raw HTML as text instead of running or rendering it", () => {
    const { container } = render(
      <Markdown source={'<script>window.pwned = true</script>\n\n<img src=x onerror="window.pwned = true">\n\n<b>not bold</b>'} />,
    );
    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("b")).toBeNull();
    expect((window as unknown as { pwned?: boolean }).pwned).toBeUndefined();
  });

  it("does not turn non-https links into anything clickable", () => {
    const { container } = render(
      <Markdown
        source={[
          "[js](javascript:alert(1))",
          "[plain http](http://example.test)",
          "[file](file:///etc/passwd)",
          "[data](data:text/html,hi)",
          "[mail](mailto:a@example.test)",
        ].join("\n\n")}
      />,
    );
    expect(container.querySelector("a")).toBeNull();
    expect(screen.queryAllByRole("button")).toHaveLength(0);
    expect(screen.getByText("js")).toBeInTheDocument();
  });

  it("routes https links through the Rust opener instead of navigating", async () => {
    const { container } = render(<Markdown source="[docs](https://tauri.app/security/)" />);
    expect(container.querySelector("a")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "docs" }));
    expect(openExternalLink).toHaveBeenCalledWith("https://tauri.app/security/");
  });

  it("does not load images", () => {
    const { container } = render(<Markdown source="![diagram of a lab](https://example.test/x.png)" />);
    expect(container.querySelector("img")).toBeNull();
    expect(screen.getByText("diagram of a lab")).toBeInTheDocument();
  });
});
