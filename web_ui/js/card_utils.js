export function normalizeCode(code) {
  if (!code) return '';
  return code.replace(/＋/g, '+').replace(/－/g, '-').replace(/ー/g, '-').trim().toUpperCase();
}

export function extractCardId(title) {
  const parts = title.split(/\s*:\s*/);
  return normalizeCode(parts[0]);
}
