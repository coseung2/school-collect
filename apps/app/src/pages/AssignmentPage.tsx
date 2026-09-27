import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Button,
  Card,
  EmptyState,
  FormField,
  LoadingState,
  Status,
} from "@school-collect/ui";
import {
  ApiError,
  fetchCollect,
  listAttachments,
  saveDraft,
  sendSubmission,
  type Attachment,
  type CollectDetail,
} from "../api";
import { AttachmentList } from "./AttachmentList";
import {
  deleteLocalDraft,
  loadLocalDraft,
  saveLocalDraft,
} from "../drafts";
import {
  canSubmit,
  formatDue,
  labelOf,
  messageOf,
  payloadText,
  RequestFailure,
  toneOf,
  useOnline,
} from "../helpers";
import { navigate } from "../routing";

export function AssignmentPage({
  collectId,
  role,
  tenantId,
  token,
  userId,
}: {
  collectId: string;
  role: string;
  tenantId: string;
  token: string;
  userId: string;
}) {
  const online = useOnline();
  const [detail, setDetail] = useState<CollectDetail | null>(null);
  const [values, setValues] = useState<Record<string, string>>({});
  const [error, setError] = useState<unknown>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [conflict, setConflict] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [pendingLocal, setPendingLocal] = useState(false);
  const [attachments, setAttachments] = useState<Attachment[]>([]);
  const [attachmentError, setAttachmentError] = useState<string | null>(null);
  const [busyItemKeys, setBusyItemKeys] = useState<string[]>([]);

  // Attachments are optional to the page: if they cannot load (offline, or
  // storage not configured), the text answer still works.
  const loadAttachments = useCallback(async () => {
    try {
      const value = await listAttachments(token, tenantId, collectId, "mine");
      setAttachments(value.attachments);
      setAttachmentError(null);
    } catch (caught) {
      setAttachmentError(messageOf(caught));
    }
  }, [collectId, tenantId, token]);

  // An attachment being sent is page state, not just that row's: the submit
  // button waits for it, so a file cannot be left half-written by handing the
  // answer in too early.
  const onAttachmentBusy = useCallback((key: string, busy: boolean) => {
    setBusyItemKeys((current) => {
      const others = current.filter((entry) => entry !== key);
      return busy ? [...others, key] : others;
    });
  }, []);

  const load = useCallback(async () => {
    try {
      const value = await fetchCollect(token, tenantId, collectId);
      const nextValues: Record<string, string> = {};
      for (const item of value.items) {
        nextValues[item.key] = payloadText(value.submission?.payload, item.key);
      }
      // 재시작 복구: 서버로 보내지 못한 로컬 초안이 있으면 그 내용을 보여 줍니다.
      const local = await loadLocalDraft(tenantId, userId, collectId);
      if (local) {
        for (const item of value.items) {
          const saved = local.payload?.[item.key];
          if (typeof saved === "string") {
            nextValues[item.key] = saved;
          }
        }
      }
      setDetail(value);
      setValues(nextValues);
      setPendingLocal(Boolean(local));
      setError(null);
    } catch (caught) {
      setError(caught);
    }
  }, [collectId, tenantId, token, userId]);

  useEffect(() => {
    void load();
    void loadAttachments();
  }, [load, loadAttachments]);

  const items = useMemo(
    () =>
      (detail?.items ?? [])
        .slice()
        .sort((left, right) => left.position - right.position),
    [detail],
  );

  if (error && !detail) {
    return (
      <div className="app-page">
        <RequestFailure
          action={
            <Button
              onClick={() => navigate({ page: "assignments" })}
              variant="secondary"
            >
              목록으로
            </Button>
          }
          error={error}
          online={online}
          title="제출 화면을 열지 못했습니다"
        />
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="app-page">
        <LoadingState description="제출할 항목을 불러오고 있습니다." title="불러오는 중" />
      </div>
    );
  }

  const submitted = detail.submission?.status === "submitted";
  const closed = detail.status === "closed";
  const open = detail.status === "published";
  const editable = open && !submitted && canSubmit(role);
  const attachmentBusy = busyItemKeys.length > 0;
  const unfinishedAttachments = attachments.filter(
    (entry) => entry.status === "pending" || entry.status === "uploading",
  ).length;
  const expectedVersion = detail.submission?.version ?? 0;
  const collectIdForSave = detail.id;

  function missingRequired(): string[] {
    return items
      .filter((item) => item.required && values[item.key]?.trim() === "")
      .map((item) => item.label);
  }

  async function persist(next: () => Promise<unknown>, done: string) {
    setBusy(true);
    setNotice(null);
    setConflict(null);
    setError(null);
    try {
      await next();
      await load();
      setNotice(done);
    } catch (caught) {
      if (caught instanceof ApiError && caught.code === "version_conflict") {
        const version =
          caught.currentVersion === null ? "최신" : String(caught.currentVersion);
        setConflict(
          `다른 곳에서 내용이 바뀌었습니다. ${version} 버전을 불러왔습니다. 확인한 뒤 다시 저장하세요.`,
        );
        await load();
        return;
      }
      setError(caught);
    } finally {
      setBusy(false);
    }
  }

  async function onSave() {
    // 로컬에 먼저 남기고 서버로 보냅니다. 서버 저장이 실패해도(오프라인·충돌)
    // 입력한 내용은 이 컴퓨터에 남습니다.
    await persist(async () => {
      await saveLocalDraft({
        tenantId,
        userId,
        collectId: collectIdForSave,
        payload: values,
        baseVersion: expectedVersion,
      });
      setPendingLocal(true);
      await saveDraft(token, tenantId, collectIdForSave, expectedVersion, values);
      await deleteLocalDraft(tenantId, userId, collectIdForSave);
      setPendingLocal(false);
    }, `초안을 저장했습니다. 버전 ${expectedVersion + 1}을 사용합니다.`);
  }

  async function onSubmit() {
    const missing = missingRequired();
    if (missing.length > 0) {
      setError(new Error(`필수 항목을 입력하세요: ${missing.join(", ")}`));
      return;
    }
    await persist(async () => {
      // 로컬에만 있는 초안은 먼저 서버로 보낸 뒤 제출합니다.
      await saveLocalDraft({
        tenantId,
        userId,
        collectId: collectIdForSave,
        payload: values,
        baseVersion: expectedVersion,
      });
      await saveDraft(token, tenantId, collectIdForSave, expectedVersion, values);
      await sendSubmission(token, tenantId, collectIdForSave);
      await deleteLocalDraft(tenantId, userId, collectIdForSave);
      setPendingLocal(false);
    }, "제출했습니다. 이제 내용을 바꿀 수 없습니다.");
  }

  return (
    <div className="app-page">
      <div className="app-detail-head">
        <Button
          onClick={() => navigate({ page: "assignments" })}
          size="small"
          variant="quiet"
        >
          ← 목록
        </Button>
        <Status tone={toneOf(detail.submission?.status ?? detail.status)}>
          {submitted ? "제출 완료" : labelOf(detail.status)}
        </Status>
      </div>

      <Card>
        <h2>{detail.title}</h2>
        {detail.description ? (
          <p className="app-card-description">{detail.description}</p>
        ) : null}
        <dl className="app-meta">
          <div>
            <dt>마감</dt>
            <dd>{formatDue(detail.dueAt) ?? "없음"}</dd>
          </div>
          <div>
            <dt>저장 버전</dt>
            <dd>{expectedVersion}</dd>
          </div>
        </dl>
        {!editable ? (
          <p className="app-card-description">
            {submitted
              ? "제출이 완료되어 더 이상 수정할 수 없습니다."
              : closed
                ? "마감된 수합은 읽기만 가능합니다."
                : canSubmit(role)
                  ? "배포된 뒤에만 작성할 수 있습니다."
                  : "열람 권한으로는 제출할 수 없습니다. 최종 권한은 서버가 판단합니다."}
          </p>
        ) : null}
        {conflict ? (
          <p className="app-form__error" role="alert">
            {conflict}
          </p>
        ) : null}
        {pendingLocal ? (
          <p className="app-form__notice" role="status">
            이 컴퓨터에 저장된 초안을 불러왔습니다. 서버로 보내려면 임시 저장이나 제출을
            누르세요.
          </p>
        ) : null}
        {error ? (
          <p className="app-form__error" role="alert">
            {messageOf(error)}
          </p>
        ) : null}
        {notice ? <p className="app-form__notice">{notice}</p> : null}
      </Card>

      {items.length === 0 ? (
        <EmptyState
          description="이 수합에 입력할 항목이 없습니다."
          title="항목이 없습니다"
        />
      ) : (
        <form
          className="app-form"
          onSubmit={(event) => {
            event.preventDefault();
            void onSubmit();
          }}
        >
          {items.map((item) => (
            <FormField
              htmlFor={`assignment-${item.key}`}
              key={item.key}
              label={item.label}
              required={item.required}
            >
              <textarea
                disabled={!editable}
                id={`assignment-${item.key}`}
                onChange={(event) =>
                  setValues((current) => ({
                    ...current,
                    [item.key]: event.target.value,
                  }))
                }
                rows={4}
                value={values[item.key] ?? ""}
              />
              {attachmentError === null ? (
                <AttachmentList
                  attachments={attachments.filter((entry) => entry.itemKey === item.key)}
                  collectId={collectIdForSave}
                  editable={editable}
                  itemKey={item.key}
                  itemLabel={item.label}
                  onBusyChange={onAttachmentBusy}
                  onChanged={loadAttachments}
                  tenantId={tenantId}
                  token={token}
                />
              ) : null}
            </FormField>
          ))}
          {attachmentError ? (
            <p className="app-form__notice" role="status">
              첨부 파일을 불러오지 못했습니다: {attachmentError}
            </p>
          ) : null}
          <div className="app-form__actions">
            {unfinishedAttachments > 0 ? (
              <p className="app-form__error" role="alert">
                올리기가 끝나지 않은 첨부 파일이 {unfinishedAttachments}개 있습니다.
                지우거나 다시 올린 뒤 제출할 수 있습니다.
              </p>
            ) : null}
            <Button
              disabled={!editable || busy}
              onClick={() => void onSave()}
              type="button"
              variant="secondary"
            >
              임시 저장
            </Button>
            <Button
              disabled={
                !editable || busy || attachmentBusy || unfinishedAttachments > 0
              }
              loading={busy}
              type="submit"
            >
              제출
            </Button>
          </div>
        </form>
      )}
    </div>
  );
}
