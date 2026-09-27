import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Button,
  Card,
  EmptyState,
  ListRow,
  ListSurface,
  LoadingState,
  Status,
} from "@school-collect/ui";
import {
  listAssignments,
  listCollects,
  type Assignment,
  type Collect,
} from "../api";
import {
  canManage,
  compareDue,
  formatDue,
  isDueSoon,
  isUrgentDue,
  labelOf,
  RequestFailure,
  toneOf,
  useOnline,
} from "../helpers";
import { navigate } from "../routing";

export function OverviewPage({
  role,
  tenantId,
  token,
}: {
  role: string;
  tenantId: string;
  token: string;
}) {
  const online = useOnline();
  const manage = canManage(role);
  const [assignments, setAssignments] = useState<Assignment[] | null>(null);
  const [collects, setCollects] = useState<Collect[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const load = useCallback(async () => {
    try {
      const assigned = await listAssignments(token, tenantId);
      const managed = manage ? await listCollects(token, tenantId) : { collects: [] };
      setAssignments(assigned.assignments);
      setCollects(managed.collects);
      setError(null);
    } catch (caught) {
      setError(caught);
    }
  }, [manage, tenantId, token]);

  useEffect(() => {
    void load();
  }, [load]);

  const pending = useMemo(
    () =>
      (assignments ?? []).filter((item) => item.submissionStatus !== "submitted"),
    [assignments],
  );
  const activeManaged = useMemo(
    () => (collects ?? []).filter((item) => item.status === "published"),
    [collects],
  );
  const dueSoon = useMemo(() => {
    const fromAssignments = pending.filter((item) =>
      isDueSoon(item.dueAt, item.status),
    );
    const fromCollects = manage
      ? activeManaged.filter((item) => isDueSoon(item.dueAt, item.status))
      : [];
    const seen = new Set<string>();
    return [...fromAssignments, ...fromCollects]
      .sort((left, right) => compareDue(left.dueAt, right.dueAt))
      .filter((item) => {
        const id = "collectId" in item ? item.collectId : item.id;
        if (seen.has(id)) {
          return false;
        }
        seen.add(id);
        return true;
      });
  }, [activeManaged, manage, pending]);

  if (error) {
    return (
      <div className="app-page">
        <RequestFailure
          action={
            <Button onClick={() => void load()} variant="secondary">
              다시 시도
            </Button>
          }
          error={error}
          online={online}
          title="홈을 불러오지 못했습니다"
        />
      </div>
    );
  }

  if (assignments === null || collects === null) {
    return (
      <div className="app-page">
        <LoadingState
          description="제출할 수합과 진행 중인 수합을 모으고 있습니다."
          title="불러오는 중"
        />
      </div>
    );
  }

  return (
    <div className="app-page">
      <section className="app-stats-grid">
        <Card>
          <p className="app-card-eyebrow">내 제출</p>
          <p className="app-stat">{pending.length}</p>
          <p className="app-card-description">아직 제출하지 않은 수합</p>
        </Card>
        {manage ? (
          <Card>
            <p className="app-card-eyebrow">자료수합</p>
            <p className="app-stat">{activeManaged.length}</p>
            <p className="app-card-description">지금 진행 중인 수합</p>
          </Card>
        ) : null}
        <Card variant={dueSoon.length > 0 ? "attention" : "default"}>
          <p className="app-card-eyebrow">마감 임박</p>
          <p className="app-stat">{dueSoon.length}</p>
          <p className="app-card-description">7일 안이거나 지난 마감</p>
        </Card>
      </section>

      <section className="app-section">
        <div className="app-section-heading">
          <h2>내가 제출할 수합</h2>
          <Button
            onClick={() => navigate({ page: "assignments" })}
            size="small"
            variant="quiet"
          >
            전체 보기
          </Button>
        </div>
        {pending.length === 0 ? (
          <EmptyState
            description="배정된 수합을 모두 제출했거나, 아직 배정된 수합이 없습니다."
            title="제출할 수합이 없습니다"
          />
        ) : (
          <ListSurface>
            {pending.map((item) => (
              <ListRow
                action={
                  <Button
                    onClick={() =>
                      navigate({ page: "assignment", id: item.collectId })
                    }
                    size="small"
                    variant="secondary"
                  >
                    작성
                  </Button>
                }
                description={formatDue(item.dueAt)}
                key={item.collectId}
                meta={
                  <Status tone={toneOf(item.assignmentStatus)}>
                    {labelOf(item.assignmentStatus)}
                  </Status>
                }
                status={
                  <Status tone={toneOf(item.submissionStatus)}>
                    제출 {labelOf(item.submissionStatus)}
                  </Status>
                }
                title={item.title}
                urgent={isUrgentDue(item.dueAt, item.status)}
              />
            ))}
          </ListSurface>
        )}
      </section>

      {manage ? (
        <section className="app-section">
          <div className="app-section-heading">
            <h2>관리 중인 수합</h2>
            <Button
              onClick={() => navigate({ page: "collects" })}
              size="small"
              variant="quiet"
            >
              전체 보기
            </Button>
          </div>
          {activeManaged.length === 0 ? (
            <EmptyState
              description="진행 중인 수합이 없습니다. 자료수합에서 초안을 배포하면 여기에 표시됩니다."
              title="진행 중인 수합이 없습니다"
            />
          ) : (
            <ListSurface>
              {activeManaged.map((item) => (
                <ListRow
                  action={
                    <Button
                      onClick={() => navigate({ page: "collect", id: item.id })}
                      size="small"
                      variant="secondary"
                    >
                      열기
                    </Button>
                  }
                  description={formatDue(item.dueAt)}
                  key={item.id}
                  status={
                    <Status tone={toneOf(item.status)}>
                      {labelOf(item.status)}
                    </Status>
                  }
                  title={item.title}
                  urgent={isUrgentDue(item.dueAt, item.status)}
                />
              ))}
            </ListSurface>
          )}
        </section>
      ) : null}

      <section className="app-section">
        <div className="app-section-heading">
          <h2>마감 임박</h2>
        </div>
        {dueSoon.length === 0 ? (
          <EmptyState
            description="7일 안에 마감되는 수합이 없습니다."
            title="임박한 마감이 없습니다"
          />
        ) : (
          <ListSurface>
            {dueSoon.map((item) => {
              const id = "collectId" in item ? item.collectId : item.id;
              const assignment = "collectId" in item;
              return (
                <ListRow
                  action={
                    <Button
                      onClick={() =>
                        navigate(
                          assignment
                            ? { page: "assignment", id }
                            : { page: "collect", id },
                        )
                      }
                      size="small"
                      variant="secondary"
                    >
                      {assignment ? "작성" : "열기"}
                    </Button>
                  }
                  description={formatDue(item.dueAt)}
                  key={`${assignment ? "a" : "c"}-${id}`}
                  status={
                    <Status tone={toneOf(item.status)}>
                      {labelOf(item.status)}
                    </Status>
                  }
                  title={item.title}
                  urgent={isUrgentDue(item.dueAt, item.status)}
                />
              );
            })}
          </ListSurface>
        )}
      </section>
    </div>
  );
}
