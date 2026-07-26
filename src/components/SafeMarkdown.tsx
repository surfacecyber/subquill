import ReactMarkdown from "react-markdown";

const ALLOWED_ELEMENTS = [
  "h1",
  "h2",
  "h3",
  "h4",
  "h5",
  "h6",
  "p",
  "ul",
  "ol",
  "li",
  "strong",
  "em",
  "code",
  "pre",
  "blockquote",
] as const;

interface SafeMarkdownProps {
  markdown: string;
}

export function SafeMarkdown({ markdown }: SafeMarkdownProps) {
  return (
    <ReactMarkdown
      allowedElements={[...ALLOWED_ELEMENTS]}
      unwrapDisallowed
      skipHtml
      urlTransform={() => ""}
    >
      {markdown}
    </ReactMarkdown>
  );
}
