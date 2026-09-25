import { useCallback, useEffect, useState } from "react";
import {
  Button,
  EmptyState,
  ListRow,
  ListSurface,
  LoadingState,
  Status,
} from "@school-collect/ui";
import { listAssignments, type Assignment } from "../api";
import {
  formatDue,
  isUrgentDue,
  labelOf,
  RequestFailure,
  toneOf,
  useOnline,
} from "../helpers";
import { navigate } from "../routing";

export function AssignmentsPage({
  tenantId,
  token,
}: {
  tenantId: string;
  token: string;
}) {
  const online = useOnline();
  const [assignments, setAssignments] = useState<Assignment[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const load = useCallback(async () => {
    try {
      const value = await listAssignments(token, tenantId);
      setAssignments(value.assignments);
      setError(null);
    } catch (caught) {
      setError(caught);
    }
  }, [tenantId, token]);

  useEffect(() => {
    void load();
  }, [load]);

  if (error && assignments === null) {
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
          title="내 제출을 불러오지 못했습니다"
        />
      </div>
    );
  }

  return (
    <div className="app-page">
      <section className="app-section">
        <div className="app-section-heading">
          <h2>배정된 수합</h2>
          <Button onClick={() => void load()} size="small" variant="quiet">
            새로고침
          </Button>
        </div>
        {assignments === null ? (
          <LoadingState
            description="배정된 수합을 불러오고 있습니다."
            title="불러오는 중"
          />
        ) : assignments.length === 0 ? (
          <EmptyState
            description="담당자가 수합을 배포하고 대상을 지정하면 여기에 표시됩니다."
            title="배정된 수합이 없습니다"
          />
        ) : (
          <ListSurface>
            {assignments.map((item) => (
              <ListRow
                action={
                  <Button
                    onClick={() =>
                      navigate({ page: "assignment", id: item.collectId })
                    }
                    size="small"
                    variant="secondary"
                  >
                    {item.submissionStatus === "submitted" || item.status === "closed"
                      ? "보기"
                      : "작성"}
                  </Button>
                }
                description={formatDue(item.dueAt)}
                key={item.collectId}
                meta={
                  <Status tone={toneOf(item.assignmentStatus)}>
                    배정 {labelOf(item.assignmentStatus)}
                  </Status>
                }
                status={
                  <Status tone={toneOf(item.submissionStatus ?? item.status)}>
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
    </div>
  );
}
