export function normalizeQuote(quote) {
  const text = typeof quote?.text === "string" ? quote.text.trim() : "";
  if (!text) return null;

  const author = typeof quote.author === "string" ? quote.author.trim() : "";
  return { text, author };
}

export function formatQuote(quote, fallback) {
  const normalized = normalizeQuote(quote);
  if (!normalized) return fallback;

  const author = normalized.author ? `\n—— ${normalized.author}` : "";
  return `"${normalized.text}"${author}`;
}
