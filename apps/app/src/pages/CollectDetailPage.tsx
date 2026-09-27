import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Card,
  EmptyState,
  ListRow,
  ListSurface,
  LoadingState,
  PermissionState,
  Status,
} from "@school-collect/ui";
import {
  closeCollect,
  fetchCollect,
  fetchCollectStatus,
  publishCollect,
  type CollectDetail,
  type CollectStatus,
} from "../api";
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
  }, [load]);

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
      </section>

      <section className="app-section">
        <div className="app-section-heading">
          <h2>제출 현황</h2>
          <Status tone="info">
            {status?.submitted ?? 0}/{status?.assigned ?? 0}
          </Status>
        </div>
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
