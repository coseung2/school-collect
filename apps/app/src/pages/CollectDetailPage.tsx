import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Card,
  Checkbox,
  EmptyState,
  FormField,
  ListRow,
  ListSurface,
  LoadingState,
  PermissionState,
  Status,
} from "@school-collect/ui";
import {
  closeCollect,
  exportCollectCsv,
  fetchCollect,
  fetchCollectStatus,
  listAttachments,
  listMembers,
  publishCollect,
  updateCollectAssignments,
  updateCollectItems,
  type Attachment,
  type CollectDetail,
  type CollectItemInput,
  type CollectStatus,
  type Member,
} from "../api";
import { exportFileName, saveExportFile } from "../files";
import { AttachmentList } from "./AttachmentList";
import {
  canManage,
  formatDue,
  labelOf,
  messageOf,
  RequestFailure,
  roleOf,
  toneOf,
  useOnline,
} from "../helpers";
import { navigate } from "../routing";

export function CollectDetailPage({
  collectId,
  role,
  tenantId,
  token,
}: {
  collectId: string;
  role: string;
  tenantId: string;
  token: string;
}) {
  const online = useOnline();
  const manage = canManage(role);
  const [detail, setDetail] = useState<CollectDetail | null>(null);
  const [status, setStatus] = useState<CollectStatus | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [statusError, setStatusError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [itemDrafts, setItemDrafts] = useState<CollectItemInput[] | null>(null);
  const [members, setMembers] = useState<Member[] | null>(null);
  const [targetDrafts, setTargetDrafts] = useState<string[] | null>(null);
  const [editError, setEditError] = useState<string | null>(null);
  const [editNotice, setEditNotice] = useState<string | null>(null);
  const [attachments, setAttachments] = useState<Attachment[]>([]);

  // Review aid only: the status list still renders when files cannot load.
  const loadAttachments = useCallback(async () => {
    try {
      setAttachments((await listAttachments(token, tenantId, collectId)).attachments);
    } catch {
      setAttachments([]);
    }
  }, [collectId, tenantId, token]);

  const load = useCallback(async () => {
    try {
      const nextDetail = await fetchCollect(token, tenantId, collectId);
      setDetail(nextDetail);
      setError(null);
      try {
        setStatus(await fetchCollectStatus(token, tenantId, collectId));
        setStatusError(null);
      } catch (caught) {
        setStatus(null);
        setStatusError(caught);
      }
    } catch (caught) {
      setError(caught);
    }
  }, [collectId, tenantId, token]);

  useEffect(() => {
    void load();
    void loadAttachments();
  }, [load, loadAttachments]);

  if (!manage) {
    return (
      <div className="app-page">
        <PermissionState
          action={
            <Button
              onClick={() => navigate({ page: "assignments" })}
              variant="secondary"
            >
              내 제출로 이동
            </Button>
          }
          description="수합 상세의 배포·마감·현황은 관리자와 담당자 화면입니다. 최종 권한은 서버가 판단합니다."
          title="이 화면을 볼 수 없습니다"
        />
      </div>
    );
  }

  if (error && !detail) {
    return (
      <div className="app-page">
        <RequestFailure
          action={
            <Button
              onClick={() => navigate({ page: "collects" })}
              variant="secondary"
            >
              목록으로
            </Button>
          }
          error={error}
          online={online}
          title="수합을 열지 못했습니다"
        />
      </div>
    );
  }

  if (!detail) {
    return (
      <div className="app-page">
        <LoadingState description="수합 상세를 불러오고 있습니다." title="불러오는 중" />
      </div>
    );
  }

  const canPublish = detail.status === "draft";
  const canClose = detail.status === "published";
  const editable = detail.status !== "closed";
  const hasItems = detail.items.length > 0;
  const submitted = (status?.rows ?? []).filter(
    (row) => row.submissionStatus === "submitted",
  );
  const outstanding = (status?.rows ?? []).filter(
    (row) => row.assignmentStatus && row.submissionStatus !== "submitted",
  );

  async function run(action: () => Promise<unknown>, done: string) {
    setBusy(true);
    setNotice(null);
    try {
      await action();
      await load();
      setNotice(done);
    } catch (caught) {
      setError(caught);
      setNotice(null);
    } finally {
      setBusy(false);
    }
  }

  /** 항목 편집은 현재 목록을 초안으로 복사해 시작합니다. */
  function startItemEdit() {
    setItemDrafts(
      detail
        ? detail.items
            .slice()
            .sort((left, right) => left.position - right.position)
            .map((item) => ({
              key: item.key,
              label: item.label,
              required: item.required,
            }))
        : [],
    );
    setEditError(null);
    setEditNotice(null);
  }

  async function saveItems() {
    if (!detail || !itemDrafts) {
      return;
    }
    const keys = itemDrafts.map((item) => item.key.trim());
    if (itemDrafts.some((item) => item.label.trim() === "")) {
      setEditError("항목 이름을 입력하세요.");
      return;
    }
    if (keys.some((key) => !/^[a-z][a-z0-9_]*$/.test(key))) {
      setEditError(
        "항목 키는 영문 소문자로 시작하고 소문자·숫자·밑줄만 쓸 수 있습니다(예: note).",
      );
      return;
    }
    if (new Set(keys).size !== keys.length) {
      setEditError("항목 키는 서로 달라야 합니다.");
      return;
    }
    setBusy(true);
    setEditError(null);
    setEditNotice(null);
    try {
      const next = await updateCollectItems(
        token,
        tenantId,
        detail.id,
        detail.version,
        itemDrafts,
      );
      setDetail(next);
      setItemDrafts(null);
      setEditNotice("항목을 저장했습니다.");
      setNotice("항목을 저장했습니다.");
    } catch (caught) {
      setEditError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  /** 대상 편집은 구성원 목록과 현재 배정을 함께 불러와 시작합니다. */
  async function startTargetEdit() {
    setEditError(null);
    setEditNotice(null);
    try {
      const loaded = members ?? (await listMembers(token, tenantId)).members;
      setMembers(loaded);
      setTargetDrafts((status?.rows ?? []).map((row) => row.userId));
    } catch (caught) {
      setEditError(messageOf(caught));
    }
  }

  async function saveTargets() {
    if (!detail || !targetDrafts) {
      return;
    }
    setBusy(true);
    setEditError(null);
    setEditNotice(null);
    try {
      const next = await updateCollectAssignments(
        token,
        tenantId,
        detail.id,
        detail.version,
        targetDrafts,
      );
      setDetail(next);
      setTargetDrafts(null);
      setStatus(await fetchCollectStatus(token, tenantId, detail.id));
      setEditNotice("대상을 저장했습니다.");
      setNotice("대상을 저장했습니다.");
    } catch (caught) {
      setEditError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  /** 결과를 CSV로 받아 다운로드 폴더에 저장합니다. */
  async function exportResults() {
    if (!detail) {
      return;
    }
    setBusy(true);
    setEditError(null);
    setEditNotice(null);
    setNotice(null);
    try {
      const csv = await exportCollectCsv(token, tenantId, detail.id);
      const path = await saveExportFile(
        exportFileName(detail.title, new Date()),
        csv,
      );
      setNotice(`결과를 저장했습니다: ${path}`);
    } catch (caught) {
      setEditError(messageOf(caught));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="app-page">
      <div className="app-detail-head">
        <Button
          onClick={() => navigate({ page: "collects" })}
          size="small"
          variant="quiet"
        >
          ← 목록
        </Button>
        <Status tone={toneOf(detail.status)}>{labelOf(detail.status)}</Status>
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
            <dt>진행</dt>
            <dd>
              {status?.submitted ?? detail.progress.submitted} /{" "}
              {status?.assigned ?? detail.progress.assigned} 제출
            </dd>
          </div>
        </dl>
        {error ? (
          <p className="app-form__error" role="alert">
            {messageOf(error)}
          </p>
        ) : null}
        {notice ? <p className="app-form__notice">{notice}</p> : null}
        {editError ? (
          <p className="app-form__error" role="alert">
            {editError}
          </p>
        ) : null}
        <div className="app-row-actions">
          <Button
            disabled={busy}
            loading={busy}
            onClick={() => void exportResults()}
            size="small"
            variant="secondary"
          >
            결과 CSV 내보내기
          </Button>
        </div>
        {canPublish && !hasItems ? (
          <p className="app-form__error" role="status">
            항목이 없으면 배포할 수 없습니다. 수합을 다시 만들 때 항목을 넣어 주세요.
          </p>
        ) : null}
        {canPublish ? (
          <Button
            disabled={!hasItems || busy}
            loading={busy}
            onClick={() =>
              void run(
                () => publishCollect(token, tenantId, detail.id),
                "수합을 배포했습니다.",
              )
            }
          >
            배포
          </Button>
        ) : null}
        {canClose ? (
          <Button
            disabled={busy}
            loading={busy}
            onClick={() =>
              void run(
                () => closeCollect(token, tenantId, detail.id),
                "수합을 마감했습니다.",
              )
            }
          >
            마감
          </Button>
        ) : null}
      </Card>

      <section className="app-section">
        <div className="app-section-heading">
          <h2>항목</h2>
          {editable && !itemDrafts ? (
            <Button onClick={startItemEdit} size="small" variant="secondary">
              항목 편집
            </Button>
          ) : null}
        </div>
        {detail.items.length === 0 ? (
          <EmptyState
            description="이 수합에는 아직 항목이 없습니다."
            title="항목이 없습니다"
          />
        ) : (
          <ListSurface>
            {detail.items
              .slice()
              .sort((left, right) => left.position - right.position)
              .map((item) => (
                <ListRow
                  key={item.key}
                  meta={item.required ? <Status tone="warning">필수</Status> : undefined}
                  title={item.label}
                />
              ))}
          </ListSurface>
        )}
        {itemDrafts ? (
          <Card>
            <div className="app-section-heading">
              <h3>항목 편집</h3>
              <p className="app-card-description">
                이름과 필수 여부를 바꾸고 항목을 더할 수 있습니다. 답변이 있는 항목은 서버가
                삭제를 거부합니다.
              </p>
            </div>
            <ul className="app-item-editor">
              {itemDrafts.map((item, index) => (
                <li className="app-item-editor__row" key={`${item.key}-${index}`}>
                  <FormField htmlFor={`edit-item-label-${index}`} label={`항목 ${index + 1}`}>
                    <input
                      id={`edit-item-label-${index}`}
                      onChange={(event) =>
                        setItemDrafts((current) =>
                          current
                            ? current.map((entry, position) =>
                                position === index
                                  ? { ...entry, label: event.target.value }
                                  : entry,
                              )
                            : current,
                        )
                      }
                      value={item.label}
                    />
                  </FormField>
                  <FormField
                    hint={
                      detail.items.some((existing) => existing.key === item.key)
                        ? "기존 항목의 키는 바꿀 수 없습니다."
                        : "예: note"
                    }
                    htmlFor={`edit-item-key-${index}`}
                    label="키"
                  >
                    <input
                      disabled={detail.items.some((existing) => existing.key === item.key)}
                      id={`edit-item-key-${index}`}
                      onChange={(event) =>
                        setItemDrafts((current) =>
                          current
                            ? current.map((entry, position) =>
                                position === index
                                  ? { ...entry, key: event.target.value }
                                  : entry,
                              )
                            : current,
                        )
                      }
                      value={item.key}
                    />
                  </FormField>
                  <label className="app-check">
                    <input
                      checked={item.required}
                      onChange={(event) =>
                        setItemDrafts((current) =>
                          current
                            ? current.map((entry, position) =>
                                position === index
                                  ? { ...entry, required: event.target.checked }
                                  : entry,
                              )
                            : current,
                        )
                      }
                      type="checkbox"
                    />
                    필수
                  </label>
                  <Button
                    onClick={() =>
                      setItemDrafts((current) =>
                        current ? current.filter((_, position) => position !== index) : current,
                      )
                    }
                    size="small"
                    type="button"
                    variant="quiet"
                  >
                    삭제
                  </Button>
                </li>
              ))}
            </ul>
            {editError ? (
              <p className="app-form__error" role="alert">
                {editError}
              </p>
            ) : null}
            {editNotice ? <p className="app-form__notice">{editNotice}</p> : null}
            <div className="app-row-actions">
              <Button
                onClick={() =>
                  setItemDrafts((current) => [
                    ...(current ?? []),
                    { key: "", label: "", required: false },
                  ])
                }
                size="small"
                type="button"
                variant="secondary"
              >
                항목 추가
              </Button>
              <Button disabled={busy} loading={busy} onClick={() => void saveItems()} size="small">
                항목 저장
              </Button>
              <Button
                onClick={() => {
                  setItemDrafts(null);
                  setEditError(null);
                  setEditNotice(null);
                }}
                size="small"
                type="button"
                variant="quiet"
              >
                취소
              </Button>
            </div>
          </Card>
        ) : null}
      </section>

      <section className="app-section">
        <div className="app-section-heading">
          <h2>제출 현황</h2>
          <div className="app-row-actions">
            {editable && !targetDrafts ? (
              <Button onClick={() => void startTargetEdit()} size="small" variant="secondary">
                대상 편집
              </Button>
            ) : null}
            <Status tone="info">
              {status?.submitted ?? 0}/{status?.assigned ?? 0}
            </Status>
          </div>
        </div>
        {targetDrafts ? (
          <Card>
            <div className="app-section-heading">
              <h3>대상 편집</h3>
              <p className="app-card-description">
                제출할 구성원을 고릅니다. 이미 작성한 구성원은 서버가 제외를 거부하고, viewer는
                대상이 될 수 없습니다.
              </p>
            </div>
            {members === null ? (
              <LoadingState description="구성원을 불러오고 있습니다." title="불러오는 중" />
            ) : members.length === 0 ? (
              <EmptyState description="이 학교에는 아직 구성원이 없습니다." title="구성원 없음" />
            ) : (
              <ul className="app-check-list">
                {members.map((member) => {
                  const selectable = member.role !== "viewer";
                  return (
                    <li key={member.userId}>
                      <Checkbox
                        checked={targetDrafts.includes(member.userId)}
                        disabled={!selectable}
                        label={member.displayName ?? member.userId}
                        meta={
                          selectable ? roleOf(member.role) : `${roleOf(member.role)} · 대상 아님`
                        }
                        onChange={(event) =>
                          setTargetDrafts((current) => {
                            if (!current) {
                              return current;
                            }
                            return event.target.checked
                              ? [...current, member.userId]
                              : current.filter((id) => id !== member.userId);
                          })
                        }
                      />
                    </li>
                  );
                })}
              </ul>
            )}
            {editError ? (
              <p className="app-form__error" role="alert">
                {editError}
              </p>
            ) : null}
            {editNotice ? <p className="app-form__notice">{editNotice}</p> : null}
            <div className="app-row-actions">
              <Button
                disabled={busy}
                loading={busy}
                onClick={() => void saveTargets()}
                size="small"
              >
                대상 저장
              </Button>
              <Button
                onClick={() => {
                  setTargetDrafts(null);
                  setEditError(null);
                  setEditNotice(null);
                }}
                size="small"
                type="button"
                variant="quiet"
              >
                취소
              </Button>
            </div>
          </Card>
        ) : null}
        {!status ? (
          statusError ? (
            <RequestFailure
              action={
                <Button onClick={() => void load()} variant="secondary">
                  다시 시도
                </Button>
              }
              error={statusError}
              online={online}
              title="제출 현황을 불러오지 못했습니다"
            />
          ) : (
            <LoadingState description="제출 명단을 불러오고 있습니다." title="불러오는 중" />
          )
        ) : status.rows.length === 0 ? (
          <EmptyState
            description="배정된 구성원이 없습니다."
            title="현황이 없습니다"
          />
        ) : (
          <>
            <h3 className="app-subheading">제출한 구성원</h3>
            {submitted.length === 0 ? (
              <EmptyState
                description="아직 제출한 구성원이 없습니다."
                title="제출 없음"
              />
            ) : (
              <ListSurface>
                {submitted.map((row) => (
                  <ListRow
                    description={roleOf(row.role)}
                    key={row.userId}
                    meta={
                      row.submittedAt
                        ? new Date(row.submittedAt).toLocaleString("ko-KR")
                        : undefined
                    }
                    status={
                      <Status tone="success">{labelOf(row.submissionStatus)}</Status>
                    }
                    title={row.displayName ?? row.userId}
                  />
                ))}
              </ListSurface>
            )}
            {submitted.some((row) => attachments.some((entry) => entry.userId === row.userId)) ? (
              <>
                <h3 className="app-subheading">제출된 첨부 파일</h3>
                {submitted
                  .filter((row) => attachments.some((entry) => entry.userId === row.userId))
                  .map((row) => (
                    <div className="app-review-files" key={row.userId}>
                      <p className="app-review-files__owner">{row.displayName ?? row.userId}</p>
                      <AttachmentList
                        attachments={attachments.filter((entry) => entry.userId === row.userId)}
                        collectId={collectId}
                        editable={false}
                        itemKey=""
                        itemLabel={row.displayName ?? row.userId}
                        onChanged={loadAttachments}
                        tenantId={tenantId}
                        token={token}
                      />
                    </div>
                  ))}
              </>
            ) : null}
            <h3 className="app-subheading">미제출 구성원</h3>
            {outstanding.length === 0 ? (
              <EmptyState
                description="배정된 구성원이 모두 제출했습니다."
                title="미제출 없음"
              />
            ) : (
              <ListSurface>
                {outstanding.map((row) => (
                  <ListRow
                    description={roleOf(row.role)}
                    key={row.userId}
                    meta={
                      <Status tone={toneOf(row.assignmentStatus)}>
                        {labelOf(row.assignmentStatus)}
                      </Status>
                    }
                    status={
                      <Status tone={toneOf(row.submissionStatus)}>
                        제출 {labelOf(row.submissionStatus)}
                      </Status>
                    }
                    title={row.displayName ?? row.userId}
                  />
                ))}
              </ListSurface>
            )}
          </>
        )}
      </section>
    </div>
  );
}
