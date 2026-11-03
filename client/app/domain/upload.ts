import type { Send } from "./api";

// A form sent to the service through the browser's own request, which tells how many of its bytes have gone:
// a video is sent whole, with no ceiling on its size, and the reader is shown how far it has come. A refusal
// is thrown as the fetcher throws one, its status and its body, so the api sorts both alike.

export function browserSend(address: (path: string) => string): Send {
  return <T>(path: string, form: FormData, progress: (sent: number, total: number) => void) =>
    new Promise<T>((resolve, reject) => {
      const request = new XMLHttpRequest();

      request.open("POST", address(path));
      request.responseType = "text";
      request.upload.onprogress = (event) => progress(event.loaded, event.lengthComputable ? event.total : 0);
      request.onload = () => {
        let data: unknown = null;

        try {
          data = request.responseText ? JSON.parse(request.responseText) : null;
        } catch {
          data = null;
        }

        if (request.status >= 200 && request.status < 300) resolve(data as T);
        else reject({ status: request.status, data });
      };
      request.onerror = () => reject(new Error("the upload did not reach the service"));
      request.onabort = () => reject(new Error("the upload was stopped"));
      request.send(form);
    });
}
