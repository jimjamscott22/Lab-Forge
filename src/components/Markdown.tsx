import ReactMarkdown from "react-markdown";
import type { Components } from "react-markdown";
import { api } from "../api/client";

/**
 * Safe Markdown rendering.
 *
 * - react-markdown never renders raw HTML (no rehype-raw): tags show up as text.
 * - Links never navigate the WebView. https links become buttons that ask Rust to open
 *   them in the system browser (Rust re-validates the scheme); anything else is inert text.
 * - Images are not rendered; the alt text is shown instead.
 */
const components: Components = {
  a({ href, children }) {
    if (typeof href === "string" && /^https:\/\//i.test(href)) {
      return (
        <button
          type="button"
          className="link"
          title={href}
          onClick={() => {
            void api.openExternalLink(href).catch(() => undefined);
          }}
        >
          {children}
        </button>
      );
    }
    return <span className="link-inert">{children}</span>;
  },
  img({ alt }) {
    return <span>{alt ?? ""}</span>;
  },
};

// Defaults would pass through http, mailto, etc.; we handle every link ourselves above.
const urlTransform = (url: string) => url;

export function Markdown({ source, className = "" }: { source: string; className?: string }) {
  return (
    <div className={`prose-lab ${className}`}>
      <ReactMarkdown components={components} urlTransform={urlTransform}>
        {source}
      </ReactMarkdown>
    </div>
  );
}
