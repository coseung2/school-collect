import { useCallback, useEffect, useState } from "react";
import {
  Button,
  EmptyState,
  ListRow,
  ListSurface,
  LoadingState,
  PermissionState,
  Status,
} from "@school-collect/ui";
import { listMembers, type Member } from "../api";
import {
  canManage,
  RequestFailure,
  roleOf,
  toneOf,
  useOnline,
} from "../helpers";
import { navigate } from "../routing";

export function MembersPage({
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
  const [members, setMembers] = useState<Member[] | null>(null);
  const [error, setError] = useState<unknown>(null);

  const load = useCallback(async () => {
    try {
      const value = await listMembers(token, tenantId);
      setMembers(value.members);
      setError(null);
    } catch (caught) {
      setError(caught);
    }
  }, [tenantId, token]);

  useEffect(() => {
    if (manage) {
      void load();
    }
  }, [load, manage]);

  if (!manage) {
    return (
      <div className="app-page">
        <PermissionState
          action={
            <Button
              onClick={() => navigate({ page: "overview" })}
              variant="secondary"
            >
              홈으로
            </Button>
          }
          description="구성원 목록은 관리자와 담당자 화면입니다. 최종 권한은 서버가 판단합니다."
          title="이 화면을 볼 수 없습니다"
        />
      </div>
    );
  }

  if (error && members === null) {
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
          title="구성원을 불러오지 못했습니다"
        />
      </div>
    );
  }

  return (
    <div className="app-page">
      <section className="app-section">
        <div className="app-section-heading">
          <h2>구성원과 역할</h2>
          <Button onClick={() => void load()} size="small" variant="quiet">
            새로고침
          </Button>
        </div>
        {members === null ? (
          <LoadingState
            description="구성원 목록을 불러오고 있습니다."
            title="불러오는 중"
          />
        ) : members.length === 0 ? (
          <EmptyState
            description="이 학교에 표시할 구성원이 없습니다."
            title="구성원이 없습니다"
          />
        ) : (
          <ListSurface>
            {members.map((member) => (
              <ListRow
                description={member.userId}
                key={member.userId}
                status={<Status tone={toneOf(member.role)}>{roleOf(member.role)}</Status>}
                title={member.displayName ?? member.userId}
              />
            ))}
          </ListSurface>
        )}
      </section>
    </div>
  );
}
