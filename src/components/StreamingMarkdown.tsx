import { SafeMarkdown } from "./SafeMarkdown";

interface StreamingMarkdownProps {
  markdown: string;
  thinking?: boolean;
  thinkingLabel: string;
}

export function splitStreamingMarkdown(markdown: string): {
  committed: string;
  pending: string;
} {
  const idx = markdown.lastIndexOf("\n\n");
  if (idx === -1) {
    return { committed: "", pending: markdown };
  }
  return {
    committed: markdown.slice(0, idx),
    pending: markdown.slice(idx + 2),
  };
}

export function StreamingMarkdown({
  markdown,
  thinking = false,
  thinkingLabel,
}: StreamingMarkdownProps) {
  const trimmed = markdown.trim();
  if (thinking && !trimmed) {
    return (
      <p className="streaming-thinking" role="status">
        {thinkingLabel}
      </p>
    );
  }

  const { committed, pending } = splitStreamingMarkdown(markdown);

  return (
    <div className="streaming-markdown">
      {committed.trim() ? <SafeMarkdown markdown={committed} /> : null}
      {pending ? (
        <p className="streaming-pending">
          {pending}
          <span className="streaming-caret" aria-hidden="true" />
        </p>
      ) : trimmed ? (
        <span className="streaming-caret" aria-hidden="true" />
      ) : null}
    </div>
  );
}
