import { useId, useRef, useState } from "react";
import { Button, ListRow, ListSurface } from "@school-collect/ui";
import {
  ATTACHMENT_ACCEPT,
  deleteAttachment,
  downloadAttachment,
  uploadAttachment,
  type Attachment,
} from "../api";
import { saveDownloadedFile } from "../files";
import { messageOf } from "../helpers";

/** Files attached to one item. The server enforces every limit shown here. */
export const ATTACHMENTS_PER_ITEM = 5;

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function AttachmentList({
  attachments,
  collectId,
  editable,
  itemKey,
  itemLabel,
  onChanged,
  tenantId,
  token,
}: {
  attachments: Attachment[];
  collectId: string;
  editable: boolean;
  itemKey: string;
  itemLabel: string;
  onChanged: () => Promise<void>;
  tenantId: string;
  token: string;
}) {
  const inputId = useId();
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const stored = attachments.filter((item) => item.status === "stored");
  const full = stored.length >= ATTACHMENTS_PER_ITEM;

  async function run(label: string, action: () => Promise<string | null>) {
    setBusy(label);
    setProblem(null);
    setNotice(null);
    try {
      setNotice(await action());
    } catch (error) {
      setProblem(messageOf(error));
    } finally {
      setBusy(null);
    }
  }

  async function onPick(files: FileList | null) {
    const file = files?.[0];
    if (input.current) input.current.value = "";
    if (!file) return;
    await run("upload", async () => {
      await uploadAttachment(token, tenantId, collectId, itemKey, file);
      await onChanged();
      return `'${file.name}'을(를) 올렸습니다.`;
    });
  }

  return (
    <div className="app-attachments">
      {stored.length > 0 ? (
        <ListSurface aria-label={`${itemLabel} 첨부 파일`}>
          {stored.map((attachment) => (
            <ListRow
              action={
                <div className="app-row-actions">
                  <Button
                    disabled={busy !== null}
                    onClick={() =>
                      void run(`download-${attachment.id}`, async () => {
                        const bytes = await downloadAttachment(token, tenantId, attachment);
                        const path = await saveDownloadedFile(attachment.fileName, bytes);
                        return `다운로드 폴더에 저장했습니다: ${path}`;
                      })
                    }
                    size="small"
                    variant="quiet"
                  >
                    받기
                  </Button>
                  {editable ? (
                    <Button
                      aria-label={`${attachment.fileName} 지우기`}
                      disabled={busy !== null}
                      onClick={() =>
                        void run(`delete-${attachment.id}`, async () => {
                          await deleteAttachment(token, tenantId, attachment.id);
                          await onChanged();
                          return `'${attachment.fileName}'을(를) 지웠습니다.`;
                        })
                      }
                      size="small"
                      variant="quiet"
                    >
                      지우기
                    </Button>
                  ) : null}
                </div>
              }
              key={attachment.id}
              meta={formatSize(attachment.byteSize)}
              title={attachment.fileName}
            />
          ))}
        </ListSurface>
      ) : null}
      {editable ? (
        <div className="app-attachments__add">
          <input
            accept={ATTACHMENT_ACCEPT}
            aria-hidden="true"
            className="app-visually-hidden"
            disabled={busy !== null || full}
            id={inputId}
            onChange={(event) => void onPick(event.target.files)}
            ref={input}
            tabIndex={-1}
            type="file"
          />
          <Button
            aria-label={`${itemLabel}에 파일 첨부`}
            disabled={busy !== null || full}
            loading={busy === "upload"}
            onClick={() => input.current?.click()}
            size="small"
            type="button"
            variant="secondary"
          >
            파일 첨부
          </Button>
          <span className="app-attachments__hint">
            {full
              ? `이 항목에는 파일을 ${ATTACHMENTS_PER_ITEM}개까지 올릴 수 있습니다.`
              : "PDF·한글·오피스·이미지·텍스트·zip, 10MB까지"}
          </span>
        </div>
      ) : null}
      {problem ? (
        <p className="app-form__error" role="alert">
          {problem}
        </p>
      ) : null}
      {notice ? (
        <p className="app-form__notice" role="status">
          {notice}
        </p>
      ) : null}
    </div>
  );
}
