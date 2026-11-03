import type { Title } from "../../shared/types";

// What the screens' forms send: a file with the text parts the service reads beside it, and how far a sending
// has come.

/** A multipart body: the part `file` under the file's own name, and each text part given that is not blank. */
export function fileForm(file: File, parts: Record<string, string | number | null | undefined>): FormData {
  const form = new FormData();

  form.append("file", file, file.name);

  for (const [name, value] of Object.entries(parts)) {
    if (value === null || value === undefined) continue;

    const text = String(value).trim();

    if (text) form.append(name, text);
  }

  return form;
}

/** A file's name without its extension, as the service names an episode where no name is given. */
export function bareName(fileName: string): string {
  const dot = fileName.lastIndexOf(".");

  return dot > 0 ? fileName.slice(0, dot) : fileName;
}

/** A size in bytes in megabytes, to one place. */
export function megabytes(bytes: number, locale?: string): string {
  return (bytes / 1_000_000).toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
}

/** How far a sending has come: `12.3/450.0 MB 2%`, the share floored so a hundred stands only once all is sent. */
export function sentText(sent: number, total: number, locale?: string): string {
  const share = total > 0 ? Math.floor((Math.min(sent, total) / total) * 100) : 0;

  return `${megabytes(sent, locale)}/${megabytes(total, locale)} MB ${share}%`;
}

/** Whether a name is one the service takes: not blank and on one line. */
export function nameOk(name: string): boolean {
  return name.trim() !== "" && !/[\r\n]/.test(name);
}

/** The titles a video may be filed under: the realm's, by name. */
export function fileTargets(titles: Title[]): Title[] {
  return [...titles].sort((a, b) => a.name.localeCompare(b.name));
}
